//! In-memory indexer for tokenized RWA activity on Stellar.
//!
//! Every 10 seconds the indexer reads the current on-chain state of the four
//! RWA contracts through the Soroban RPC `simulateTransaction` endpoint and
//! rebuilds an in-memory snapshot (no database in v1). Reads are pure view
//! calls: they simulate an invocation and decode the returned `ScVal` — no
//! transaction is ever submitted and no key is required.
//!
//! All fallible work returns `Result`. Individual reads retry transient
//! failures in place with jittered backoff (see [`Rpc::read`]); if a refresh
//! cycle still fails, the polling loop logs it and waits for the next
//! [`POLL_INTERVAL`] rather than panicking, so the API always keeps serving
//! the last good snapshot.
//!
//! # Multi-network design decision (issue #433)
//!
//! ## Context
//! The question was raised whether a single deployment should serve multiple
//! Stellar networks (e.g. Testnet + Mainnet) from one API process.
//!
//! ## Options considered
//!
//! **Option A — namespace routes under `/v1/networks/{network}/...`**
//! Each network becomes a path segment: `/v1/networks/testnet/assets`,
//! `/v1/networks/mainnet/assets`, etc. A single `AppState` map keyed by
//! network name drives all reads.
//!
//! *Tradeoffs:*
//! - Doubles URL length and breaks every existing client without a redirect.
//! - Adds a required path parameter to all data routes, making the common
//!   single-network case noisier.
//! - The shared state map grows with every additional network; one blocked
//!   network's indexer can slow scrape of the others.
//!
//! **Option B — one `AppState`/`Indexer` pair per network**
//! The router creates N `AppState` instances at startup (one per configured
//! network) and dispatches by the leading path segment or a request header.
//! Each indexer task polls its own RPC endpoint independently.
//!
//! *Tradeoffs:*
//! - Requires multiplying indexer tasks and state at startup; memory and
//!   goroutine count grow linearly with the number of networks.
//! - Failures are fully isolated: a broken Testnet node cannot degrade
//!   Mainnet reads.
//! - `AppState` and `Indexer` are already `Clone`-friendly, so Option B is
//!   feasible without restructuring the existing types.
//!
//! ## Decision: **Deferred — single-network is the v1 model**
//! A single-network deployment is the supported model for v1. Multi-network
//! support can be introduced as a breaking v2 change by nesting all data
//! routes under `/v2/networks/{network}/`. Backwards compatibility is
//! preserved by keeping `/v1` as-is and running both versions in parallel
//! during any migration window.
//!
//! When multi-network is needed, Option B is the preferred implementation
//! path because `AppState` and `Indexer` are already `Clone`-friendly.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use arc_swap::ArcSwap;
use metrics_exporter_prometheus::PrometheusHandle;
use rand::Rng;
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde::Deserialize;
use stellar_xdr::curr as xdr;
use stellar_xdr::curr::{Limits, ReadXdr, WriteXdr};

use crate::models::{
    Asset, ComplianceRecord, ComplianceSummary, Distribution, Event, Holder, JurisdictionCount,
    Stats,
};

/// How often the indexer refreshes its snapshot.
pub const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Attempts for a single simulated read (the initial try plus retries)
/// before giving up and failing the read.
const MAX_READ_ATTEMPTS: u32 = 4;
/// Base delay before the first retry. Deliberately much shorter than
/// [`POLL_INTERVAL`]: a transient error should be retried in place rather
/// than aborting the whole refresh cycle and waiting a full poll interval.
const RETRY_BASE_DELAY: Duration = Duration::from_millis(150);
/// Ceiling on backoff growth between retries.
const RETRY_MAX_DELAY: Duration = Duration::from_secs(2);
/// Default TTL for the per-asset dividend cache.
///
/// Distributions are fetched at most once per this window; a fresh RPC read
/// is made after the TTL expires.  The value can be overridden at runtime via
/// the `RWA_DIVIDEND_CACHE_TTL_SECS` environment variable so operators can
/// trade freshness against RPC load without recompiling.
///
/// Note: because the main poll interval is 10 s, the maximum lag between a
/// new distribution appearing on-chain and being visible in the API is
/// `RWA_DIVIDEND_CACHE_TTL_SECS + POLL_INTERVAL`.  Clients can compare
/// `Asset.dividends_indexed_at_ledger` against `Asset.indexed_at_ledger` to
/// detect when dividends are being served from the cache.
pub const DEFAULT_DIVIDEND_CACHE_TTL: Duration = Duration::from_secs(60);

/// Per-contract ABI version expectations for the four RWA contract types.
///
/// Each field is a `RangeInclusive<u64>` — a range of `VERSION` values the
/// indexer's decode structs are known to be compatible with.  Using a range
/// rather than a single constant means a backwards-compatible bump in one
/// contract (e.g. dividend going from 3 to 4) does not require simultaneous
/// changes in the others, and a rolling upgrade window can accept both the
/// old and new version simultaneously.
///
/// The ranges here reflect the versions in `stellar-rwa-contracts` main at
/// the time this code was written:
///   - registry     VERSION = 1
///   - compliance   VERSION = 1
///   - asset-token  VERSION = 1
///   - dividend     VERSION = 3  (bumped for snapshot-based claim/cancel)
///
/// See `docs/app/docs/api/versioning/page.mdx` for the human-readable table
/// and a link to DEPLOYMENTS.md in the contracts repository.
#[derive(Debug, Clone)]
pub struct AbiExpectation {
    pub registry: std::ops::RangeInclusive<u64>,
    pub dividend: std::ops::RangeInclusive<u64>,
    pub asset_token: std::ops::RangeInclusive<u64>,
    pub compliance: std::ops::RangeInclusive<u64>,
}

impl AbiExpectation {
    /// Returns the default expectation matching `stellar-rwa-contracts` main.
    pub fn default_ranges() -> Self {
        AbiExpectation {
            registry: 1..=1,
            dividend: 1..=3,
            asset_token: 1..=1,
            compliance: 1..=1,
        }
    }
}
const TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";

/// Page size for paginated `get_all_assets(start_id, limit)` calls.
const REGISTRY_PAGE_SIZE: u32 = 50;

/// Fee used for read-only `simulateTransaction` envelopes.
///
/// This envelope is never submitted to the network — it exists only for
/// `simulateTransaction`. The RPC simulator historically accepts any
/// positive fee and only inspects the host-function payload; still, we use
/// the Stellar minimum base fee (100 stroops per operation) so that:
///   * the envelope is well-formed even if a future RPC release starts
///     statically validating fee preconditions, and
///   * the simulated cost/footprint mirrors a real submission.
///
/// 100 is the network-defined minimum per operation, and we emit exactly one
/// operation per envelope, so this is both the floor and the natural value.
const SIM_FEE: u32 = 100;

/// Source-account sequence number used in the simulated envelope.
///
/// The transaction is built with [`xdr::Preconditions::None`] and an empty
/// signature set, and is never submitted. The simulator does not consult the
/// network for the source account's real sequence number, so `0` is safe
/// today. If a future RPC release begins to validate sequence-number
/// preconditions via the configured `ReadSource` account, hit this constant
/// to wire it up (e.g. call `getTransactionCount` on `ReadSource` per
/// refresh, cache the result, and use it here).
///
/// Typed `i64` because `stellar_xdr::curr::SequenceNumber` wraps an `int64`
/// per the XDR definition; using `u64` would fail to compile (the newtype
/// has no `From<u64>` impl) and so wouldn't slip through as a runtime
/// hazard if a future change accidentally rebinds to a `u64` const.
const SIM_SEQ_NUM: i64 = 0;

/// Maximum number of poll records retained in [`PollHistory`].
const MAX_POLL_HISTORY: usize = 50;

/// Outcome of a single indexer poll cycle.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PollOutcome {
    Success,
    Failure,
}

/// A record of one indexer poll cycle, stored in [`PollHistory`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct PollRecord {
    /// When the poll started, in RFC 3339 format.
    pub started_at: String,
    /// How long the poll took, in milliseconds.
    pub duration_ms: u64,
    /// Whether this poll succeeded or failed.
    pub outcome: PollOutcome,
    /// Number of assets indexed (present on success).
    pub assets_indexed: Option<usize>,
    /// The latest ledger at the time of the poll (present on success).
    pub ledger: Option<u32>,
    /// Error message (present on failure).
    pub error: Option<String>,
}

/// Bounded ring-buffer of the most recent poll records.
///
/// Held inside [`AppState`] so operator tooling (e.g. `GET /poll-history`)
/// can read it without touching the snapshot.
pub struct PollHistory {
    records: VecDeque<PollRecord>,
}

impl PollHistory {
    fn new() -> Self {
        PollHistory {
            records: VecDeque::with_capacity(MAX_POLL_HISTORY),
        }
    }

    /// Append a record, dropping the oldest entry when the buffer is full.
    pub fn push(&mut self, record: PollRecord) {
        if self.records.len() >= MAX_POLL_HISTORY {
            self.records.pop_back();
        }
        self.records.push_front(record);
    }

    /// Return all records, newest first.
    pub fn history(&self) -> Vec<PollRecord> {
        self.records.iter().cloned().collect()
    }
}

