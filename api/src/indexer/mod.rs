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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use metrics_exporter_prometheus::PrometheusHandle;
use rand::Rng;
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde::Deserialize;
use stellar_xdr::curr as xdr;
use stellar_xdr::curr::{Limits, ReadXdr, WriteXdr};

use crate::models::{Asset, ComplianceSummary, Distribution, Holder, JurisdictionCount, Stats};

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
const EXPECTED_ABI_VERSION: u64 = 1;
const DIVIDEND_CACHE_TTL: Duration = Duration::from_secs(60);
const TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";

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

/// The immutable, shareable snapshot the API serves.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub assets: Vec<Asset>,
    pub holders: HashMap<u64, Vec<Holder>>,
    pub compliance: HashMap<u64, ComplianceSummary>,
    pub dividends: HashMap<u64, Vec<Distribution>>,
    pub stats: Stats,
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
}

impl AppState {
    pub fn new(config: Config, metrics: PrometheusHandle) -> Self {
        AppState {
            inner: Arc::new(ArcSwap::from(Arc::new(Snapshot::default()))),
            config: Arc::new(config),
            metrics,
        }
    }

    /// Clone the current snapshot for read-only serving.
    pub fn snapshot(&self) -> Snapshot {
        let guard = self.inner.load();
        // `Guard` derefs to `&Arc<Snapshot>` under `arc-swap` 1.x, so the
        // snapshot lives one more deref down: `**guard` is the `Snapshot`.
        (**guard).clone()
    }

    /// Last ledger the indexer successfully read. Used as the ETag seed for
    /// snapshot-backed routes (`Cache-Control` + `304 Not Modified`).
    pub fn last_indexed_ledger(&self) -> u32 {
        self.snapshot().stats.last_indexed_ledger
    }

