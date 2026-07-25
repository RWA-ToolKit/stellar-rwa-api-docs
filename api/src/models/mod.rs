//! Serializable domain models returned by the REST API.
//!
//! On-chain, monetary valuations are stored as USD cents (`i128`) and token
//! amounts as integers in each token's own `decimals` base. Large integers are
//! serialized as JSON strings to avoid precision loss in JavaScript consumers;
//! a friendly `*_usd` field (f64 dollars) is included for valuations.

use serde::Serialize;

/// A tokenized real-world asset, joined from the registry entry and the token
/// contract metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub id: u64,
    pub token_contract: String,
    pub issuer: String,
    pub name: String,
    pub symbol: String,
    pub asset_type: String,
    pub description: String,
    /// Valuation in USD cents (raw i128, as a string).
    pub valuation_cents: String,
    /// Valuation in USD dollars, for convenience.
    pub valuation_usd: f64,
    pub decimals: u32,
    /// Total supply in base units (raw i128, as a string).
    pub total_supply: String,
    /// Number of addresses currently holding a positive balance.
    pub holders: usize,
    pub active: bool,
    pub paused: bool,
    pub compliance_contract: String,
    pub created_at_ledger: u32,
}

/// A single holder of an asset token.
#[derive(Debug, Clone, Serialize)]
pub struct Holder {
    pub address: String,
    /// Balance in base units (raw i128, as a string).
    pub balance: String,
    /// Share of total supply, 0–100 with two decimals.
    pub share_percent: f64,
}

/// Aggregate, non-PII summary of an asset's compliance allowlist.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ComplianceSummary {
    /// Total addresses that have ever been added to the allowlist.
    pub total_records: usize,
    /// Addresses whose stored KYC status is `Approved`.
    ///
    /// This reflects the stored status field only, not the on-chain
    /// `is_allowed` gate — a record can count as `approved` here while
    /// failing `is_allowed` (e.g. an expired record, or one in a
    /// jurisdiction later blocked via `is_jurisdiction_blocked`). Treat
    /// this as "ever approved by compliance", not "currently permitted
    /// to transact".
    pub approved: usize,
    pub suspended: usize,
    pub rejected: usize,
    pub pending: usize,
    /// Count of records with an expiry set (non-zero `expires_at`).
    pub with_expiry: usize,
    /// Distribution of records across ISO jurisdiction codes.
    pub jurisdictions: Vec<JurisdictionCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JurisdictionCount {
    pub jurisdiction: String,
    pub count: usize,
}

/// A dividend distribution for an asset.
#[derive(Debug, Clone, Serialize)]
pub struct Distribution {
    pub id: u64,
    pub asset_token: String,
    pub payment_token: String,
    /// Total pool in payment-token base units (raw i128, as a string).
    pub total_amount: String,
    /// Amount claimed so far (raw i128, as a string).
    pub distributed: String,
    /// Percentage of the pool claimed, 0–100 with two decimals.
    pub claimed_percent: f64,
    pub completed: bool,
    pub snapshot_ledger: u32,
    pub created_at_ledger: u32,
}

/// Platform-wide statistics.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Stats {
    pub total_assets: usize,
    pub active_assets: usize,
    /// Total value locked across active assets, in USD cents (string).
    pub tvl_cents: String,
    pub tvl_usd: f64,
    /// Distinct addresses with a positive token balance across all assets.
    pub total_holders: usize,
    pub total_distributions: usize,
    /// The last ledger the indexer successfully read from.
    pub last_indexed_ledger: u32,
    /// RFC3339 timestamp of the last successful index refresh.
    pub last_updated: Option<String>,
}

/// Standard error body returned by the API.
#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub message: String,
}