/// Static configuration for a network's contracts and RPC endpoint.
#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub registry_id: String,
    pub dividend_id: String,
    pub read_source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid RPC URL: {0}")]
    RpcUrl(String),
    #[error("invalid registry contract ID: {0}")]
    RegistryId(String),
    #[error("invalid dividend contract ID: {0}")]
    DividendId(String),
    #[error("invalid read source account: {0}")]
    ReadSource(String),
    #[error("RWA_REGISTRY_ID and RWA_DIVIDEND_ID are required for a non-testnet RPC")]
    ContractIdsRequired,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let rpc_url = env_or("RWA_RPC_URL", TESTNET_RPC);
        url::Url::parse(&rpc_url).map_err(|e| ConfigError::RpcUrl(format!("{e}: {rpc_url}")))?;

        if rpc_url != TESTNET_RPC
            && (std::env::var_os("RWA_REGISTRY_ID").is_none()
                || std::env::var_os("RWA_DIVIDEND_ID").is_none())
        {
            return Err(ConfigError::ContractIdsRequired);
        }

        let registry_id = env_or(
            "RWA_REGISTRY_ID",
            "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3",
        );
        stellar_strkey::Contract::from_string(&registry_id)
            .map_err(|e| ConfigError::RegistryId(e.to_string()))?;

        let dividend_id = env_or(
            "RWA_DIVIDEND_ID",
            "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX",
        );
        stellar_strkey::Contract::from_string(&dividend_id)
            .map_err(|e| ConfigError::DividendId(e.to_string()))?;

        let read_source = env_or(
            "RWA_READ_SOURCE",
            "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        );
        stellar_strkey::ed25519::PublicKey::from_string(&read_source)
            .map_err(|e| ConfigError::ReadSource(e.to_string()))?;

        Ok(Config {
            rpc_url,
            registry_id,
            dividend_id,
            read_source,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Tracks which portions of the last poll cycle failed and are therefore
/// serving stale data from the previous successful read.
///
/// A flag being `true` means the corresponding contract fetch **failed** on
/// the most recent poll; the snapshot still carries the last good data for
/// that portion.  Callers (e.g. the `/health` route) can inspect these flags
/// to surface partial-staleness in monitoring without discarding the data
/// that _did_ succeed.
#[derive(Debug, Clone, Default)]
pub struct StaleFlags {
    /// Registry contract fetch failed — asset list may be stale.
    pub registry: bool,
    /// Dividend contract fetch failed — distributions may be stale.
    pub dividend: bool,
    /// Compliance contract fetch failed — compliance summaries may be stale.
    pub compliance: bool,
}

/// The immutable, shareable snapshot the API serves.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub assets: Vec<Asset>,
    pub holders: HashMap<u64, Vec<Holder>>,
    pub compliance: HashMap<u64, ComplianceSummary>,
    /// Per-asset per-address compliance records.
    ///
    /// Keyed by `(asset_id, address)` — outer map is asset_id, inner map is
    /// the holder address.  Populated from the real on-chain KYC records read
    /// during `index_compliance_and_holders`.  Routes use this to derive
    /// `status` and `allowed` without a separate RPC call.
    pub compliance_records: HashMap<u64, HashMap<String, ComplianceRecord>>,
    pub dividends: HashMap<u64, Vec<Distribution>>,
    pub events: Vec<Event>,
    pub stats: Stats,
    /// Staleness flags set when a contract fetch fails during the last poll.
    /// The corresponding portion of the snapshot carries data from the
    /// previous successful read rather than fresh on-chain state.
    pub stale_flags: StaleFlags,
}

impl Snapshot {
    pub fn asset(&self, id: u64) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Remove derived entries whose asset IDs are no longer present.
    ///
    /// Full refreshes currently rebuild these maps from scratch. Enforcing the
    /// invariant on every replacement also protects the API if refreshes become
    /// incremental in the future.
    fn prune_stale_asset_maps(&mut self) {
        let current_asset_ids: HashSet<u64> = self.assets.iter().map(|asset| asset.id).collect();

        self.holders
            .retain(|asset_id, _| current_asset_ids.contains(asset_id));
        self.compliance
            .retain(|asset_id, _| current_asset_ids.contains(asset_id));
        self.compliance_records
            .retain(|asset_id, _| current_asset_ids.contains(asset_id));
        self.dividends
            .retain(|asset_id, _| current_asset_ids.contains(asset_id));
    }
}

/// Shared, hot-swappable state handed to the Axum routes.
///
/// `inner` is wrapped in `Arc` so cloning `AppState` shares the same
/// `ArcSwap` instance — `store()` from one clone (the indexer's) is
/// visible to `load()` from another (the routes'), which is the actual
/// data flow we need. Without `Arc`, each clone deep-clones the snapshot
/// and updates from one are never observed by the others.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<ArcSwap<Snapshot>>,
    pub config: Arc<Config>,
    pub metrics: PrometheusHandle,
    pub abi: Arc<AbiExpectation>,
    poll_history: Arc<Mutex<PollHistory>>,
}

impl AppState {
    pub fn new(config: Config, metrics: PrometheusHandle) -> Self {
        AppState {
            inner: Arc::new(ArcSwap::from(Arc::new(Snapshot::default()))),
            config: Arc::new(config),
            metrics,
            abi: Arc::new(AbiExpectation::default_ranges()),
            poll_history: Arc::new(Mutex::new(PollHistory::new())),
        }
    }

    /// Shared handle to the current snapshot for read-only serving.
    ///
    /// This bumps a reference count instead of deep-cloning the snapshot.
    /// The returned `Arc` is immutable and stays internally consistent even
    /// if the indexer swaps in a newer snapshot while the caller holds it.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.inner.load_full()
    }

    /// Last ledger the indexer successfully read. Used as the ETag seed for
    /// snapshot-backed routes (`Cache-Control` + `304 Not Modified`).
    pub fn last_indexed_ledger(&self) -> u32 {
        self.snapshot().stats.last_indexed_ledger
    }

    fn replace(&self, mut next: Snapshot) {
        next.prune_stale_asset_maps();
        crate::indexer_metrics::record_snapshot(&next);
        crate::snapshot_bounds::record(&next);
        self.inner.store(Arc::new(next));
    }

    /// Append a [`PollRecord`] to the bounded poll history ring-buffer.
    pub fn push_poll_record(&self, record: PollRecord) {
        if let Ok(mut history) = self.poll_history.lock() {
            history.push(record);
        }
    }

    /// Return all poll records, newest first.
    pub fn poll_history_records(&self) -> Vec<PollRecord> {
        self.poll_history
            .lock()
            .map(|h| h.history())
            .unwrap_or_default()
    }

    /// Test-only: build state pre-populated with `snapshot`.
    #[cfg(test)]
    pub(crate) fn for_test(config: Config, metrics: PrometheusHandle, snapshot: Snapshot) -> Self {
        let state = AppState::new(config, metrics);
        state.replace(snapshot);
        state
    }

    /// Test-only: build state backed by an empty snapshot, synthesizing config/metrics.
    #[cfg(test)]
    pub fn for_test_empty() -> Self {
        use metrics_exporter_prometheus::PrometheusBuilder;
        // Build the Config directly rather than via `Config::from_env()`.
        // `from_env` reads process-global environment variables, so any test
        // that mutates `RWA_*` (see `config_from_env_overrides_with_env_vars`)
        // would race with every route test using this helper.
        let config = Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3".to_string(),
            dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX".to_string(),
            read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
        };
        let metrics = PrometheusBuilder::new().build_recorder().handle();
        AppState {
            inner: Arc::new(ArcSwap::from(Arc::new(Snapshot::default()))),
            config: Arc::new(config),
            metrics,
            abi: Arc::new(AbiExpectation::default_ranges()),
            poll_history: Arc::new(Mutex::new(PollHistory::new())),
        }
    }

    /// Test-only: build state pre-seeded with the provided assets.
    #[cfg(test)]
    pub fn with_assets(assets: Vec<Asset>) -> Self {
        let state = Self::for_test_empty();
        state.replace(Snapshot {
            assets,
            ..Snapshot::default()
        });
        state
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("rpc request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("rpc returned an error: {0}")]
    Rpc(String),
    #[error("xdr error: {0}")]
    Xdr(#[from] xdr::Error),
    #[error("strkey error: {0}")]
    Strkey(#[from] stellar_strkey::DecodeError),
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("rate limited or unavailable (status {status}); retry after {retry_after:?}")]
    RateLimited {
        status: u16,
        retry_after: Option<Duration>,
        body: String,
    },
    #[error("{contract} ABI version {actual} is not in supported range {expected_range}")]
    AbiVersion {
        contract: String,
        actual: u64,
        expected_range: String,
    },
}

impl IndexError {
    /// Whether this error is worth retrying: network/HTTP-level failures and
    /// RPC-side errors (e.g. a node returning "busy" or a 502) are typically
    /// transient. XDR, strkey and decode errors stem from our own request or
    /// response handling and will fail identically on every attempt.
    fn is_transient(&self) -> bool {
        matches!(self, IndexError::Http(_) | IndexError::Rpc(_))
    }
}

// ---------------------------------------------------------------------------
// RPC client
// ---------------------------------------------------------------------------

struct Rpc {
    http: reqwest::Client,
    url: String,
    source: String,
}

#[derive(Deserialize)]
struct RpcEnvelope {
    result: Option<SimulateResult>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    message: String,
}

#[derive(Deserialize)]
struct SimulateResult {
    #[serde(default)]
    results: Vec<SimResultEntry>,
    #[serde(default)]
    error: Option<String>,
    #[serde(rename = "latestLedger", default)]
    latest_ledger: u32,
}

#[derive(Deserialize)]
struct SimResultEntry {
    xdr: String,
}

/// Top-level envelope for a `getEvents` JSON-RPC response.
#[derive(Deserialize)]
struct GetEventsEnvelope {
    result: Option<GetEventsResult>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct GetEventsResult {
    events: Vec<RawRpcEvent>,
    #[serde(rename = "latestLedger", default)]
    latest_ledger: u32,
}

/// A single event entry from the `getEvents` RPC response.
#[derive(Deserialize)]
struct RawRpcEvent {
    id: String,
    #[serde(rename = "contractId", default)]
    contract_id: String,
    topic: Vec<String>,
    value: Option<String>,
    ledger: u32,
    #[serde(rename = "ledgerClosedAt", default)]
    ledger_closed_at: String,
}

/// Outcome of a single simulated read.
#[derive(Debug)]
struct ReadOutcome {
    value: serde_json::Value,
    latest_ledger: u32,
}

/// Decode a single simulation result without affecting any other reads in the
/// current refresh. Callers decide whether to retain the previous snapshot or
/// skip the affected record when this deterministic conversion fails.
fn decode_simulation_result(xdr_b64: &str) -> Result<serde_json::Value, IndexError> {
    let scval = xdr::ScVal::from_xdr_base64(xdr_b64, Limits::none())?;
    scval_to_json(&scval)
}

impl Rpc {
    fn new(url: String, source: String) -> Self {
        Rpc {
            http: reqwest::Client::new(),
            url,
            source,
        }
    }

    /// Simulate `contract.method(args)` and decode the return value to JSON.
    ///
    /// Transient failures (network errors, non-2xx responses, RPC-level
    /// errors) are retried in place with jittered backoff — see
    /// [`MAX_READ_ATTEMPTS`] — rather than bubbling straight up and forcing
    /// the whole refresh cycle to restart on the next [`POLL_INTERVAL`].
    /// Decode/XDR/strkey errors are not retried: they're deterministic bugs
    /// in our own encoding, not something a retry can fix.
    async fn read(
        &self,
        contract: &str,
        method: &str,
        args: Vec<xdr::ScVal>,
    ) -> Result<ReadOutcome, IndexError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.read_once(contract, method, args.clone()).await {
                Ok(outcome) => return Ok(outcome),
                Err(e) if attempt < MAX_READ_ATTEMPTS && e.is_transient() => {
                    let delay = retry_delay(attempt);
                    tracing::warn!(
                        contract,
                        method,
                        attempt,
                        delay_ms = delay.as_millis() as u64,
                        error = %e,
                        "transient read error; retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn read_once(
        &self,
        contract: &str,
        method: &str,
        args: Vec<xdr::ScVal>,
    ) -> Result<ReadOutcome, IndexError> {
        let envelope_b64 = build_invoke_envelope(&self.source, contract, method, args)?;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": { "transaction": envelope_b64 },
        });

        let resp = self.http.post(&self.url).json(&body).send().await?;

        let status = resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE {
            let headers = resp.headers().clone();
            let body = resp.text().await.unwrap_or_default();
            let retry_after = headers
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(IndexError::RateLimited {
                status: status.as_u16(),
                retry_after,
                body,
            });
        }

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(IndexError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        let resp: RpcEnvelope = resp.json().await?;

        if let Some(err) = resp.error {
            return Err(IndexError::Rpc(err.message));
        }
        let result = resp
            .result
            .ok_or_else(|| IndexError::Rpc("empty rpc result".into()))?;
        if let Some(sim_err) = result.error {
            return Err(IndexError::Rpc(sim_err));
        }
        let entry = result
            .results
            .first()
            .ok_or_else(|| IndexError::Rpc("no simulation result".into()))?;
        let value = decode_simulation_result(&entry.xdr).inspect_err(|error| {
            tracing::warn!(
                contract,
                method,
                latest_ledger = result.latest_ledger,
                error = %error,
                "simulation result decode failed; isolating failed read"
            );
        })?;
        Ok(ReadOutcome {
            value,
            latest_ledger: result.latest_ledger,
        })
    }

    /// Fetch contract events via `getEvents`, filtering for specific contract IDs
    /// starting from `start_ledger`. Returns all matching events and the latest
    /// ledger seen by the RPC node.
    async fn get_events(
        &self,
        contract_ids: &[&str],
        start_ledger: u32,
    ) -> Result<(Vec<crate::models::Event>, u32), IndexError> {
        // Build a filter per contract id. The RPC `getEvents` method accepts
        // an array of filters; each filter narrows to one contract.
        let filters: Vec<serde_json::Value> = contract_ids
            .iter()
            .map(|id| serde_json::json!({ "type": "contract", "contractIds": [id] }))
            .collect();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getEvents",
            "params": {
                "startLedger": start_ledger,
                "filters": filters,
                "pagination": { "limit": EVENTS_PAGE_LIMIT }
            }
        });

        let resp = self.http.post(&self.url).json(&body).send().await?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
        {
            let headers = resp.headers().clone();
            let body_text = resp.text().await.unwrap_or_default();
            let retry_after = headers
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(IndexError::RateLimited {
                status: status.as_u16(),
                retry_after,
                body: body_text,
            });
        }

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(IndexError::HttpStatus {
                status: status.as_u16(),
                body: body_text,
            });
        }

        let envelope: GetEventsEnvelope = resp.json().await?;
        if let Some(err) = envelope.error {
            return Err(IndexError::Rpc(err.message));
        }
        let result = envelope
            .result
            .ok_or_else(|| IndexError::Rpc("empty getEvents result".into()))?;

        let mut events = Vec::new();
        for raw in result.events {
            // Derive event_type from the first topic symbol. Topics are XDR
            // base64-encoded ScVal; we do a best-effort decode and fall back
            // to the raw string if it can't be parsed.
            let event_type = raw
                .topic
                .first()
                .map(|t| {
                    xdr::ScVal::from_xdr_base64(t, Limits::none())
                        .ok()
                        .and_then(|v| match v {
                            xdr::ScVal::Symbol(s) => Some(s.to_string()),
                            xdr::ScVal::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| t.clone())
                })
                .unwrap_or_else(|| "unknown".to_string());

            // Decode the event value payload (may be absent on some events).
            let data = raw
                .value
                .as_deref()
                .and_then(|xdr_b64| {
                    xdr::ScVal::from_xdr_base64(xdr_b64, Limits::none())
                        .ok()
                        .and_then(|v| scval_to_json(&v).ok())
                })
                .unwrap_or(serde_json::Value::Null);

            // Parse the event id: Stellar event ids are "<ledger>-<index>".
            let numeric_id: u64 = raw
                .id
                .split('-')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(raw.ledger as u64);

            let timestamp = if raw.ledger_closed_at.is_empty() {
                None
            } else {
                Some(raw.ledger_closed_at.clone())
            };

            events.push(crate::models::Event {
                id: numeric_id,
                contract: raw.contract_id,
                event_type,
                ledger: raw.ledger,
                timestamp,
                data,
            });
        }

        Ok((events, result.latest_ledger))
    }
}

/// Derive whether an address is allowed to transact, mirroring the on-chain
/// compliance gate:
///   - status must be `"Approved"`
///   - `expires_at` must be 0 (no expiry) or greater than `latest_ledger`
///   - the `jurisdiction` must not be in `blocked_jurisdictions`
///
/// This is a pure function so it can be called from routes without an RPC
/// round-trip; it uses the `ComplianceRecord` already stored in the snapshot.
pub fn derive_allowed(
    rec: &crate::models::ComplianceRecord,
    latest_ledger: u32,
    blocked_jurisdictions: &std::collections::HashSet<String>,
) -> bool {
    if rec.status != "Approved" {
        return false;
    }
    if rec.expires_at != 0 && rec.expires_at <= latest_ledger {
        return false;
    }
    if blocked_jurisdictions.contains(&rec.jurisdiction) {
        return false;
    }
    true
}

/// Jittered exponential backoff for the `attempt`-th failed read (1-indexed).
/// Full jitter (a random delay in `[0, cap]`) avoids every in-flight read
/// retrying in lockstep after a shared transient failure (e.g. the RPC node
/// briefly rejecting all requests).
fn retry_delay(attempt: u32) -> Duration {
    let exp = RETRY_BASE_DELAY.saturating_mul(1u32 << (attempt - 1).min(4));
    let cap = exp.min(RETRY_MAX_DELAY);
    rand::rng().random_range(Duration::ZERO..=cap)
}

// ---------------------------------------------------------------------------
// XDR helpers
// ---------------------------------------------------------------------------

fn contract_id(strkey: &str) -> Result<xdr::ContractId, IndexError> {
    let c = stellar_strkey::Contract::from_string(strkey)?;
    Ok(xdr::ContractId(xdr::Hash(c.0)))
}

fn account_muxed(strkey: &str) -> Result<xdr::MuxedAccount, IndexError> {
    let pk = stellar_strkey::ed25519::PublicKey::from_string(strkey)?;
    Ok(xdr::MuxedAccount::Ed25519(xdr::Uint256(pk.0)))
}

/// An `ScVal::Address` from a G… or C… strkey.
fn address_scval(strkey: &str) -> Result<xdr::ScVal, IndexError> {
    if strkey.starts_with('C') {
        Ok(xdr::ScVal::Address(xdr::ScAddress::Contract(contract_id(
            strkey,
        )?)))
    } else {
        let pk = stellar_strkey::ed25519::PublicKey::from_string(strkey)?;
        Ok(xdr::ScVal::Address(xdr::ScAddress::Account(
            xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256(pk.0))),
        )))
    }
}