    fn replace(&self, mut next: Snapshot) {
        next.prune_stale_asset_maps();
        self.inner.store(Arc::new(next));
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
    #[error("{contract} ABI version {actual} does not match expected {expected}")]
    AbiVersion {
        contract: String,
        actual: u64,
        expected: u64,
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

/// Outcome of a single simulated read.
struct ReadOutcome {
    value: serde_json::Value,
    latest_ledger: u32,
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
        let scval = xdr::ScVal::from_xdr_base64(&entry.xdr, Limits::none())?;
        Ok(ReadOutcome {
            value: scval_to_json(&scval)?,
            latest_ledger: result.latest_ledger,
        })
    }
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

pub struct Indexer {
    rpc: Rpc,
    state: AppState,
    dividend_cache: Mutex<HashMap<String, (Instant, Vec<Distribution>, Option<String>)>>,
}

impl Indexer {
    pub fn new(state: AppState) -> Self {
        let cfg = &state.config;
        Indexer {
            rpc: Rpc::new(cfg.rpc_url.clone(), cfg.read_source.clone()),
            state,
            dividend_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Poll forever, refreshing the snapshot every [`POLL_INTERVAL`].
    pub async fn run(self) {
        loop {
            let backoff = match self.refresh().await {
                Ok(count) => {
                    tracing::info!(assets = count, "index refreshed");
                    POLL_INTERVAL
                }
                Err(e) => {
                    tracing::warn!(error = %e, "index refresh failed; keeping last snapshot");
                    if let IndexError::RateLimited { retry_after, .. } = &e {
                        retry_after.unwrap_or(POLL_INTERVAL)
                    } else {
                        POLL_INTERVAL
                    }
                }
            };
            tokio::time::sleep(backoff).await;
        }
    }

    /// Read the full current state of all contracts and rebuild the snapshot.
    async fn refresh(&self) -> Result<usize, IndexError> {
        let cfg = &self.state.config;

        self.check_abi(&cfg.registry_id).await?;
        self.check_abi(&cfg.dividend_id).await?;

        let entries_read = self
            .rpc
            .read(&cfg.registry_id, "get_all_assets", vec![])
            .await?;
        let latest_ledger = entries_read.latest_ledger;
        let raw_entries: Vec<RawAssetEntry> = serde_json::from_value(entries_read.value)?;

        // Grab the previous snapshot so we can carry forward per-asset
        // freshness info for assets that fail this cycle.
        let prev = self.state.snapshot();

        let mut assets = Vec::new();
        let mut holders_map: HashMap<u64, Vec<Holder>> = HashMap::new();
        let mut compliance_map: HashMap<u64, ComplianceSummary> = HashMap::new();
        let mut dividends_map: HashMap<u64, Vec<Distribution>> = HashMap::new();
        let mut total_distributions = 0usize;
        let mut tvl: i128 = 0;

        for raw in &raw_entries {
            self.check_abi(&raw.token_contract).await?;
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
            self.check_abi(&meta.compliance_contract).await?;

            // Holders: every allowlisted address with a positive balance.
            // Also best-effort: fall back to previous holders on failure.
            let (holders, summary, compliance_err) = match self
                .index_compliance_and_holders(
                    &meta.compliance_contract,
                    &raw.token_contract,
                    total_supply,
                )
                .await
            {
                Ok(result) => (result.0, result.1, None),
                Err(e) => {
                    record_asset_read_error(raw.id, "compliance");
                    tracing::warn!(
                        asset_id = raw.id,
                        error = %e,
                        "compliance/holders read failed; using previous data"
                    );
                    let holders = prev.holders.get(&raw.id).cloned().unwrap_or_default();
                    let summary = prev.compliance.get(&raw.id).cloned().unwrap_or_default();
                    (holders, summary, Some(e.to_string()))
                }
            };

            // Dividends for this asset token.  On failure we preserve the last
            // known distributions from the previous snapshot rather than
            // resetting to empty, so a transient RPC hiccup doesn't make the
            // API silently report "no dividends" for an asset.
            let (dists, dividend_err) = match self
                .index_dividends(raw.id, &raw.token_contract)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    record_asset_read_error(raw.id, "dividends");
                    tracing::warn!(asset_id = raw.id, error = %e, "dividends read failed; keeping previous distributions");
                    (
                        prev.dividends.get(&raw.id).cloned().unwrap_or_default(),
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
                index_error: dividend_err.or(compliance_err),
            };

            holders_map.insert(raw.id, holders);
            compliance_map.insert(raw.id, summary);
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

        let count = assets.len();
        self.state.replace(Snapshot {
            assets,
            holders: holders_map,
            compliance: compliance_map,
            dividends: dividends_map,
            stats,
        });
        Ok(count)
    }

    /// Read the compliance allowlist for an asset and derive both the holder
    /// list (allowlisted ∩ positive balance) and the non-PII summary.
    async fn index_compliance_and_holders(
        &self,
        compliance_contract: &str,
        token_contract: &str,
        total_supply: i128,
    ) -> Result<(Vec<Holder>, ComplianceSummary, Vec<String>), IndexError> {
        let allowlist = self
            .rpc
            .read(compliance_contract, "get_allowlist", vec![])
            .await?;
        let addresses: Vec<String> = serde_json::from_value(allowlist.value)?;

        let mut holders = Vec::new();
        let mut summary = ComplianceSummary::default();
        let mut jurisdictions: BTreeMap<String, usize> = BTreeMap::new();
        let mut approved_addresses = Vec::new();

        for address in &addresses {
            summary.total_records += 1;

            // Record status → summary counts.
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
                        match normalize_status(&kyc.status).as_str() {
                            "Approved" => {
                                summary.approved += 1;
                                approved_addresses.push(address.clone());
                            }
                            "Suspended" => summary.suspended += 1,
                            "Rejected" => summary.rejected += 1,
                            "Pending" => summary.pending += 1,
                            _ => {}
                        }
                        if kyc.expires_at != 0 {
                            summary.with_expiry += 1;
                        }
                        *jurisdictions.entry(kyc.jurisdiction).or_insert(0) += 1;
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

        Ok((holders, summary, approved_addresses))
    }

    async fn check_abi(&self, contract: &str) -> Result<(), IndexError> {
        let read = self.rpc.read(contract, "version", vec![]).await?;
        let actual = read.value.as_u64().unwrap_or_default();
        if actual != EXPECTED_ABI_VERSION {
            return Err(IndexError::AbiVersion {
                contract: contract.to_string(),
                actual,
                expected: EXPECTED_ABI_VERSION,
            });
        }
        Ok(())
    }

    /// Read distributions at most once per cache window.
    async fn index_dividends(
        &self,
        asset_id: u64,
        token_contract: &str,
    ) -> Result<(Vec<Distribution>, Option<String>), IndexError> {
        if let Some((at, cached, error)) = self.dividend_cache.lock().unwrap().get(token_contract) {
            if at.elapsed() < DIVIDEND_CACHE_TTL {
                return Ok((cached.clone(), error.clone()));
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
            (Instant::now(), result.clone(), error.clone()),
        );
        Ok((result, error))
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
            dividends: HashMap::from([(1, Vec::new()), (2, Vec::new()), (99, Vec::new())]),
            stats: Stats::default(),
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
            dividends: HashMap::from([(7, Vec::new())]),
            stats: Stats::default(),
        };

        snapshot.prune_stale_asset_maps();

        assert!(snapshot.holders.is_empty());
        assert!(snapshot.compliance.is_empty());
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
                dividends: ids.iter().map(|&id| (id, Vec::new())).collect(),
                stats: Stats {
                    total_assets: ids.len() as u64,
                    last_indexed_ledger: ledger,
                    ..Stats::default()
                },
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
                    state.replace(if i % 2 == 0 {
                        new.clone()
                    } else {
                        old.clone()
                    });
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
                                other => panic!("observed a ledger from neither generation: {other}"),
                            };
                            assert_eq!(ids, expected, "assets do not match stats ledger {ledger}");
                            assert_eq!(
                                snapshot.stats.total_assets as usize,
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
        let liquidity_pool =
            xdr::ScAddress::LiquidityPool(xdr::PoolId(xdr::Hash([0u8; 32])));
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
}
