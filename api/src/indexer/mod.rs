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
use std::time::Duration;

use arc_swap::ArcSwap;
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

/// Maximum number of allowlist addresses processed per asset per refresh
/// cycle.  Each address costs 2 RPC round-trips (`get_record` + `balance`),
/// so this bounds the per-cycle RPC budget to `2 × COMPLIANCE_PAGE_SIZE` per
/// asset regardless of allowlist size.  The indexer remembers its position
/// and continues from where it left off on the next cycle, so the full
/// allowlist is always covered — just spread across multiple cycles.
/// Override with the `RWA_COMPLIANCE_PAGE_SIZE` environment variable.
const DEFAULT_COMPLIANCE_PAGE_SIZE: usize = 50;

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
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let rpc_url = env_or("RWA_RPC_URL", "https://soroban-testnet.stellar.org");
        url::Url::parse(&rpc_url).map_err(|e| ConfigError::RpcUrl(format!("{e}: {rpc_url}")))?;

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
}

/// Shared, hot-swappable state handed to the Axum routes.
#[derive(Clone)]
pub struct AppState {
    inner: ArcSwap<Snapshot>,
    pub config: Arc<Config>,
    pub metrics: PrometheusHandle,
    /// Per-asset offset into the allowlist for the next compliance page.
    /// Keyed by asset id; absent ≡ 0 (start from the beginning).
    /// Wrapped in `Arc<Mutex<…>>` so the `Clone`-able `AppState` shares one
    /// mutable map across all clones (routes + indexer).
    compliance_offsets: Arc<Mutex<HashMap<u64, usize>>>,
    /// How many allowlist addresses to process per asset per refresh cycle.
    pub compliance_page_size: usize,
}