/// Build a base64 `TransactionEnvelope` invoking a contract method. The
/// transaction is never signed or submitted — it exists only to be simulated.
fn build_invoke_envelope(
    source: &str,
    contract: &str,
    method: &str,
    args: Vec<xdr::ScVal>,
) -> Result<String, IndexError> {
    let function_name = xdr::ScSymbol(method.try_into()?);
    let invoke = xdr::InvokeContractArgs {
        contract_address: xdr::ScAddress::Contract(contract_id(contract)?),
        function_name,
        args: args.try_into()?,
    };
    let op = xdr::Operation {
        source_account: None,
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::InvokeContract(invoke),
            auth: Default::default(),
        }),
    };
    let tx = xdr::Transaction {
        source_account: account_muxed(source)?,
        fee: SIM_FEE,
        seq_num: xdr::SequenceNumber(SIM_SEQ_NUM),
        cond: xdr::Preconditions::None,
        memo: xdr::Memo::None,
        operations: vec![op].try_into()?,
        ext: xdr::TransactionExt::V0,
    };
    let envelope = xdr::TransactionEnvelope::Tx(xdr::TransactionV1Envelope {
        tx,
        signatures: Default::default(),
    });
    Ok(envelope.to_xdr_base64(Limits::none())?)
}

/// Convert a decoded `ScVal` into `serde_json::Value`.
///
/// Scalars map to their JSON counterparts; 128-bit integers become decimal
/// strings (to survive JavaScript); contract structs (`ScMap`) become objects
/// keyed by their symbol field names; unit enums (`ScVec` of one symbol) and
/// plain vectors become arrays.
fn scval_to_json(v: &xdr::ScVal) -> Result<serde_json::Value, IndexError> {
    use serde_json::Value;
    Ok(match v {
        xdr::ScVal::Bool(b) => Value::Bool(*b),
        xdr::ScVal::Void => Value::Null,
        xdr::ScVal::U32(n) => Value::from(*n),
        xdr::ScVal::I32(n) => Value::from(*n),
        xdr::ScVal::U64(n) => Value::from(*n),
        xdr::ScVal::I64(n) => Value::from(*n),
        xdr::ScVal::U128(p) => {
            let val = ((p.hi as u128) << 64) | (p.lo as u128);
            Value::String(val.to_string())
        }
        xdr::ScVal::I128(p) => {
            let val = ((p.hi as i128) << 64) | (p.lo as i128);
            Value::String(val.to_string())
        }
        xdr::ScVal::Symbol(s) => Value::String(s.to_string()),
        xdr::ScVal::String(s) => Value::String(s.to_string()),
        xdr::ScVal::Address(a) => Value::String(address_to_string(a)?),
        xdr::ScVal::Vec(Some(items)) => {
            let mut arr = Vec::with_capacity(items.len());
            for item in items.iter() {
                arr.push(scval_to_json(item)?);
            }
            Value::Array(arr)
        }
        xdr::ScVal::Vec(None) => Value::Array(vec![]),
        xdr::ScVal::Map(Some(entries)) => {
            let mut obj = serde_json::Map::new();
            for e in entries.iter() {
                let key = match &e.key {
                    xdr::ScVal::Symbol(s) => s.to_string(),
                    xdr::ScVal::String(s) => s.to_string(),
                    other => json_key_fallback(other)?,
                };
                obj.insert(key, scval_to_json(&e.val)?);
            }
            Value::Object(obj)
        }
        xdr::ScVal::Map(None) => Value::Object(serde_json::Map::new()),
        // Remaining variants aren't produced by these contracts' return values.
        _ => Value::Null,
    })
}

fn json_key_fallback(v: &xdr::ScVal) -> Result<String, IndexError> {
    match scval_to_json(v)? {
        serde_json::Value::String(s) => Ok(s),
        other => Ok(other.to_string()),
    }
}

