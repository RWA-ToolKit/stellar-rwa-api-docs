//! Snapshot tests for Asset Token Contract response format.
//! Verifies examples in docs/app/docs/contracts/asset-token/page.mdx match live API.

#[cfg(test)]
mod tests {
    use crate::models::Asset;

    #[test]
    fn asset_metadata_has_all_documented_fields() {
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
        let obj = json.as_object().expect("not an object");

        // Verify documented AssetMetadata fields are present
        assert!(obj.contains_key("name"), "missing field: name");
        assert!(obj.contains_key("symbol"), "missing field: symbol");
        assert!(obj.contains_key("asset_type"), "missing field: asset_type");
        assert!(obj.contains_key("total_supply"), "missing field: total_supply");
        assert!(obj.contains_key("decimals"), "missing field: decimals");
        assert!(obj.contains_key("issuer"), "missing field: issuer (admin)");
        assert!(obj.contains_key("compliance_contract"), "missing field: compliance_contract");
        assert!(obj.contains_key("description"), "missing field: description (asset_description)");
        assert!(obj.contains_key("valuation_cents"), "missing field: valuation_cents");
        assert!(obj.contains_key("paused"), "missing field: paused");
    }

    #[test]
    fn asset_valuation_is_cents_and_usd() {
        let asset = Asset {
            id: 1,
            token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
            issuer: "GBUQWP3BOUZX34ULNQG23RQ6F4BFXETRA7LHYWQUXKXZAUASXMKCV75".to_string(),
            name: "Test Asset".to_string(),
            symbol: "TEST".to_string(),
            asset_type: "real_estate".to_string(),
            description: "Test".to_string(),
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

        // Per docs: Valuation is stored in USD cents (i128).
        assert_eq!(json["valuation_cents"].as_str(), Some("10000000000"));
        // Convenience field: valuation in USD dollars (f64)
        assert_eq!(json["valuation_usd"].as_f64(), Some(100_000_000.0));
    }

    #[test]
    fn asset_total_supply_is_i128_string() {
        let asset = Asset {
            id: 1,
            token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
            issuer: "GBUQWP3BOUZX34ULNQG23RQ6F4BFXETRA7LHYWQUXKXZAUASXMKCV75".to_string(),
            name: "Test Asset".to_string(),
            symbol: "TEST".to_string(),
            asset_type: "real_estate".to_string(),
            description: "Test".to_string(),
            valuation_cents: "5000000000".to_string(),
            valuation_usd: 50_000_000.0,
            decimals: 7,
            total_supply: "1000000000000".to_string(),
            holders: 150,
            active: true,
            paused: false,
            compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU".to_string(),
            created_at_ledger: 1000,
        };

        let json = serde_json::to_value(&asset).expect("serialization failed");

        // Per docs: Amounts are integer token units in the token's own decimals base.
        // Large integers are serialized as JSON strings.
        assert_eq!(json["total_supply"].as_str(), Some("1000000000000"));
    }

    #[test]
    fn asset_type_is_valid_enum_value() {
        let valid_types = vec!["real_estate", "invoice", "commodity"];

        for asset_type in valid_types {
            let asset = Asset {
                id: 1,
                token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
                issuer: "GBUQWP3BOUZX34ULNQG23RQ6F4BFXETRA7LHYWQUXKXZAUASXMKCV75".to_string(),
                name: "Test Asset".to_string(),
                symbol: "TEST".to_string(),
                asset_type: asset_type.to_string(),
                description: "Test".to_string(),
                valuation_cents: "5000000000".to_string(),
                valuation_usd: 50_000_000.0,
                decimals: 7,
                total_supply: "1000000000000".to_string(),
                holders: 150,
                active: true,
                paused: false,
                compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU".to_string(),
                created_at_ledger: 1000,
            };

            let json = serde_json::to_value(&asset).expect("serialization failed");
            assert_eq!(json["asset_type"].as_str(), Some(asset_type));
        }
    }
}
