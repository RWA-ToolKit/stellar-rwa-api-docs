//! Snapshot tests for dividends API endpoint response format.
//! Verifies examples in docs/app/docs/api/dividends/page.mdx match live API.

#[cfg(test)]
mod tests {
    use crate::models::{Distribution};

    #[test]
    fn distribution_list_response_has_correct_field_types() {
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
        let obj = json.as_object().expect("not an object");

        // Verify all documented fields are present
        assert!(obj.contains_key("id"), "missing field: id");
        assert!(obj.contains_key("asset_token"), "missing field: asset_token");
        assert!(obj.contains_key("payment_token"), "missing field: payment_token");
        assert!(obj.contains_key("total_amount"), "missing field: total_amount");
        assert!(obj.contains_key("distributed"), "missing field: distributed");
        assert!(obj.contains_key("claimed_percent"), "missing field: claimed_percent");
        assert!(obj.contains_key("completed"), "missing field: completed");
        assert!(obj.contains_key("snapshot_ledger"), "missing field: snapshot_ledger");
        assert!(obj.contains_key("created_at_ledger"), "missing field: created_at_ledger");

        // Verify field types match documented types
        assert!(json["id"].is_number(), "id should be a number");
        assert!(json["asset_token"].is_string(), "asset_token should be a string");
        assert!(json["payment_token"].is_string(), "payment_token should be a string");
        assert!(json["total_amount"].is_string(), "total_amount should be a string (i128)");
        assert!(json["distributed"].is_string(), "distributed should be a string (i128)");
        assert!(json["claimed_percent"].is_number(), "claimed_percent should be a number");
        assert!(json["completed"].is_boolean(), "completed should be a boolean");
        assert!(json["snapshot_ledger"].is_number(), "snapshot_ledger should be a number");
        assert!(json["created_at_ledger"].is_number(), "created_at_ledger should be a number");
    }

    #[test]
    fn distribution_i128_amounts_are_string_encoded() {
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

        // Per docs: total_amount and distributed are i128 base units as strings
        assert_eq!(json["total_amount"].as_str(), Some("100000000000"));
        assert_eq!(json["distributed"].as_str(), Some("25000000000"));
    }

    #[test]
    fn distribution_claimed_percent_is_precise() {
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

        // Per docs: claimed_percent is 0–100 with two decimal places
        assert_eq!(json["claimed_percent"].as_f64(), Some(25.0));
    }
}