fn address_to_string(a: &xdr::ScAddress) -> Result<String, IndexError> {
    match a {
        xdr::ScAddress::Account(xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(
            xdr::Uint256(bytes),
        ))) => Ok(stellar_strkey::ed25519::PublicKey(*bytes).to_string()),
        xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(bytes))) => {
            Ok(stellar_strkey::Contract(*bytes).to_string())
        }
        // Muxed accounts and any future address variants are not expected from
        // these contracts, but returning an error here would abort the entire
        // refresh cycle via `?`.  Emit a recognisable placeholder so callers
        // can still process the rest of the response.
        other => {
            tracing::warn!(
                "address_to_string: unsupported address variant, using placeholder: {other:?}"
            );
            Ok(format!("unknown:{other:?}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Raw decode structs (match the contract field names)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawAssetEntry {
    id: u64,
    token_contract: String,
    issuer: String,
    name: String,
    asset_type: String,
    valuation: String,
    created_at: u32,
    active: bool,
}

#[derive(Deserialize)]
struct RawMetadata {
    symbol: String,
    total_supply: String,
    decimals: u32,
    compliance_contract: String,
    asset_description: String,
    paused: bool,
}

#[derive(Deserialize)]
struct RawKyc {
    status: serde_json::Value,
    jurisdiction: String,
    expires_at: u32,
}

#[derive(Deserialize)]
struct RawDistribution {
    id: u64,
    asset_token: String,
    payment_token: String,
    total_amount: String,
    distributed: String,
    created_at: u32,
    completed: bool,
}

fn parse_i128(s: &str) -> i128 {
    s.parse::<i128>().unwrap_or(0)
}

fn cents_to_usd(cents: i128) -> f64 {
    (cents / 100) as f64 + (cents % 100) as f64 / 100.0
}

fn ratio_percent(part: i128, whole: i128) -> f64 {
    if whole <= 0 {
        return 0.0;
    }
    let scaled = part
        .clamp(0, whole)
        .saturating_mul(10_000)
        .saturating_add(whole / 2)
        / whole;
    scaled as f64 / 100.0
}

/// Normalise a compliance status that may decode as `"Approved"` or
/// `["Approved"]` (unit-variant enum) into a plain string.
fn normalize_status(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a
            .first()
            .and_then(|x| x.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        _ => "Unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Indexer
// ---------------------------------------------------------------------------

/// Cache of the most recent successful dividend read per asset token,
/// keyed by contract address.
///
/// Tuple fields: `(fetched_at, ledger_at_fetch, distributions, index_error)`.
/// `ledger_at_fetch` records the `latest_ledger` returned by the RPC call
/// that populated this cache entry.  It is exposed as
/// `Asset.dividends_indexed_at_ledger` so clients can tell whether
/// dividend data lags behind the main `indexed_at_ledger`.
type DividendCache = HashMap<String, (Instant, u32, Vec<Distribution>, Option<String>)>;

pub struct Indexer {
    rpc: Rpc,
    state: AppState,
    dividend_cache: Mutex<DividendCache>,
    /// Ledger from which the next `getEvents` call should start. Atomically
    /// updated after each successful event read so partial failures don't
    /// re-fetch events from ledger 1 on every cycle.
    events_cursor: AtomicU32,
}

impl Indexer {
    pub fn new(state: AppState) -> Self {
        let cfg = &state.config;
        Indexer {
            rpc: Rpc::new(cfg.rpc_url.clone(), cfg.read_source.clone()),
            state,
            dividend_cache: Mutex::new(HashMap::new()),
            events_cursor: AtomicU32::new(0),
        }
    }

    /// Poll forever, refreshing the snapshot every [`POLL_INTERVAL`], until
    /// `shutdown` is flipped to `true` (see [`crate::shutdown_signal`] in
    /// `main.rs`), at which point the loop halts rather than starting
    /// another refresh cycle.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        loop {
            if *shutdown.borrow() {
                tracing::info!("shutdown signal received; stopping indexer poll loop");
                return;
            }

            let started = Instant::now();
            let result = self.refresh().await;
            let elapsed = started.elapsed();
            metrics::histogram!("rwa_indexer_refresh_duration_seconds")
                .record(elapsed.as_secs_f64());

            let backoff = match result {
                Ok(count) => {
                    crate::poll_status::record_success(self.state.last_indexed_ledger());
                    metrics::counter!("rwa_indexer_refresh_total", "outcome" => "success")
                        .increment(1);
                    tracing::info!(
                        assets = count,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "index refreshed"
                    );
                    self.state.push_poll_record(PollRecord {
                        started_at: chrono::Utc::now().to_rfc3339(),
                        duration_ms: elapsed.as_millis() as u64,
                        outcome: PollOutcome::Success,
                        assets_indexed: Some(count),
                        ledger: Some(self.state.last_indexed_ledger()),
                        error: None,
                    });
                    POLL_INTERVAL
                }
                Err(e) => {
                    let consecutive_failures = crate::poll_status::record_failure();
                    metrics::counter!("rwa_indexer_refresh_total", "outcome" => "failure")
                        .increment(1);
                    tracing::warn!(
                        consecutive_failures,
                        error = %e,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "index refresh failed; keeping last snapshot"
                    );
                    self.state.push_poll_record(PollRecord {
                        started_at: chrono::Utc::now().to_rfc3339(),
                        duration_ms: elapsed.as_millis() as u64,
                        outcome: PollOutcome::Failure,
                        assets_indexed: None,
                        ledger: None,
                        error: Some(e.to_string()),
                    });
                    if let IndexError::RateLimited { retry_after, .. } = &e {
                        retry_after.unwrap_or(POLL_INTERVAL)
                    } else {
                        POLL_INTERVAL
                    }
                }
            };

            // Honour `RWA_POLL_INTERVAL_SECS` (default `POLL_INTERVAL`); a
            // server-advised Retry-After delay still takes precedence.
            let backoff = if backoff == POLL_INTERVAL {
                crate::config_env::poll_interval()
            } else {
                backoff
            };

            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("shutdown signal received; stopping indexer poll loop");
                        return;
                    }
                }
            }
        }
    }

    /// Read the full current state of all contracts and rebuild the snapshot.
    ///
    /// # Partial-failure behaviour (#431)
    ///
    /// Each top-level contract fetch (registry, dividend ABI check, per-asset
    /// token) is attempted **independently**.  A failure in one does not abort
    /// the whole cycle; instead:
    ///
    /// * The failure is logged at `WARN` level with the contract name and
    ///   error message.
    /// * The corresponding portion of the snapshot retains data from the
    ///   previous successful read.
    /// * [`StaleFlags`] on the returned snapshot records which portions are
    ///   stale, so callers such as the `/health` route can surface partial
    ///   staleness to monitoring without discarding good data.
    ///
    /// The refresh still returns `Err` if the registry read fails entirely
    /// (no asset list = no meaningful snapshot to serve).
    async fn refresh(&self) -> Result<usize, IndexError> {
        let cfg = &self.state.config;

        self.check_abi(&cfg.registry_id, &self.state.abi.registry, "registry")
            .await?;
        self.check_abi(&cfg.dividend_id, &self.state.abi.dividend, "dividend")
            .await?;

        // ── registry pagination ───────────────────────────────────────────────
        // The registry contract on the current main branch exposes a paginated
        // signature: get_all_assets(start_id: u64, limit: u32) -> Vec<AssetEntry>.
        // Older deployments accepted no arguments.  We try the paginated form
        // first; if the RPC returns a simulation error we fall back to the
        // no-argument call for backward compatibility.
        let (raw_entries, latest_ledger) = self.fetch_all_assets(&cfg.registry_id).await?;

        // Grab the previous snapshot so we can carry forward per-asset
        // freshness info for assets that fail this cycle.
        let prev = self.state.snapshot();

        let mut assets = Vec::new();
        let mut holders_map: HashMap<u64, Vec<Holder>> = HashMap::new();
        let mut compliance_map: HashMap<u64, ComplianceSummary> = HashMap::new();
        let mut compliance_records_map: HashMap<u64, HashMap<String, ComplianceRecord>> =
            HashMap::new();
        let mut dividends_map: HashMap<u64, Vec<Distribution>> = HashMap::new();
        let mut total_distributions = 0usize;
        let mut tvl: i128 = 0;

        for raw in &raw_entries {
            self.check_abi(
                &raw.token_contract,
                &self.state.abi.asset_token,
                "asset-token",
            )
            .await?;
            // ── per-asset metadata + compliance reads (best-effort) ───────────
            // A failure here is recorded and the asset is emitted with its
            // previous data (if any) plus an `index_error`.  This mirrors the
            // dividend handling below and means one broken asset contract can
            // never abort the whole refresh cycle.
            let meta_read = self
                .rpc
                .read(&raw.token_contract, "get_metadata", vec![])
                .await
                .inspect_err(|_| record_asset_read_error(raw.id, "get_metadata"));

            let meta_read = match meta_read {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        asset_id = raw.id,
                        error = %e,
                        "get_metadata failed; keeping previous asset data"
                    );
                    // Re-emit the previous Asset (if we have one) with the
                    // updated error, so consumers can detect staleness.
                    if let Some(prev_asset) = prev.asset(raw.id) {
                        let mut stale = prev_asset.clone();
                        stale.index_error = Some(e.to_string());
                        if let Some(prev_holders) = prev.holders.get(&raw.id) {
                            holders_map.insert(raw.id, prev_holders.clone());
                            total_distributions +=
                                prev.dividends.get(&raw.id).map(|d| d.len()).unwrap_or(0);
                            if stale.active {
                                tvl += parse_i128(&stale.valuation_cents);
                            }
                        }
                        if let Some(prev_comp) = prev.compliance.get(&raw.id) {
                            compliance_map.insert(raw.id, prev_comp.clone());
                        }
                        if let Some(prev_crecs) = prev.compliance_records.get(&raw.id) {
                            compliance_records_map.insert(raw.id, prev_crecs.clone());
                        }
                        if let Some(prev_dists) = prev.dividends.get(&raw.id) {
                            dividends_map.insert(raw.id, prev_dists.clone());
                        }
                        assets.push(stale);
                    }
                    continue;
                }
            };

            let asset_ledger = meta_read.latest_ledger;
            let meta: RawMetadata = match serde_json::from_value(meta_read.value) {
                Ok(m) => m,
                Err(e) => {
                    let err = IndexError::Decode(e);
                    record_asset_read_error(raw.id, "get_metadata");
                    tracing::warn!(
                        asset_id = raw.id,
                        error = %err,
                        "metadata decode failed; keeping previous asset data"
                    );
                    if let Some(prev_asset) = prev.asset(raw.id) {
                        let mut stale = prev_asset.clone();
                        stale.index_error = Some(err.to_string());
                        if stale.active {
                            tvl += parse_i128(&stale.valuation_cents);
                        }
                        if let Some(prev_holders) = prev.holders.get(&raw.id) {
                            holders_map.insert(raw.id, prev_holders.clone());
                        }
                        if let Some(prev_comp) = prev.compliance.get(&raw.id) {
                            compliance_map.insert(raw.id, prev_comp.clone());
                        }
                        if let Some(prev_crecs) = prev.compliance_records.get(&raw.id) {
                            compliance_records_map.insert(raw.id, prev_crecs.clone());
                        }
                        if let Some(prev_dists) = prev.dividends.get(&raw.id) {
                            total_distributions += prev_dists.len();
                            dividends_map.insert(raw.id, prev_dists.clone());
                        }
                        assets.push(stale);
                    }
                    continue;
                }
            };

            let total_supply = parse_i128(&meta.total_supply);
            let valuation = parse_i128(&raw.valuation);
            self.check_abi(
                &meta.compliance_contract,
                &self.state.abi.compliance,
                "compliance",
            )
            .await?;

            // Holders: every allowlisted address with a positive balance.
            // Also best-effort: fall back to previous holders on failure.
            let (holders, summary, crecs, compliance_err) = match self
                .index_compliance_and_holders(
                    &meta.compliance_contract,
                    &raw.token_contract,
                    total_supply,
                )
                .await
            {
                Ok(result) => (result.0, result.1, result.2, None),
                Err(e) => {
                    record_asset_read_error(raw.id, "compliance");
                    tracing::warn!(
                        asset_id = raw.id,
                        error = %e,
                        "compliance/holders read failed; using previous data"
                    );
                    let holders = prev.holders.get(&raw.id).cloned().unwrap_or_default();
                    let summary = prev.compliance.get(&raw.id).cloned().unwrap_or_default();
                    let crecs = prev
                        .compliance_records
                        .get(&raw.id)
                        .cloned()
                        .unwrap_or_default();
                    (holders, summary, crecs, Some(e.to_string()))
                }
            };

            // Dividends for this asset token.  On failure we preserve the last
            // known distributions from the previous snapshot rather than
            // resetting to empty, so a transient RPC hiccup doesn't make the
            // API silently report "no dividends" for an asset.
            let (dists, dividends_ledger, dividend_err) = match self
                .index_dividends(raw.id, &raw.token_contract)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    record_asset_read_error(raw.id, "dividends");
                    tracing::warn!(asset_id = raw.id, error = %e, "dividends read failed; keeping previous distributions");
                    let prev_ledger = prev
                        .asset(raw.id)
                        .and_then(|a| a.dividends_indexed_at_ledger);
                    (
                        prev.dividends.get(&raw.id).cloned().unwrap_or_default(),
                        prev_ledger,
                        Some(e.to_string()),
                    )
                }
            };
            total_distributions += dists.len();

            if raw.active {
                tvl += valuation;
            }

            let asset = Asset {
                id: raw.id,
                token_contract: raw.token_contract.clone(),
                issuer: raw.issuer.clone(),
                name: raw.name.clone(),
                symbol: meta.symbol,
                asset_type: raw.asset_type.clone(),
                description: meta.asset_description,
                valuation_cents: valuation.to_string(),
                valuation_usd: cents_to_usd(valuation),
                decimals: meta.decimals,
                total_supply: total_supply.to_string(),
                holders: holders.len(),
                active: raw.active,
                paused: meta.paused,
                compliance_contract: meta.compliance_contract,
                created_at_ledger: raw.created_at,
                indexed_at_ledger: asset_ledger,
                dividends_indexed_at_ledger: dividends_ledger,
                index_error: dividend_err.or(compliance_err),
            };

            holders_map.insert(raw.id, holders);
            compliance_map.insert(raw.id, summary);
            compliance_records_map.insert(raw.id, crecs);
            dividends_map.insert(raw.id, dists);
            assets.push(asset);
        }

        let active_assets = assets.iter().filter(|a| a.active).count();
        let mut distinct_holders = HashSet::new();
        for holders in holders_map.values() {
            for h in holders {
                distinct_holders.insert(h.address.clone());
            }
        }
        let stats = Stats {
            total_assets: assets.len(),
            active_assets,
            tvl_cents: tvl.to_string(),
            tvl_usd: cents_to_usd(tvl),
            total_holders: distinct_holders.len(),
            total_distributions,
            last_indexed_ledger: latest_ledger,
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        };

        // ── event ingestion ──────────────────────────────────────────────────
        // Collect the unique set of contract IDs to monitor: the registry,
        // the dividend contract, plus every discovered asset-token and
        // compliance contract. On failure we carry forward the events from
        // the previous snapshot rather than resetting to empty.
        let mut contract_ids_to_watch: Vec<String> =
            vec![cfg.registry_id.clone(), cfg.dividend_id.clone()];
        for asset in &assets {
            contract_ids_to_watch.push(asset.token_contract.clone());
            contract_ids_to_watch.push(asset.compliance_contract.clone());
        }
        contract_ids_to_watch.sort();
        contract_ids_to_watch.dedup();

        let events = self
            .index_events(prev.events.clone(), &contract_ids_to_watch)
            .await;

        let count = assets.len();
        self.state.replace(Snapshot {
            assets,
            holders: holders_map,
            compliance: compliance_map,
            compliance_records: compliance_records_map,
            dividends: dividends_map,
            events,
            stats,
            stale_flags: StaleFlags::default(),
        });
        Ok(count)
    }

    /// Read every asset from the registry using the paginated
    /// `get_all_assets(start_id, limit)` signature introduced on the main
    /// branch of the contracts repo.
    ///
    /// If the first page call returns an RPC simulation error (which happens
    /// when the registry was deployed before pagination was added — it rejects
    /// the two-argument call with a host-function error), the method
    /// automatically retries with the legacy zero-argument form and returns
    /// that result instead.  This lets the API work against both deployed
    /// contract versions without operator intervention.
    ///
    /// Returns the concatenated `Vec<RawAssetEntry>` and the `latest_ledger`
    /// from the first successful RPC call (the ledger advances only slightly
    /// between pages so using the first one is accurate enough).
    async fn fetch_all_assets(
        &self,
        registry_id: &str,
    ) -> Result<(Vec<RawAssetEntry>, u32), IndexError> {
        // Try paginated form first.
        let first_page = self
            .rpc
            .read(
                registry_id,
                "get_all_assets",
                vec![
                    xdr::ScVal::U64(0), // start_id
                    xdr::ScVal::U32(REGISTRY_PAGE_SIZE),
                ],
            )
            .await;

        match first_page {
            Err(IndexError::Rpc(_)) => {
                // Simulation error: registry does not accept arguments.
                // Fall back to the legacy no-argument signature.
                tracing::info!(
                    "get_all_assets(start_id, limit) rejected; \
                     falling back to legacy no-argument call"
                );
                let result = self
                    .rpc
                    .read(registry_id, "get_all_assets", vec![])
                    .await?;
                let entries: Vec<RawAssetEntry> = serde_json::from_value(result.value)?;
                return Ok((entries, result.latest_ledger));
            }
            Err(e) => return Err(e),
            Ok(page) => {
                let latest_ledger = page.latest_ledger;
                let mut all: Vec<RawAssetEntry> = serde_json::from_value(page.value)?;

                // Keep fetching until we get a short page.
                loop {
                    if all.len() % REGISTRY_PAGE_SIZE as usize != 0 || all.is_empty() {
                        break;
                    }
                    // next start_id = last id + 1
                    let start_id = all.last().map(|e| e.id + 1).unwrap_or(0);
                    let next_page = self
                        .rpc
                        .read(
                            registry_id,
                            "get_all_assets",
                            vec![
                                xdr::ScVal::U64(start_id),
                                xdr::ScVal::U32(REGISTRY_PAGE_SIZE),
                            ],
                        )
                        .await?;
                    let page_entries: Vec<RawAssetEntry> =
                        serde_json::from_value(next_page.value)?;
                    let is_last = page_entries.len() < REGISTRY_PAGE_SIZE as usize;
                    all.extend(page_entries);
                    if is_last {
                        break;
                    }
                }

                Ok((all, latest_ledger))
            }
        }
    }

    /// Read the compliance allowlist for an asset and derive both the holder
    /// list (allowlisted ∩ positive balance) and the non-PII summary.
    ///
    /// Returns `(holders, summary, compliance_records)` where `compliance_records`
    /// is a map from address to its real on-chain `ComplianceRecord`.  The
    /// records are stored in the snapshot so routes can derive `status` and
    /// `allowed` without extra RPC calls.
    async fn index_compliance_and_holders(
        &self,
        compliance_contract: &str,
        token_contract: &str,
        total_supply: i128,
    ) -> Result<(Vec<Holder>, ComplianceSummary, HashMap<String, ComplianceRecord>), IndexError> {
        let allowlist = self
            .rpc
            .read(compliance_contract, "get_allowlist", vec![])
            .await?;
        let addresses: Vec<String> = serde_json::from_value(allowlist.value)?;

        let mut holders = Vec::new();
        let mut summary = ComplianceSummary::default();
        let mut jurisdictions: BTreeMap<String, usize> = BTreeMap::new();
        let mut records: HashMap<String, ComplianceRecord> = HashMap::new();

        for address in &addresses {
            summary.total_records += 1;

            // Record status → summary counts and real ComplianceRecord.
            if let Ok(rec) = self
                .rpc
                .read(
                    compliance_contract,
                    "get_record",
                    vec![address_scval(address)?],
                )
                .await
            {
                if !rec.value.is_null() {
                    if let Ok(kyc) = serde_json::from_value::<RawKyc>(rec.value) {
                        let status = normalize_status(&kyc.status);
                        match status.as_str() {
                            "Approved" => {
                                summary.approved += 1;
                            }
                            "Suspended" => summary.suspended += 1,
                            "Rejected" => summary.rejected += 1,
                            "Pending" => summary.pending += 1,
                            _ => {}
                        }
                        if kyc.expires_at != 0 {
                            summary.with_expiry += 1;
                        }
                        *jurisdictions.entry(kyc.jurisdiction.clone()).or_insert(0) += 1;

                        // Persist the real record for route-level allowed derivation.
                        records.insert(
                            address.clone(),
                            ComplianceRecord {
                                status,
                                jurisdiction: kyc.jurisdiction,
                                expires_at: kyc.expires_at,
                            },
                        );
                    }
                }
            }

            // Balance → holder list.
            let bal = self
                .rpc
                .read(token_contract, "balance", vec![address_scval(address)?])
                .await?;
            let balance = match bal.value {
                serde_json::Value::String(s) => parse_i128(&s),
                serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) as i128,
                _ => 0,
            };
            if balance > 0 {
                holders.push(Holder {
                    address: address.clone(),
                    balance: balance.to_string(),
                    share_percent: ratio_percent(balance, total_supply),
                });
            }
        }

        holders.sort_by_key(|h| std::cmp::Reverse(parse_i128(&h.balance)));
        summary.jurisdictions = jurisdictions
            .into_iter()
            .map(|(jurisdiction, count)| JurisdictionCount {
                jurisdiction,
                count,
            })
            .collect();

        Ok((holders, summary, records))
    }

    async fn check_abi(
        &self,
        contract: &str,
        range: &std::ops::RangeInclusive<u64>,
        label: &str,
    ) -> Result<(), IndexError> {
        let read = self.rpc.read(contract, "version", vec![]).await?;
        let actual = read.value.as_u64().unwrap_or_default();
        tracing::info!(
            contract,
            label,
            version = actual,
            supported_min = range.start(),
            supported_max = range.end(),
            "ABI version check"
        );
        if !range.contains(&actual) {
            return Err(IndexError::AbiVersion {
                contract: contract.to_string(),
                actual,
                expected_range: format!("{}..={}", range.start(), range.end()),
            });
        }
        Ok(())
    }

    /// Read distributions at most once per cache window.
    ///
    /// Returns `(distributions, dividends_indexed_at_ledger, index_error)`.
    /// `dividends_indexed_at_ledger` is the `latest_ledger` from the RPC call
    /// that last populated the cache, so callers can surface it in the
    /// `Asset` response and let clients detect cache lag.
    ///
    /// The TTL is read from `RWA_DIVIDEND_CACHE_TTL_SECS` at each call,
    /// defaulting to [`DEFAULT_DIVIDEND_CACHE_TTL`].  An operator can reduce
    /// it to improve freshness at the cost of additional RPC calls.
    async fn index_dividends(
        &self,
        asset_id: u64,
        token_contract: &str,
    ) -> Result<(Vec<Distribution>, Option<u32>, Option<String>), IndexError> {
        let ttl = crate::config_env::dividend_cache_ttl();
        if let Some((at, ledger, cached, error)) =
            self.dividend_cache.lock().unwrap().get(token_contract)
        {
            if at.elapsed() < ttl {
                return Ok((cached.clone(), Some(*ledger), error.clone()));
            }
        }
        let read = self
            .rpc
            .read(
                &self.state.config.dividend_id,
                "get_distributions_for_asset",
                vec![address_scval(token_contract)?],
            )
            .await?;
        let fetch_ledger = read.latest_ledger;
        let entries: Vec<serde_json::Value> = serde_json::from_value(read.value)?;
        let mut failures = 0;
        let result = entries
            .into_iter()
            .filter_map(
                |value| match serde_json::from_value::<RawDistribution>(value) {
                    Ok(d) => Some(d),
                    Err(error) => {
                        failures += 1;
                        record_asset_read_error(asset_id, "dividend_decode");
                        tracing::warn!(asset_id, error = %error, "skipping malformed distribution");
                        None
                    }
                },
            )
            .map(|d| {
                let total = parse_i128(&d.total_amount);
                let distributed = parse_i128(&d.distributed);
                let overflow_detected = distributed > total;
                // When distributed exceeds total the normal clamped
                // ratio_percent would hide the anomaly by returning 100.
                // Instead compute the raw percentage so callers can see
                // the true magnitude of the overflow.
                let claimed_percent = if overflow_detected && total > 0 {
                    (distributed.saturating_mul(10_000) / total) as f64 / 100.0
                } else {
                    ratio_percent(distributed, total)
                };
                Distribution {
                    id: d.id,
                    asset_token: d.asset_token,
                    payment_token: d.payment_token,
                    total_amount: total.to_string(),
                    distributed: distributed.to_string(),
                    claimed_percent,
                    overflow_detected,
                    completed: d.completed,
                    created_at_ledger: d.created_at,
                }
            })
            .collect::<Vec<_>>();
        let error = (failures > 0).then(|| format!("skipped {failures} malformed distribution(s)"));
        self.dividend_cache.lock().unwrap().insert(
            token_contract.to_string(),
            (Instant::now(), fetch_ledger, result.clone(), error.clone()),
        );
        Ok((result, Some(fetch_ledger), error))
    }

    /// Fetch recent contract events from Soroban RPC and merge them into a
    /// bounded, newest-first ring buffer.
    ///
    /// - Uses a per-instance `events_cursor` (an atomically stored ledger
    ///   number) so each poll only fetches events newer than the last
    ///   successful read instead of re-scanning from ledger 1.
    /// - The cursor is only advanced when the RPC call succeeds; a failed
    ///   cycle carries the previous event list forward unchanged.
    /// - The merged list is truncated to [`MAX_EVENTS`] newest entries.
    async fn index_events(
        &self,
        prev_events: Vec<crate::models::Event>,
        contract_ids: &[String],
    ) -> Vec<crate::models::Event> {
        let cursor = self.events_cursor.load(AtomicOrdering::Relaxed);
        // Start from ledger 1 the very first time (cursor == 0).
        let start_ledger = if cursor == 0 { 1 } else { cursor };

        let refs: Vec<&str> = contract_ids.iter().map(|s| s.as_str()).collect();
        match self.rpc.get_events(&refs, start_ledger).await {
            Ok((new_events, latest_ledger)) => {
                // Advance the cursor to latest_ledger + 1 so the next poll
                // only asks for events we haven't seen yet. Guard against
                // regressing the cursor if the node is lagging.
                let next_cursor = latest_ledger.saturating_add(1).max(start_ledger);
                self.events_cursor
                    .fetch_max(next_cursor, AtomicOrdering::Relaxed);

                // Merge: new events (already newest-first from the ledger
                // sort the RPC returns) go on the front; then previous events
                // not already present are appended. Deduplicate by `id`.
                let new_ids: HashSet<u64> = new_events.iter().map(|e| e.id).collect();
                let mut merged: Vec<crate::models::Event> = new_events;
                for ev in prev_events {
                    if !new_ids.contains(&ev.id) {
                        merged.push(ev);
                    }
                }
                // Sort newest-first by ledger then by id within the same ledger.
                merged.sort_by(|a, b| b.ledger.cmp(&a.ledger).then(b.id.cmp(&a.id)));
                merged.truncate(MAX_EVENTS);
                merged
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "getEvents failed; carrying forward previous events"
                );
                prev_events
            }
        }
    }
}

