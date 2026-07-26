//! Serializable domain models returned by the REST API.
//!
//! On-chain, monetary valuations are stored as USD cents (`i128`) and token
//! amounts as integers in each token's own `decimals` base. Large integers are
//! serialized as JSON strings to avoid precision loss in JavaScript consumers;
//! a friendly `*_usd` field (f64 dollars) is included for valuations.

use serde::Serialize;

#[cfg(test)]
mod distribution_tests;

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
    /// Addresses currently passing the on-chain `is_allowed` gate.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compliance_summary_serializes_with_expected_fields() {
        let compliance = ComplianceSummary {
            total_records: 1,
            approved: 1,
            suspended: 0,
            rejected: 0,
            pending: 0,
            with_expiry: 0,
            jurisdictions: vec![JurisdictionCount {
                jurisdiction: "US".to_string(),
                count: 1,
            }],
        };

        let json = serde_json::to_value(&compliance).expect("serialization failed");

        assert_eq!(json["total_records"], 1);
        assert_eq!(json["approved"], 1);
        assert_eq!(json["suspended"], 0);
        assert_eq!(json["rejected"], 0);
        assert_eq!(json["pending"], 0);
        assert_eq!(json["with_expiry"], 0);
        assert_eq!(json["jurisdictions"][0]["jurisdiction"], "US");
        assert_eq!(json["jurisdictions"][0]["count"], 1);
    }

    #[test]
    fn distribution_serializes_with_expected_fields() {
        let distribution = Distribution {
            id: 1,
            asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
            payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            total_amount: "100000000000".to_string(),
            distributed: "25000000000".to_string(),
            claimed_percent: 25.0,
            completed: false,
            snapshot_ledger: 3510000,
            created_at_ledger: 3510000,
        };

        let json = serde_json::to_value(&distribution).expect("serialization failed");

        assert_eq!(json["id"], 1);
        assert_eq!(json["asset_token"], "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ");
        assert_eq!(json["payment_token"], "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC");
        assert_eq!(json["total_amount"], "100000000000");
        assert_eq!(json["distributed"], "25000000000");
        assert_eq!(json["claimed_percent"], 25.0);
        assert_eq!(json["completed"], false);
        assert_eq!(json["snapshot_ledger"], 3510000);
        assert_eq!(json["created_at_ledger"], 3510000);
    }

    #[test]
    fn asset_serializes_with_expected_fields() {
        let asset = Asset {
            id: 1,
            token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
            issuer: "GBUQWP3BOUZX34ULNQG23RQ6F4BFXETRA7LHYWQUXKXZAUASXMKCV75".to_string(),
            name: "Moonlight Portfolio Real Estate Fund".to_string(),
            symbol: "MLOFT".to_string(),
            asset_type: "real_estate".to_string(),
            description: "A diversified real estate fund".to_string(),
            valuation_cents: "10000000000".to_string(),
            valuation_usd: 100_000_000.0,
            decimals: 7,
            total_supply: "1000000000000".to_string(),
            holders: 150,
            active: true,
            paused: false,
            compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU".to_string(),
            created_at_ledger: 1000,
        };

        let json = serde_json::to_value(&asset).expect("serialization failed");

        assert_eq!(json["id"], 1);
        assert_eq!(json["name"], "Moonlight Portfolio Real Estate Fund");
        assert_eq!(json["symbol"], "MLOFT");
        assert_eq!(json["asset_type"], "real_estate");
        assert_eq!(json["valuation_cents"], "10000000000");
        assert_eq!(json["valuation_usd"], 100_000_000.0);
        assert_eq!(json["total_supply"], "1000000000000");
        assert_eq!(json["decimals"], 7);
        assert_eq!(json["holders"], 150);
        assert_eq!(json["active"], true);
        assert_eq!(json["paused"], false);
    }
}