impl AppState {
    pub fn new(config: Config, metrics: PrometheusHandle) -> Self {
        let compliance_page_size = std::env::var("RWA_COMPLIANCE_PAGE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_COMPLIANCE_PAGE_SIZE)
            .max(1); // must be at least 1
        AppState {
            inner: ArcSwap::from_arc(Arc::new(Snapshot::default())),
            config: Arc::new(config),
            metrics,
            compliance_offsets: Arc::new(Mutex::new(HashMap::new())),
            compliance_page_size,
        }
    }

    /// Clone the current snapshot for read-only serving.
    pub fn snapshot(&self) -> Snapshot {
        let guard = self.inner.load();
        (*guard).clone()
    }

    fn replace(&self, next: Snapshot) {
        self.inner.store(Arc::new(next));
    }

    /// Return the current allowlist offset for `asset_id` and advance it by
    /// `page_size`, wrapping back to 0 when it reaches or exceeds
    /// `total_len`.  Returns `(offset, wrapped)` where `wrapped` is true when
    /// the offset cycled back to 0 this call (i.e. we just finished the last
    /// page and are starting fresh next cycle).
    fn next_compliance_offset(
        &self,
        asset_id: u64,
        page_size: usize,
        total_len: usize,
    ) -> (usize, bool) {
        let mut map = self.compliance_offsets.lock().unwrap();
        let offset = *map.get(&asset_id).unwrap_or(&0);
        let next = offset + page_size;
        let wrapped = next >= total_len;
        map.insert(asset_id, if wrapped { 0 } else { next });
        (offset, wrapped)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("rpc request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("rpc returned an error: {0}")]
    Rpc(String),
    #[error("xdr error: {0}")]
    Xdr(String),
    #[error("strkey error: {0}")]
    Strkey(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("rate limited or unavailable (status {status}); retry after {retry_after:?}")]
    RateLimited { status: u16, retry_after: Option<Duration>, body: String },
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

impl From<xdr::Error> for IndexError {
    fn from(e: xdr::Error) -> Self {
        IndexError::Xdr(e.to_string())
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

        let resp = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await?;

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
    let c = stellar_strkey::Contract::from_string(strkey)
        .map_err(|e| IndexError::Strkey(e.to_string()))?;
    Ok(xdr::ContractId(xdr::Hash(c.0)))
}

fn account_muxed(strkey: &str) -> Result<xdr::MuxedAccount, IndexError> {
    let pk = stellar_strkey::ed25519::PublicKey::from_string(strkey)
        .map_err(|e| IndexError::Strkey(e.to_string()))?;
    Ok(xdr::MuxedAccount::Ed25519(xdr::Uint256(pk.0)))
}

/// An `ScVal::Address` from a G… or C… strkey.
fn address_scval(strkey: &str) -> Result<xdr::ScVal, IndexError> {
    if strkey.starts_with('C') {
        Ok(xdr::ScVal::Address(xdr::ScAddress::Contract(contract_id(
            strkey,
        )?)))
    } else {
        let pk = stellar_strkey::ed25519::PublicKey::from_string(strkey)
            .map_err(|e| IndexError::Strkey(e.to_string()))?;
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
    let function_name = xdr::ScSymbol(
        method
            .try_into()
            .map_err(|_| IndexError::Xdr(format!("invalid symbol: {method}")))?,
    );
    let invoke = xdr::InvokeContractArgs {
        contract_address: xdr::ScAddress::Contract(contract_id(contract)?),
        function_name,
        args: args
            .try_into()
            .map_err(|_| IndexError::Xdr("too many args".into()))?,
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
        fee: 100,
        seq_num: xdr::SequenceNumber(0),
        cond: xdr::Preconditions::None,
        memo: xdr::Memo::None,
        operations: vec![op]
            .try_into()
            .map_err(|_| IndexError::Xdr("operation build failed".into()))?,
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
        other => Err(IndexError::Decode(format!(
            "unsupported address: {other:?}"
        ))),
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
    snapshot_ledger: u32,
    created_at: u32,
    completed: bool,
}

fn parse_i128(s: &str) -> i128 {
    s.parse::<i128>().unwrap_or(0)
}

fn cents_to_usd(cents: i128) -> f64 {
    cents as f64 / 100.0
}

fn ratio_percent(part: i128, whole: i128) -> f64 {
    if whole <= 0 {
        return 0.0;
    }
    let pct = (part as f64 / whole as f64) * 100.0;
    (pct.clamp(0.0, 100.0) * 100.0).round() / 100.0
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
}

impl Indexer {
    pub fn new(state: AppState) -> Self {
        let cfg = &state.config;
        Indexer {
            rpc: Rpc::new(cfg.rpc_url.clone(), cfg.read_source.clone()),
            state,
        }
    }

    /// Poll forever, refreshing the snapshot every [`POLL_INTERVAL`].
    pub async fn run(self) {
        let mut consecutive_failures: u64 = 0;
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

        let entries_read = self
            .rpc
            .read(&cfg.registry_id, "get_all_assets", vec![])
            .await?;
        let latest_ledger = entries_read.latest_ledger;
        let raw_entries: Vec<RawAssetEntry> = serde_json::from_value(entries_read.value)
            .map_err(|e| IndexError::Decode(e.to_string()))?;

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
                            total_distributions += prev
                                .dividends
                                .get(&raw.id)
                                .map(|d| d.len())
                                .unwrap_or(0);
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
                    let err = IndexError::Decode(e.to_string());
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

            // ── Paginated compliance + holder indexing ────────────────────────
            // Fetch the full allowlist once, then process only one page of
            // addresses this cycle.  The offset advances automatically so
            // subsequent cycles cover the rest of the list.  When the offset
            // wraps back to 0 (new full scan) we discard stale accumulated
            // data; otherwise we merge into the previous summary/holders.
            let (holders, summary, compliance_err) = 'compliance: {
                let allowlist_read = match self
                    .rpc
                    .read(&meta.compliance_contract, "get_allowlist", vec![])
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        record_asset_read_error(raw.id, "compliance");
                        tracing::warn!(
                            asset_id = raw.id,
                            error = %e,
                            "get_allowlist failed; using previous data"
                        );
                        let holders = prev.holders.get(&raw.id).cloned().unwrap_or_default();
                        let summary = prev.compliance.get(&raw.id).cloned().unwrap_or_default();
                        break 'compliance (holders, summary, Some(e.to_string()));
                    }
                };
                let addresses: Vec<String> = match serde_json::from_value(allowlist_read.value) {
                    Ok(a) => a,
                    Err(e) => {
                        record_asset_read_error(raw.id, "compliance");
                        let err = IndexError::Decode(e.to_string());
                        tracing::warn!(
                            asset_id = raw.id,
                            error = %err,
                            "allowlist decode failed; using previous data"
                        );
                        let holders = prev.holders.get(&raw.id).cloned().unwrap_or_default();
                        let summary = prev.compliance.get(&raw.id).cloned().unwrap_or_default();
                        break 'compliance (holders, summary, Some(err.to_string()));
                    }
                };

                let page_size = self.state.compliance_page_size;
                let (offset, wrapped) = self.state.next_compliance_offset(
                    raw.id,
                    page_size,
                    addresses.len().max(1),
                );
                let end = (offset + page_size).min(addresses.len());
                let page = &addresses[offset..end];

                // On a wrap we start a fresh scan — discard stale counts.
                // Mid-scan we carry forward the accumulated summary/holders.
                let (prev_summary, prev_holders): (Option<&ComplianceSummary>, &[Holder]) =
                    if wrapped {
                        (None, &[])
                    } else {
                        (
                            prev.compliance.get(&raw.id),
                            prev.holders
                                .get(&raw.id)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                        )
                    };

                tracing::debug!(
                    asset_id = raw.id,
                    offset,
                    page_len = page.len(),
                    total = addresses.len(),
                    wrapped,
                    "compliance page"
                );

                match self
                    .index_compliance_and_holders(
                        &meta.compliance_contract,
                        &raw.token_contract,
                        total_supply,
                        page,
                        prev_summary,
                        prev_holders,
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
                }
            };

            // Dividends for this asset token; treated as empty on failure so one
            // asset's dividend contract issue doesn't abort the whole refresh.
            let dists = match self.index_dividends(&raw.token_contract).await {
                Ok(dists) => dists,
                Err(e) => {
                    record_asset_read_error(raw.id, "dividends");
                    tracing::warn!(asset_id = raw.id, error = %e, "dividends read failed; treating as empty");
                    prev.dividends.get(&raw.id).cloned().unwrap_or_default()
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
                index_error: compliance_err,
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
        self.state
            .replace(Snapshot {
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
    ///
    /// Only the addresses in `page` are queried this cycle — the caller is
    /// responsible for slicing the full allowlist down to at most
    /// [`AppState::compliance_page_size`] entries before calling this.
    ///
    /// `prev_summary` is the accumulated summary from prior pages in the
    /// current scan.  Pass `None` (or a default) when starting a fresh scan
    /// from offset 0 so stale counts are not carried forward.
    ///
    /// Returns `(holders_this_page, merged_summary, approved_addresses)`.
    /// `holders_this_page` contains only the holders found in `page`; the
    /// caller merges these with holders from previous pages.
    async fn index_compliance_and_holders(
        &self,
        compliance_contract: &str,
        token_contract: &str,
        total_supply: i128,
        page: &[String],
        prev_summary: Option<&ComplianceSummary>,
        prev_holders: &[Holder],
    ) -> Result<(Vec<Holder>, ComplianceSummary, Vec<String>), IndexError> {
        // Start from the accumulated summary (mid-scan carry-forward) or a
        // fresh default (start of a new full scan).
        let mut summary = prev_summary.cloned().unwrap_or_default();
        // Carry forward holders from previous pages in this scan.
        let mut holders: Vec<Holder> = prev_holders.to_vec();
        let mut jurisdictions: BTreeMap<String, usize> = BTreeMap::new();
        // Re-populate jurisdiction map from the existing summary counts so we
        // can merge new entries into it correctly.
        for jc in &summary.jurisdictions {
            jurisdictions.insert(jc.jurisdiction.clone(), jc.count);
        }
        let mut approved_addresses = Vec::new();

        for address in page {
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

    /// Read all distributions for an asset token from the dividend contract.
    async fn index_dividends(&self, token_contract: &str) -> Result<Vec<Distribution>, IndexError> {
        let read = self
            .rpc
            .read(
                &self.state.config.dividend_id,
                "get_distributions_for_asset",
                vec![address_scval(token_contract)?],
            )
            .await?;
        let raw: Vec<RawDistribution> =
            serde_json::from_value(read.value).map_err(|e| IndexError::Decode(e.to_string()))?;
        Ok(raw
            .into_iter()
            .map(|d| {
                let total = parse_i128(&d.total_amount);
                let distributed = parse_i128(&d.distributed);
                Distribution {
                    id: d.id,
                    asset_token: d.asset_token,
                    payment_token: d.payment_token,
                    total_amount: total.to_string(),
                    distributed: distributed.to_string(),
                    claimed_percent: ratio_percent(distributed, total),
                    completed: d.completed,
                    snapshot_ledger: d.snapshot_ledger,
                    created_at_ledger: d.created_at,
                }
            })
            .collect())
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
        assert!(!IndexError::Decode("bad json".into()).is_transient());
        assert!(!IndexError::Xdr("bad xdr".into()).is_transient());
        assert!(!IndexError::Strkey("bad key".into()).is_transient());
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
}