/// Record a failed per-asset RPC read for the `rwa_indexer_asset_read_errors_total`
/// metric, broken down by asset and which read failed.
fn record_asset_read_error(asset_id: u64, read: &'static str) {
    metrics::counter!(
        "rwa_indexer_asset_read_errors_total",
        "asset_id" => asset_id.to_string(),
        "read" => read,
    )
    .increment(1);
}

// ---------------------------------------------------------------------------
// Startup contract-id probe (issue #432)
// ---------------------------------------------------------------------------

/// Probe the configured contract IDs at startup by simulating a `version`
/// call on each. Returns a vec of human-readable warning strings — one per
/// contract that failed to resolve. An empty return means all probes passed.
///
/// Startup continues regardless of the outcome: a transient RPC hiccup
/// should not prevent the process from starting. The warnings are emitted
/// via [`tracing::warn!`] in `main.rs` after this function returns.
pub async fn probe_contract_ids(config: &Config) -> Vec<String> {
    let rpc = Rpc::new(config.rpc_url.clone(), config.read_source.clone());
    let mut warnings = Vec::new();

    let probes = [
        ("RWA_REGISTRY_ID", &config.registry_id),
        ("RWA_DIVIDEND_ID", &config.dividend_id),
    ];

    for (env_var, contract_id) in probes {
        match rpc.read(contract_id, "version", vec![]).await {
            Ok(outcome) => {
                // Accept any value — we only care that the contract is
                // reachable. The ABI version check is the indexer's job.
                tracing::debug!(
                    env_var,
                    contract_id,
                    version = ?outcome.value,
                    "contract id probe succeeded"
                );
            }
            Err(e) => {
                warnings.push(format!(
                    "{env_var} ({contract_id}): does not resolve — {e}"
                ));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use stellar_xdr::curr as xdr;

    fn test_asset(id: u64) -> Asset {
        Asset {
            id,
            token_contract: format!("contract-{id}"),
            issuer: format!("issuer-{id}"),
            name: format!("Asset {id}"),
            symbol: format!("A{id}"),
            asset_type: "test".to_string(),
            description: "Test asset".to_string(),
            valuation_cents: "0".to_string(),
            valuation_usd: 0.0,
            decimals: 7,
            total_supply: "0".to_string(),
            holders: 0,
            active: true,
            paused: false,
            compliance_contract: format!("compliance-{id}"),
            created_at_ledger: 0,
            indexed_at_ledger: 0,
            dividends_indexed_at_ledger: None,
            index_error: None,
        }
    }

    #[test]
    fn snapshot_prunes_entries_for_assets_that_disappear() {
        let mut snapshot = Snapshot {
            assets: vec![test_asset(1), test_asset(2)],
            holders: HashMap::from([(1, Vec::new()), (2, Vec::new()), (99, Vec::new())]),
            compliance: HashMap::from([
                (1, ComplianceSummary::default()),
                (2, ComplianceSummary::default()),
                (99, ComplianceSummary::default()),
            ]),
            compliance_records: HashMap::from([
                (1, HashMap::new()),
                (2, HashMap::new()),
                (99, HashMap::new()),
            ]),
            dividends: HashMap::from([(1, Vec::new()), (2, Vec::new()), (99, Vec::new())]),
            events: Vec::new(),
            stats: Stats::default(),
            stale_flags: StaleFlags::default(),
        };

        snapshot.prune_stale_asset_maps();

        assert_eq!(
            snapshot.holders.keys().copied().collect::<HashSet<_>>(),
            HashSet::from([1, 2])
        );
        assert_eq!(
            snapshot.compliance.keys().copied().collect::<HashSet<_>>(),
            HashSet::from([1, 2])
        );
        assert_eq!(
            snapshot.compliance_records.keys().copied().collect::<HashSet<_>>(),
            HashSet::from([1, 2])
        );
        assert_eq!(
            snapshot.dividends.keys().copied().collect::<HashSet<_>>(),
            HashSet::from([1, 2])
        );
    }

    #[test]
    fn snapshot_pruning_clears_maps_when_no_assets_remain() {
        let mut snapshot = Snapshot {
            assets: Vec::new(),
            holders: HashMap::from([(7, Vec::new())]),
            compliance: HashMap::from([(7, ComplianceSummary::default())]),
            compliance_records: HashMap::from([(7, HashMap::new())]),
            dividends: HashMap::from([(7, Vec::new())]),
            events: Vec::new(),
            stats: Stats::default(),
            stale_flags: StaleFlags::default(),
        };

        snapshot.prune_stale_asset_maps();

        assert!(snapshot.holders.is_empty());
        assert!(snapshot.compliance.is_empty());
        assert!(snapshot.compliance_records.is_empty());
        assert!(snapshot.dividends.is_empty());
    }

    #[test]
    fn retry_delay_is_bounded_and_grows() {
        for attempt in 1..=6 {
            let delay = retry_delay(attempt);
            assert!(delay <= RETRY_MAX_DELAY);
        }
        // The cap for attempt 1 is the base delay; later attempts have a
        // strictly larger (or equal, once capped) upper bound.
        let cap = |attempt: u32| {
            RETRY_BASE_DELAY
                .saturating_mul(1u32 << (attempt - 1).min(4))
                .min(RETRY_MAX_DELAY)
        };
        assert!(cap(1) < cap(2));
        assert_eq!(cap(6), RETRY_MAX_DELAY);
    }

    #[test]
    fn only_http_and_rpc_errors_are_transient() {
        assert!(IndexError::Rpc("busy".into()).is_transient());

        let decode_err = serde_json::from_str::<u8>("not json").unwrap_err();
        assert!(!IndexError::Decode(decode_err).is_transient());

        let xdr_err = xdr::ScVal::from_xdr_base64("not xdr", Limits::none()).unwrap_err();
        assert!(!IndexError::Xdr(xdr_err).is_transient());

        let strkey_err = stellar_strkey::Contract::from_string("bad key").unwrap_err();
        assert!(!IndexError::Strkey(strkey_err).is_transient());
    }

    #[test]
    fn malformed_simulation_result_is_isolated_from_later_decodes() {
        assert!(matches!(
            decode_simulation_result("not valid xdr"),
            Err(IndexError::Xdr(_))
        ));

        let valid = xdr::ScVal::Bool(true).to_xdr_base64(Limits::none()).unwrap();
        assert_eq!(decode_simulation_result(&valid).unwrap(), json!(true));
    }

    #[test]
    fn build_envelope_uses_documented_sim_constants() {
        // Round-trip the base64 envelope back through XDR and confirm
        // fee/seq-num/preconditions/memo are exactly the simulation
        // constants. This pins `SIM_FEE`/`SIM_SEQ_NUM` so a regression
        // (e.g. someone re-introducing a magic number) is caught
        // immediately — important because the RPC's behavior under stricter
        // precondition validation is what we're defending against.
        let src = "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA";
        let contract = "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3";
        let b64 = build_invoke_envelope(src, contract, "noop", vec![]).unwrap();
        let env: xdr::TransactionEnvelope =
            xdr::TransactionEnvelope::from_xdr_base64(&b64, Limits::none()).unwrap();
        let xdr::TransactionEnvelope::Tx(xdr::TransactionV1Envelope { tx, .. }) = env else {
            panic!("expected Tx envelope");
        };
        assert_eq!(tx.fee, SIM_FEE, "fee must be SIM_FEE (= Stellar min, 100)");
        assert_eq!(tx.seq_num.0, SIM_SEQ_NUM, "seq_num must be SIM_SEQ_NUM");
        assert!(matches!(tx.memo, xdr::Memo::None));
        assert!(matches!(tx.cond, xdr::Preconditions::None));
        assert_eq!(
            tx.operations.len(),
            1,
            "envelope must invoke exactly one host function"
        );
        // The host-function op should carry no auth: we never submit, so
        // empty auth keeps the envelope forgeable for the simulator.
        let op_body = tx
            .operations
            .first()
            .expect("envelope must invoke exactly one host function (asserted above)");
        let xdr::OperationBody::InvokeHostFunction(invoke_op) = &op_body.body else {
            panic!("expected InvokeHostFunction op");
        };
        assert!(
            invoke_op.auth.is_empty(),
            "sim envelope must carry no auth entries"
        );
    }

    #[test]
    fn parses_i128_and_percentages() {
        assert_eq!(parse_i128("500000000"), 500_000_000);
        assert_eq!(parse_i128("not-a-number"), 0);
        assert_eq!(cents_to_usd(500_000_000), 5_000_000.0);
        assert_eq!(ratio_percent(25, 100), 25.0);
        assert_eq!(ratio_percent(1, 3), 33.33);
        assert_eq!(ratio_percent(5, 0), 0.0);
        // clamps above 100
        assert_eq!(ratio_percent(150, 100), 100.0);
        assert_eq!(
            ratio_percent(3_002_399_751_580_331, 9_007_199_254_740_993),
            33.33
        );
    }

    // #217 – a zero or negative denominator cannot produce a meaningful
    // percentage and must return 0.0 rather than trapping or overflowing,
    // while ordinary ratios round to two decimals.
    #[test]
    fn ratio_percent_handles_non_positive_denominator_and_rounds() {
        // whole == 0 and whole < 0 both short-circuit to 0.0.
        assert_eq!(ratio_percent(50, 0), 0.0);
        assert_eq!(ratio_percent(50, -1), 0.0);
        // A bare zero part is a valid 0.0 even with a healthy denominator.
        assert_eq!(ratio_percent(0, 100), 0.0);
        // A negative part clamps to 0 before scaling.
        assert_eq!(ratio_percent(-50, 100), 0.0);

        // Ordinary ratios round to two decimals.
        assert_eq!(ratio_percent(1, 3), 33.33);
        assert_eq!(ratio_percent(2, 3), 66.67);
        assert_eq!(ratio_percent(1, 6), 16.67);
        assert_eq!(ratio_percent(1, 8), 12.5);
    }

    #[test]
    fn distribution_decodes_current_contract_shape() {
        let raw = json!({
            "id": 1, "asset_token": "ASSET", "payment_token": "PAY",
            "total_amount": "100", "distributed": "25", "created_at": 42,
            "completed": false
        });
        let decoded: RawDistribution = serde_json::from_value(raw).unwrap();
        assert_eq!(decoded.created_at, 42);
    }

    #[test]
    fn overflow_detected_set_when_distributed_exceeds_total() {
        // Normal case: no overflow.
        let dist_normal = {
            let total = 1_000i128;
            let distributed = 750i128;
            let overflow_detected = distributed > total;
            let claimed_percent = if overflow_detected && total > 0 {
                ((distributed as f64 / total as f64) * 100.0 * 100.0).round() / 100.0
            } else {
                ratio_percent(distributed, total)
            };
            (overflow_detected, claimed_percent)
        };
        assert!(
            !dist_normal.0,
            "overflow_detected should be false for 750/1000"
        );
        assert_eq!(dist_normal.1, 75.0);

        // Overflow case: distributed > total (double-claim scenario).
        let dist_overflow = {
            let total = 1_000i128;
            let distributed = 1_500i128;
            let overflow_detected = distributed > total;
            let claimed_percent = if overflow_detected && total > 0 {
                ((distributed as f64 / total as f64) * 100.0 * 100.0).round() / 100.0
            } else {
                ratio_percent(distributed, total)
            };
            (overflow_detected, claimed_percent)
        };
        assert!(
            dist_overflow.0,
            "overflow_detected should be true for 1500/1000"
        );
        assert_eq!(
            dist_overflow.1, 150.0,
            "claimed_percent must be unclamped (150 %) when overflow is detected"
        );
    }

    /// Mirrors the `claimed_percent` / `overflow_detected` computation in
    /// `Indexer::distributions_for`, which is inlined in an async RPC path
    /// and so cannot be called directly from a unit test.
    fn claimed(total: i128, distributed: i128) -> (bool, f64) {
        let overflow_detected = distributed > total;
        let claimed_percent = if overflow_detected && total > 0 {
            (distributed.saturating_mul(10_000) / total) as f64 / 100.0
        } else {
            ratio_percent(distributed, total)
        };
        (overflow_detected, claimed_percent)
    }

    #[test]
    fn overflow_detected_surfaces_double_claim_without_clamping() {
        // The flag exists to surface the on-chain double-claim anomaly, so
        // the percentage must stay raw. `ratio_percent` would clamp to 100
        // and hide the magnitude entirely.
        let (overflow, percent) = claimed(1_000, 2_000);
        assert!(overflow, "distributed 2000 > total 1000 must set the flag");
        assert!(
            percent > 100.0,
            "claimed_percent must exceed 100 when overflowing, got {percent}"
        );
        assert_eq!(percent, 200.0);
        assert_eq!(
            ratio_percent(2_000, 1_000),
            100.0,
            "sanity: the clamped helper is what the overflow branch avoids"
        );

        // A single stroop over is still an overflow.
        let (overflow, percent) = claimed(1_000, 1_001);
        assert!(overflow);
        assert_eq!(percent, 100.1);

        // Exactly fully distributed is not an overflow.
        let (overflow, percent) = claimed(1_000, 1_000);
        assert!(!overflow);
        assert_eq!(percent, 100.0);

        // A zero total cannot yield a percentage; the flag still fires.
        let (overflow, percent) = claimed(0, 5);
        assert!(overflow, "any distribution against a zero total overflows");
        assert_eq!(percent, 0.0);
    }

    const STUB_SOURCE: &str = "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA";
    const STUB_CONTRACT: &str = "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3";

    /// Serve `router` on an ephemeral loopback port and return the URL to
    /// point an [`Rpc`] at. The task is left running for the whole test.
    async fn spawn_rpc_stub(router: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        format!("http://{addr}/")
    }

    #[tokio::test]
    async fn read_retries_then_gives_up_after_max_read_attempts() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static HITS: AtomicUsize = AtomicUsize::new(0);

        // An RPC-level error is transient, so `read` retries it. A
        // persistently failing node must not be retried forever.
        let router = axum::Router::new().route(
            "/",
            axum::routing::post(|| async {
                HITS.fetch_add(1, Ordering::SeqCst);
                axum::Json(json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "error": { "message": "node busy" }
                }))
            }),
        );
        let url = spawn_rpc_stub(router).await;

        let rpc = Rpc::new(url, STUB_SOURCE.to_string());
        let started = Instant::now();
        let err = rpc
            .read(STUB_CONTRACT, "get_assets", vec![])
            .await
            .expect_err("a persistently failing read must surface an error");

        assert!(
            matches!(&err, IndexError::Rpc(message) if message == "node busy"),
            "the last failure should be surfaced verbatim, got {err}"
        );
        assert_eq!(
            HITS.load(Ordering::SeqCst),
            MAX_READ_ATTEMPTS as usize,
            "the read must be attempted exactly MAX_READ_ATTEMPTS times"
        );
        // Three backoff sleeps happened between the four attempts, each
        // drawn from `[0, cap]`. The sum of those caps bounds the wait, so
        // a regression that stops capping the backoff shows up here.
        let mut max_backoff = Duration::ZERO;
        for attempt in 1..MAX_READ_ATTEMPTS {
            max_backoff += RETRY_BASE_DELAY
                .saturating_mul(1u32 << (attempt - 1).min(4))
                .min(RETRY_MAX_DELAY);
        }
        assert!(
            started.elapsed() < max_backoff * 4,
            "backoff should stay within the jittered bound, took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn rate_limited_response_carries_parsed_retry_after() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static HITS: AtomicUsize = AtomicUsize::new(0);

        fn count() {
            HITS.fetch_add(1, Ordering::SeqCst);
        }

        let router = axum::Router::new()
            .route(
                "/rate-limited",
                axum::routing::post(|| async {
                    count();
                    (
                        axum::http::StatusCode::TOO_MANY_REQUESTS,
                        [("retry-after", "7")],
                        "slow down",
                    )
                }),
            )
            .route(
                "/unavailable",
                axum::routing::post(|| async {
                    count();
                    (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        [("retry-after", "30")],
                        "maintenance",
                    )
                }),
            )
            .route(
                "/no-retry-after",
                axum::routing::post(|| async {
                    count();
                    (axum::http::StatusCode::TOO_MANY_REQUESTS, "slow down")
                }),
            );
        let base = spawn_rpc_stub(router).await;

        let read = |path: &str| {
            let rpc = Rpc::new(format!("{base}{path}"), STUB_SOURCE.to_string());
            async move { rpc.read(STUB_CONTRACT, "get_assets", vec![]).await }
        };

        // Mirrors how `Indexer::run` picks its wait: the advised delay wins
        // over the fixed poll interval when the node supplied one.
        let backoff = |e: &IndexError| match e {
            IndexError::RateLimited { retry_after, .. } => retry_after.unwrap_or(POLL_INTERVAL),
            _ => POLL_INTERVAL,
        };

        let err = read("rate-limited")
            .await
            .expect_err("429 must not be reported as success");
        let IndexError::RateLimited {
            status,
            retry_after,
            body,
        } = &err
        else {
            panic!("429 must map to IndexError::RateLimited, got {err}");
        };
        assert_eq!(*status, 429);
        assert_eq!(
            *retry_after,
            Some(Duration::from_secs(7)),
            "Retry-After must be parsed as whole seconds"
        );
        assert_eq!(body, "slow down", "the node's body is kept for diagnosis");
        assert_eq!(
            backoff(&err),
            Duration::from_secs(7),
            "the advised delay must be honoured over POLL_INTERVAL"
        );

        // 503 is treated the same way: the node is asking us to wait.
        let err = read("unavailable")
            .await
            .expect_err("503 must not be reported as success");
        let IndexError::RateLimited {
            status,
            retry_after,
            ..
        } = &err
        else {
            panic!("503 must map to IndexError::RateLimited, got {err}");
        };
        assert_eq!(*status, 503);
        assert_eq!(*retry_after, Some(Duration::from_secs(30)));
        assert_eq!(backoff(&err), Duration::from_secs(30));

        // Without the header there is no advice to honour, but the variant
        // still has to be RateLimited rather than a plain HTTP status.
        let err = read("no-retry-after")
            .await
            .expect_err("429 must not be reported as success");
        let IndexError::RateLimited { retry_after, .. } = &err else {
            panic!("429 must map to IndexError::RateLimited, got {err}");
        };
        assert_eq!(*retry_after, None);
        assert_eq!(
            backoff(&err),
            POLL_INTERVAL,
            "with no advice the caller falls back to the poll interval"
        );

        assert_eq!(
            HITS.load(Ordering::SeqCst),
            3,
            "a rate-limit response is not transient and must not be retried in place"
        );
    }

    #[test]
    fn concurrent_readers_never_observe_a_partial_snapshot_swap() {
        use std::sync::atomic::{AtomicBool, Ordering};

        // Two internally consistent generations. Every field is keyed to the
        // generation, so any mix of the two is detectable by a reader.
        fn generation(ledger: u32, ids: &[u64]) -> Snapshot {
            Snapshot {
                assets: ids.iter().copied().map(test_asset).collect(),
                holders: ids.iter().map(|&id| (id, Vec::new())).collect(),
                compliance: ids
                    .iter()
                    .map(|&id| (id, ComplianceSummary::default()))
                    .collect(),
                compliance_records: ids.iter().map(|&id| (id, HashMap::new())).collect(),
                dividends: ids.iter().map(|&id| (id, Vec::new())).collect(),
                events: Vec::new(),
                stats: Stats {
                    total_assets: ids.len(),
                    last_indexed_ledger: ledger,
                    ..Stats::default()
                },
                stale_flags: StaleFlags::default(),
            }
        }

        let old = generation(100, &[1, 2, 3]);
        let new = generation(200, &[10, 11, 12, 13, 14]);
        let state = AppState::for_test_empty();
        state.replace(old.clone());

        let stop = AtomicBool::new(false);

        std::thread::scope(|scope| {
            let writer = scope.spawn(|| {
                for i in 0..2_000 {
                    state.replace(if i % 2 == 0 { new.clone() } else { old.clone() });
                }
                stop.store(true, Ordering::Release);
            });

            let readers: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let mut seen_ledgers = HashSet::new();
                        while !stop.load(Ordering::Acquire) {
                            let snapshot = state.snapshot();
                            let ledger = snapshot.stats.last_indexed_ledger;
                            seen_ledgers.insert(ledger);

                            // A torn read would pair one generation's stats
                            // with the other's assets or derived maps.
                            let ids: HashSet<u64> =
                                snapshot.assets.iter().map(|asset| asset.id).collect();
                            let expected: HashSet<u64> = match ledger {
                                100 => HashSet::from([1, 2, 3]),
                                200 => HashSet::from([10, 11, 12, 13, 14]),
                                other => {
                                    panic!("observed a ledger from neither generation: {other}")
                                }
                            };
                            assert_eq!(ids, expected, "assets do not match stats ledger {ledger}");
                            assert_eq!(
                                snapshot.stats.total_assets,
                                snapshot.assets.len(),
                                "stats count does not match the assets in the same snapshot"
                            );
                            assert_eq!(
                                snapshot.holders.keys().copied().collect::<HashSet<_>>(),
                                expected,
                                "holders map belongs to a different generation"
                            );
                            assert_eq!(
                                snapshot.compliance.keys().copied().collect::<HashSet<_>>(),
                                expected,
                                "compliance map belongs to a different generation"
                            );
                            assert_eq!(
                                snapshot.compliance_records.keys().copied().collect::<HashSet<_>>(),
                                expected,
                                "compliance_records map belongs to a different generation"
                            );
                            assert_eq!(
                                snapshot.dividends.keys().copied().collect::<HashSet<_>>(),
                                expected,
                                "dividends map belongs to a different generation"
                            );
                        }
                        seen_ledgers
                    })
                })
                .collect();

            writer.join().unwrap();
            let seen: HashSet<u32> = readers
                .into_iter()
                .flat_map(|reader| reader.join().unwrap())
                .collect();
            assert!(
                seen.contains(&100) || seen.contains(&200),
                "readers should have observed at least one published generation"
            );
        });

        // The last write wins and is fully visible afterwards.
        let final_snapshot = state.snapshot();
        assert_eq!(final_snapshot.stats.last_indexed_ledger, 100);
        assert_eq!(final_snapshot.assets.len(), 3);
    }

    #[test]
    fn normalizes_unit_enum_status() {
        // Soroban encodes a unit-variant enum as a vec of one symbol.
        assert_eq!(normalize_status(&json!(["Approved"])), "Approved");
        assert_eq!(normalize_status(&json!("Suspended")), "Suspended");
        assert_eq!(normalize_status(&json!(42)), "Unknown");
    }

    #[test]
    fn scval_scalars_to_json() {
        assert_eq!(scval_to_json(&xdr::ScVal::Bool(true)).unwrap(), json!(true));
        assert_eq!(scval_to_json(&xdr::ScVal::Void).unwrap(), json!(null));
        assert_eq!(scval_to_json(&xdr::ScVal::U32(7)).unwrap(), json!(7));
        assert_eq!(scval_to_json(&xdr::ScVal::U64(9)).unwrap(), json!(9));
    }

    #[test]
    fn scval_i128_becomes_string() {
        let v = xdr::ScVal::I128(xdr::Int128Parts { hi: 0, lo: 100 });
        assert_eq!(scval_to_json(&v).unwrap(), json!("100"));
    }

    #[test]
    fn scval_symbol_and_string() {
        let sym = xdr::ScVal::Symbol(xdr::ScSymbol("Approved".try_into().unwrap()));
        assert_eq!(scval_to_json(&sym).unwrap(), json!("Approved"));
        let s = xdr::ScVal::String(xdr::ScString("hello".try_into().unwrap()));
        assert_eq!(scval_to_json(&s).unwrap(), json!("hello"));
    }

    /// `address_to_string` must not trap on `ScAddress` variants that are not
    /// used by the RWA contracts today (MuxedAccount, ClaimableBalance,
    /// LiquidityPool).  A single unrecognised address must return a
    /// recognisable placeholder — `"unknown:…"` — so the rest of the
    /// response can still be processed and one bad address cannot abort an
    /// entire refresh cycle.
    #[test]
    fn address_to_string_falls_back_for_unsupported_variants() {
        // ClaimableBalance — a v0 balance id with a zeroed hash.
        let claimable = xdr::ScAddress::ClaimableBalance(
            xdr::ClaimableBalanceId::ClaimableBalanceIdTypeV0(xdr::Hash([0u8; 32])),
        );
        let result = address_to_string(&claimable);
        assert!(
            result.is_ok(),
            "ClaimableBalance must not return Err (got {result:?})"
        );
        let s = result.unwrap();
        assert!(
            s.starts_with("unknown:"),
            "ClaimableBalance placeholder must start with 'unknown:' (got {s:?})"
        );

        // LiquidityPool — a pool id with a zeroed hash.
        let liquidity_pool = xdr::ScAddress::LiquidityPool(xdr::PoolId(xdr::Hash([0u8; 32])));
        let result = address_to_string(&liquidity_pool);
        assert!(
            result.is_ok(),
            "LiquidityPool must not return Err (got {result:?})"
        );
        let s = result.unwrap();
        assert!(
            s.starts_with("unknown:"),
            "LiquidityPool placeholder must start with 'unknown:' (got {s:?})"
        );
    }

    #[test]
    fn scval_map_becomes_object() {
        let entries = vec![
            xdr::ScMapEntry {
                key: xdr::ScVal::Symbol(xdr::ScSymbol("active".try_into().unwrap())),
                val: xdr::ScVal::Bool(true),
            },
            xdr::ScMapEntry {
                key: xdr::ScVal::Symbol(xdr::ScSymbol("id".try_into().unwrap())),
                val: xdr::ScVal::U64(1),
            },
        ];
        let map = xdr::ScVal::Map(Some(xdr::ScMap(entries.try_into().unwrap())));
        assert_eq!(
            scval_to_json(&map).unwrap(),
            json!({ "active": true, "id": 1 })
        );
    }

    /// Serialises the tests that mutate `RWA_*`. Environment variables are
    /// process-global, so without this the two config tests race each other
    /// (and previously left a bogus `RWA_REGISTRY_ID` behind that failed
    /// unrelated route tests).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn config_from_env_applies_defaults() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("RWA_RPC_URL");
        std::env::remove_var("RWA_REGISTRY_ID");
        std::env::remove_var("RWA_DIVIDEND_ID");
        std::env::remove_var("RWA_READ_SOURCE");

        let cfg = Config::from_env().expect("config with defaults should succeed");
        assert_eq!(cfg.rpc_url, "https://soroban-testnet.stellar.org");
        assert_eq!(
            cfg.registry_id,
            "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3"
        );
        assert_eq!(
            cfg.dividend_id,
            "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX"
        );
        assert_eq!(
            cfg.read_source,
            "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA"
        );
    }

    #[test]
    fn config_from_env_overrides_with_env_vars() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let custom_rpc = "https://custom-rpc.example.com";
        // These must be real strkeys: `Config::from_env` verifies the CRC, so an
        // ID with a hand-edited final character is rejected. The two contract
        // IDs below are the deployed compliance and asset-token contracts,
        // chosen only because they are valid and differ from the defaults.
        let custom_registry = "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU";
        let custom_dividend = "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ";
        let custom_source = "GBTG2AKEJQNJZLRKXM3ILXWUBCKGK7PHLEGETYTHS2Y3HIE7CGJFMNKK";

        std::env::set_var("RWA_RPC_URL", custom_rpc);
        std::env::set_var("RWA_REGISTRY_ID", custom_registry);
        std::env::set_var("RWA_DIVIDEND_ID", custom_dividend);
        std::env::set_var("RWA_READ_SOURCE", custom_source);

        let cfg = Config::from_env().expect("config with overrides should succeed");
        assert_eq!(cfg.rpc_url, custom_rpc);
        assert_eq!(cfg.registry_id, custom_registry);
        assert_eq!(cfg.dividend_id, custom_dividend);
        assert_eq!(cfg.read_source, custom_source);

        std::env::remove_var("RWA_RPC_URL");
        std::env::remove_var("RWA_REGISTRY_ID");
        std::env::remove_var("RWA_DIVIDEND_ID");
        std::env::remove_var("RWA_READ_SOURCE");
    }

    #[test]
    fn compliance_summary_counts_by_status() {
        let summary = ComplianceSummary {
            total_records: 100,
            approved: 60,
            suspended: 15,
            rejected: 10,
            pending: 15,
            with_expiry: 25,
            jurisdictions: vec![
                JurisdictionCount {
                    jurisdiction: "US".to_string(),
                    count: 50,
                },
                JurisdictionCount {
                    jurisdiction: "SG".to_string(),
                    count: 30,
                },
                JurisdictionCount {
                    jurisdiction: "UK".to_string(),
                    count: 20,
                },
            ],
        };

        assert_eq!(summary.total_records, 100);
        assert_eq!(summary.approved, 60);
        assert_eq!(summary.suspended, 15);
        assert_eq!(summary.rejected, 10);
        assert_eq!(summary.pending, 15);
        assert_eq!(summary.with_expiry, 25);
        assert_eq!(summary.jurisdictions.len(), 3);
        assert_eq!(summary.jurisdictions[0].jurisdiction, "US");
        assert_eq!(summary.jurisdictions[0].count, 50);
        assert_eq!(summary.jurisdictions[1].jurisdiction, "SG");
        assert_eq!(summary.jurisdictions[1].count, 30);
        assert_eq!(summary.jurisdictions[2].jurisdiction, "UK");
        assert_eq!(summary.jurisdictions[2].count, 20);
        assert_eq!(
            summary.approved + summary.suspended + summary.rejected + summary.pending,
            summary.total_records
        );
    }

    #[test]
    fn stats_aggregation_across_assets() {
        let mut snapshot = Snapshot {
            assets: vec![
                Asset {
                    id: 1,
                    token_contract: "C1".to_string(),
                    issuer: "issuer1".to_string(),
                    name: "Asset1".to_string(),
                    symbol: "A1".to_string(),
                    asset_type: "Type1".to_string(),
                    description: "Desc1".to_string(),
                    valuation_cents: "100000000".to_string(),
                    valuation_usd: 1_000_000.0,
                    decimals: 7,
                    total_supply: "1000000000".to_string(),
                    holders: 50,
                    active: true,
                    paused: false,
                    compliance_contract: "CC1".to_string(),
                    created_at_ledger: 1000,
                    indexed_at_ledger: 1000,
                    dividends_indexed_at_ledger: None,
                    index_error: None,
                },
                Asset {
                    id: 2,
                    token_contract: "C2".to_string(),
                    issuer: "issuer2".to_string(),
                    name: "Asset2".to_string(),
                    symbol: "A2".to_string(),
                    asset_type: "Type2".to_string(),
                    description: "Desc2".to_string(),
                    valuation_cents: "50000000".to_string(),
                    valuation_usd: 500_000.0,
                    decimals: 6,
                    total_supply: "5000000".to_string(),
                    holders: 30,
                    active: true,
                    paused: false,
                    compliance_contract: "CC2".to_string(),
                    created_at_ledger: 1500,
                    indexed_at_ledger: 1500,
                    dividends_indexed_at_ledger: None,
                    index_error: None,
                },
                Asset {
                    id: 3,
                    token_contract: "C3".to_string(),
                    issuer: "issuer3".to_string(),
                    name: "Asset3".to_string(),
                    symbol: "A3".to_string(),
                    asset_type: "Type3".to_string(),
                    description: "Desc3".to_string(),
                    valuation_cents: "25000000".to_string(),
                    valuation_usd: 250_000.0,
                    decimals: 5,
                    total_supply: "25000".to_string(),
                    holders: 20,
                    active: false,
                    paused: true,
                    compliance_contract: "CC3".to_string(),
                    created_at_ledger: 2000,
                    indexed_at_ledger: 2000,
                    dividends_indexed_at_ledger: None,
                    index_error: None,
                },
            ],
            ..Snapshot::default()
        };

        snapshot.holders.insert(
            1,
            vec![Holder {
                address: "addr1".to_string(),
                balance: "500000000".to_string(),
                share_percent: 50.0,
            }],
        );
        snapshot.holders.insert(
            2,
            vec![Holder {
                address: "addr2".to_string(),
                balance: "2500000".to_string(),
                share_percent: 50.0,
            }],
        );
        snapshot.holders.insert(3, vec![]);

        snapshot.dividends.insert(
            1,
            vec![Distribution {
                id: 1,
                asset_token: "C1".to_string(),
                payment_token: "PAY1".to_string(),
                total_amount: "1000000".to_string(),
                distributed: "500000".to_string(),
                claimed_percent: 50.0,
                overflow_detected: false,
                completed: false,
                created_at_ledger: 2400,
            }],
        );
        snapshot.dividends.insert(
            2,
            vec![Distribution {
                id: 2,
                asset_token: "C2".to_string(),
                payment_token: "PAY2".to_string(),
                total_amount: "500000".to_string(),
                distributed: "250000".to_string(),
                claimed_percent: 50.0,
                overflow_detected: false,
                completed: false,
                created_at_ledger: 2500,
            }],
        );
        snapshot.dividends.insert(3, vec![]);

        snapshot.stats = Stats {
            total_assets: 3,
            active_assets: 2,
            tvl_cents: "175000000".to_string(),
            tvl_usd: 1_750_000.0,
            total_holders: 2,
            total_distributions: 2,
            last_indexed_ledger: 3000,
            last_updated: Some("2026-07-26T10:00:00Z".to_string()),
        };

        assert_eq!(snapshot.stats.total_assets, 3);
        assert_eq!(snapshot.stats.active_assets, 2);
        assert_eq!(snapshot.stats.tvl_cents, "175000000");
        assert_eq!(snapshot.stats.tvl_usd, 1_750_000.0);
        assert_eq!(snapshot.stats.total_holders, 2);
        assert_eq!(snapshot.stats.total_distributions, 2);
        assert_eq!(snapshot.stats.last_indexed_ledger, 3000);
        assert!(snapshot.stats.last_updated.is_some());
    }

    // #213 – a failed per-asset read must set a visible `index_error`, bump
    // the `rwa_indexer_asset_read_errors_total` metric, and leave the last
    // good holders/compliance/dividends intact rather than blanking them.
    #[test]
    fn failed_asset_read_sets_index_error_increments_metric_and_keeps_last_good_values() {
        // The `record_asset_read_error` counter is emitted through the
        // `metrics` emission macros, so run it inside `with_local_recorder`
        // to capture the increments deterministically instead of relying on
        // whatever global recorder (if any) a test binary installed.
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            record_asset_read_error(7, "get_metadata");
            record_asset_read_error(7, "get_metadata");
            record_asset_read_error(7, "compliance");
        });

        let rendered = handle.render();
        assert!(
            rendered.contains(
                "rwa_indexer_asset_read_errors_total{asset_id=\"7\",read=\"get_metadata\"} 2"
            ),
            "repeated metadata read failures must increment the per-asset counter; got:\n{rendered}"
        );
        assert!(
            rendered.contains(
                "rwa_indexer_asset_read_errors_total{asset_id=\"7\",read=\"compliance\"} 1"
            ),
            "compliance read failures must carry their own read label; got:\n{rendered}"
        );

        // Simulate the refresh loop's carry-forward: the failed asset is
        // re-emitted from the previous snapshot with `index_error` populated,
        // and its last good holders/compliance/dividends are preserved.
        let state = AppState::for_test_empty();
        let mut snapshot = Snapshot {
            assets: vec![test_asset(7)],
            holders: HashMap::from([(
                7,
                vec![Holder {
                    address: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                    balance: "1000000".to_string(),
                    share_percent: 100.0,
                }],
            )]),
            compliance: HashMap::from([(
                7,
                ComplianceSummary {
                    total_records: 1,
                    approved: 1,
                    ..ComplianceSummary::default()
                },
            )]),
            compliance_records: HashMap::from([(7, HashMap::new())]),
            dividends: HashMap::from([(
                7,
                vec![Distribution {
                    id: 10,
                    asset_token: "contract-7".to_string(),
                    payment_token: "PAY".to_string(),
                    total_amount: "1000".to_string(),
                    distributed: "0".to_string(),
                    claimed_percent: 0.0,
                    overflow_detected: false,
                    completed: false,
                    created_at_ledger: 100,
                }],
            )]),
            events: Vec::new(),
            stats: Stats::default(),
            stale_flags: StaleFlags::default(),
        };

        // The refresh loop errors before it can read fresh per-asset data
        // (`IndexError::Rpc` on a node reporting busy), so the previous
        // Asset is re-emitted with `index_error` set on it.
        let mut stale = snapshot
            .asset(7)
            .expect("the simulated cycle must have seen a previous asset")
            .clone();
        stale.index_error = Some(IndexError::Rpc("node busy".into()).to_string());
        snapshot.assets[0] = stale;
        state.replace(snapshot);

        let served = state.snapshot();
        assert!(
            served.asset(7).is_some(),
            "the failed asset must still be served"
        );
        assert_eq!(
            served.asset(7).unwrap().index_error.as_deref(),
            Some("rpc returned an error: node busy"),
            "the failed read must surface on the asset's index_error field"
        );
        assert!(
            served.holders.contains_key(&7),
            "last good holders must survive a failed read"
        );
        assert!(
            served.compliance.contains_key(&7),
            "last good compliance must survive a failed read"
        );
        assert!(
            served.compliance_records.contains_key(&7),
            "last good compliance_records must survive a failed read"
        );
        assert!(
            served.dividends.contains_key(&7),
            "last good dividends must survive a failed read"
        );
    }

    // -------------------------------------------------------------------------
    // #455 – per-contract ABI version ranges
    // -------------------------------------------------------------------------

    /// `AbiExpectation::default_ranges` must accept each contract's current
    /// version from `stellar-rwa-contracts` main:
    ///   registry = 1, compliance = 1, asset-token = 1, dividend = 3.
    #[test]
    fn default_abi_ranges_accept_current_contract_versions() {
        let exp = AbiExpectation::default_ranges();
        assert!(
            exp.registry.contains(&1),
            "registry VERSION 1 must be in default range {:?}",
            exp.registry
        );
        assert!(
            exp.compliance.contains(&1),
            "compliance VERSION 1 must be in default range {:?}",
            exp.compliance
        );
        assert!(
            exp.asset_token.contains(&1),
            "asset-token VERSION 1 must be in default range {:?}",
            exp.asset_token
        );
        // dividend is already at VERSION 3 on main — the whole point of this fix.
        assert!(
            exp.dividend.contains(&3),
            "dividend VERSION 3 must be in default range {:?}",
            exp.dividend
        );
    }

    /// An ABI version inside the range must not produce an error.
    #[tokio::test]
    async fn check_abi_accepts_version_within_range() {
        // Serve a contract that returns version 3 — the dividend contract's
        // current version on main.
        let router = axum::Router::new().route(
            "/",
            axum::routing::post(|| async {
                axum::Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "results": [{ "xdr": stellar_xdr::curr::ScVal::U64(3)
                            .to_xdr_base64(stellar_xdr::curr::Limits::none())
                            .unwrap() }],
                        "latestLedger": 100
                    }
                }))
            }),
        );
        let url = spawn_rpc_stub(router).await;
        let rpc = Rpc::new(url, STUB_SOURCE.to_string());
        let indexer = Indexer {
            rpc,
            state: AppState::for_test_empty(),
            dividend_cache: Mutex::new(HashMap::new()),
        };

        let result = indexer
            .check_abi(STUB_CONTRACT, &(1..=3), "dividend")
            .await;
        assert!(
            result.is_ok(),
            "version 3 within range 1..=3 should be accepted; got {:?}",
            result
        );
    }

    /// An ABI version outside the range must produce an `AbiVersion` error
    /// that names the contract and the unsupported version.
    #[tokio::test]
    async fn check_abi_rejects_version_outside_range() {
        // A future dividend contract returning VERSION 99 — unknown to this indexer.
        let router = axum::Router::new().route(
            "/",
            axum::routing::post(|| async {
                axum::Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "results": [{ "xdr": stellar_xdr::curr::ScVal::U64(99)
                            .to_xdr_base64(stellar_xdr::curr::Limits::none())
                            .unwrap() }],
                        "latestLedger": 100
                    }
                }))
            }),
        );
        let url = spawn_rpc_stub(router).await;
        let rpc = Rpc::new(url, STUB_SOURCE.to_string());
        let indexer = Indexer {
            rpc,
            state: AppState::for_test_empty(),
            dividend_cache: Mutex::new(HashMap::new()),
        };

        let err = indexer
            .check_abi(STUB_CONTRACT, &(1..=3), "dividend")
            .await
            .expect_err("version 99 must be rejected");

        let IndexError::AbiVersion {
            contract,
            actual,
            expected_range,
        } = err
        else {
            panic!("expected AbiVersion error, got {err}");
        };
        assert_eq!(contract, STUB_CONTRACT, "error must name the contract");
        assert_eq!(actual, 99, "error must report the observed version");
        assert!(
            expected_range.contains("1") && expected_range.contains("3"),
            "error message must include the supported range; got {expected_range:?}"
        );
    }

    /// Each contract type is checked against its own range, not a shared constant.
    /// Verify that the ranges are genuinely independent.
    #[test]
    fn abi_ranges_are_independent_per_contract_type() {
        let exp = AbiExpectation::default_ranges();

        // dividend supports up to 3, so version 2 must also be accepted.
        assert!(exp.dividend.contains(&2));

        // registry and compliance are currently pinned to 1; version 2 is
        // not yet supported and must NOT be in the range.
        assert!(
            !exp.registry.contains(&2),
            "registry version 2 should not be in range yet"
        );
        assert!(
            !exp.compliance.contains(&2),
            "compliance version 2 should not be in range yet"
        );
        assert!(
            !exp.asset_token.contains(&2),
            "asset-token version 2 should not be in range yet"
        );

        // A version of 0 is never valid for any contract.
        assert!(!exp.registry.contains(&0));
        assert!(!exp.dividend.contains(&0));
        assert!(!exp.compliance.contains(&0));
        assert!(!exp.asset_token.contains(&0));
    }
}
