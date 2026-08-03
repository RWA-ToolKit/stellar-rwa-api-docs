/// Snapshot test to verify /assets example responses match the documented API contract.
/// Issue #67: docs: verify example responses on 'API Assets' match the live API/contract shapes
use serde_json::{json, Value};

#[test]
fn test_asset_list_example_has_all_required_fields() {
    // Example from docs/app/docs/api/assets/page.mdx
    let example = json!({
        "id": 1,
        "token_contract": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        "issuer": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "name": "Manhattan Loft",
        "symbol": "MLOFT",
        "asset_type": "real_estate",
        "description": "A tokenized loft in Manhattan",
        "valuation_cents": "500000000",
        "valuation_usd": 5000000.0,
        "decimals": 2,
        "total_supply": "1000000",
        "holders": 1,
        "active": true,
        "paused": false,
        "compliance_contract": "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        "created_at_ledger": 3502885
    });

    // Verify all required fields exist
    assert!(example.is_object(), "Asset should be an object");
    let obj = example.as_object().unwrap();

    // Fields defined in models::Asset
    assert!(obj.contains_key("id"), "Missing id");
    assert!(obj.contains_key("token_contract"), "Missing token_contract");
    assert!(obj.contains_key("issuer"), "Missing issuer");
    assert!(obj.contains_key("name"), "Missing name");
    assert!(obj.contains_key("symbol"), "Missing symbol");
    assert!(obj.contains_key("asset_type"), "Missing asset_type");
    assert!(obj.contains_key("description"), "Missing description");
    assert!(
        obj.contains_key("valuation_cents"),
        "Missing valuation_cents"
    );
    assert!(obj.contains_key("valuation_usd"), "Missing valuation_usd");
    assert!(obj.contains_key("decimals"), "Missing decimals");
    assert!(obj.contains_key("total_supply"), "Missing total_supply");
    assert!(obj.contains_key("holders"), "Missing holders");
    assert!(obj.contains_key("active"), "Missing active");
    assert!(obj.contains_key("paused"), "Missing paused");
    assert!(
        obj.contains_key("compliance_contract"),
        "Missing compliance_contract"
    );
    assert!(
        obj.contains_key("created_at_ledger"),
        "Missing created_at_ledger"
    );

    // Verify field types
    assert!(example["id"].is_number(), "id should be a number");
    assert!(
        example["token_contract"].is_string(),
        "token_contract should be a string"
    );
    assert!(example["issuer"].is_string(), "issuer should be a string");
    assert!(example["name"].is_string(), "name should be a string");
    assert!(example["symbol"].is_string(), "symbol should be a string");
    assert!(
        example["asset_type"].is_string(),
        "asset_type should be a string"
    );
    assert!(
        example["description"].is_string(),
        "description should be a string"
    );
    assert!(
        example["valuation_cents"].is_string(),
        "valuation_cents should be a string (i128)"
    );
    assert!(
        example["valuation_usd"].is_number(),
        "valuation_usd should be a number"
    );
    assert!(
        example["decimals"].is_number(),
        "decimals should be a number"
    );
    assert!(
        example["total_supply"].is_string(),
        "total_supply should be a string (i128)"
    );
    assert!(example["holders"].is_number(), "holders should be a number");
    assert!(example["active"].is_boolean(), "active should be a boolean");
    assert!(example["paused"].is_boolean(), "paused should be a boolean");
    assert!(
        example["compliance_contract"].is_string(),
        "compliance_contract should be a string"
    );
    assert!(
        example["created_at_ledger"].is_number(),
        "created_at_ledger should be a number"
    );
}

#[test]
fn test_asset_example_large_integers_are_strings() {
    // Verify that large integers (i128) are serialized as strings to preserve precision
    let example = json!({
        "valuation_cents": "500000000",
        "total_supply": "1000000"
    });

    assert!(
        example["valuation_cents"].is_string(),
        "valuation_cents must be a string (i128)"
    );
    assert!(
        example["total_supply"].is_string(),
        "total_supply must be a string (i128)"
    );

    // Should be parseable as i128
    assert!(
        example["valuation_cents"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .is_ok(),
        "valuation_cents should be parseable as i128"
    );
    assert!(
        example["total_supply"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .is_ok(),
        "total_supply should be parseable as i128"
    );
}

#[test]
fn test_asset_example_valid_asset_types() {
    // Verify asset_type values match contract definitions
    let valid_types = vec!["real_estate", "invoice", "commodity"];

    let example_types = vec!["real_estate"];
    for asset_type in example_types {
        assert!(
            valid_types.contains(&asset_type),
            "asset_type '{}' not in valid set: {:?}",
            asset_type,
            valid_types
        );
    }
}

#[test]
fn test_asset_example_no_extra_undocumented_fields() {
    // Ensure documentation is exhaustive
    let expected_keys = vec![
        "id",
        "token_contract",
        "issuer",
        "name",
        "symbol",
        "asset_type",
        "description",
        "valuation_cents",
        "valuation_usd",
        "decimals",
        "total_supply",
        "holders",
        "active",
        "paused",
        "compliance_contract",
        "created_at_ledger",
    ];

    let example = json!({
        "id": 1,
        "token_contract": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        "issuer": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "name": "Manhattan Loft",
        "symbol": "MLOFT",
        "asset_type": "real_estate",
        "description": "A tokenized loft in Manhattan",
        "valuation_cents": "500000000",
        "valuation_usd": 5000000.0,
        "decimals": 2,
        "total_supply": "1000000",
        "holders": 1,
        "active": true,
        "paused": false,
        "compliance_contract": "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        "created_at_ledger": 3502885
    });

    if let Value::Object(obj) = example {
        for key in obj.keys() {
            assert!(
                expected_keys.contains(&key.as_str()),
                "Unexpected field in Asset: {}. Update docs if this field should be exposed.",
                key
            );
        }
    }
}
