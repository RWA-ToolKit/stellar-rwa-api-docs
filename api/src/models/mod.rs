//! Serializable domain models returned by the REST API.
//!
//! On-chain, monetary valuations are stored as USD cents (`i128`) and token
//! amounts as integers in each token's own `decimals` base. Large integers are
//! serialized as JSON strings to avoid precision loss in JavaScript consumers;
//! a friendly `*_usd` field (f64 dollars) is included for valuations.

use serde::Serialize;

/// A tokenized real-world asset, joined from the registry entry and the token
/// contract metadata.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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
    /// Ledger number at which this asset was last successfully indexed.
    pub indexed_at_ledger: u32,
    /// Non-null when the most recent per-asset index attempt failed.  The
    /// global fields on [`Stats`] still reflect the last successful full
    /// refresh; this tells consumers which individual assets may be stale.
    pub index_error: Option<String>,
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
    /// Allowlisted addresses whose current record could not be read this
    /// cycle (the per-address `get_record` RPC call failed). These are
    /// counted in `total_records` but not in any of the status buckets
    /// above, so `approved + suspended + rejected + pending + unread` may
    /// be less than `total_records` when a record was read but had no
    /// recognized status.
    pub unread: usize,
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
    /// Percentage of the pool claimed, rounded to two decimals.
    ///
    /// Under normal conditions this is in the range 0–100. When
    /// `overflow_detected` is `true` the value will exceed 100, exposing the
    /// raw on-chain anomaly rather than silently clamping it.
    pub claimed_percent: f64,
    /// `true` when `distributed` exceeds `total_amount`, which can happen if
    /// the on-chain dividend contract allows double-claim vectors. Callers
    /// should treat this as a data-integrity warning.
    pub overflow_detected: bool,
    pub completed: bool,
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
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_serialization() {
        let error = ApiErrorBody {
            error: "invalid_request".to_string(),
            message: "Asset not found".to_string(),
        };

        let json = serde_json::to_value(&error).expect("serialization should succeed");

        assert_eq!(json["error"], "invalid_request");
        assert_eq!(json["message"], "Asset not found");
        assert!(json.is_object());
        assert_eq!(json.as_object().unwrap().len(), 2);
    }
}
