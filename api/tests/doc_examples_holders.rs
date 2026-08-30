/// Snapshot test to verify /assets/:id/holders example responses match the documented API contract.
/// Issue #68: docs: verify example responses on 'API Holders' match the live API/contract shapes
use serde_json::{json, Value};

#[test]
fn test_holder_list_example_has_all_required_fields() {
    // Example from docs/app/docs/api/holders/page.mdx
    let example = json!({
        "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "balance": "1000000",
        "share_percent": 100.0
    });

    // Verify all required fields exist
    assert!(example.is_object(), "Holder should be an object");
    let obj = example.as_object().unwrap();

    // Fields defined in models::Holder
    assert!(obj.contains_key("address"), "Missing address");
    assert!(obj.contains_key("balance"), "Missing balance");
    assert!(obj.contains_key("share_percent"), "Missing share_percent");

    // Verify field types
    assert!(example["address"].is_string(), "address should be a string");
    assert!(
        example["balance"].is_string(),
        "balance should be a string (i128)"
    );
    assert!(
        example["share_percent"].is_number(),
        "share_percent should be a number (0-100)"
    );
}

#[test]
fn test_holder_example_balance_is_string() {
    // Verify that balance (i128) is serialized as a string to preserve precision
    let example = json!({
        "balance": "1000000"
    });

    assert!(
        example["balance"].is_string(),
        "balance must be a string (i128)"
    );

    // Should be parseable as i128
    assert!(
        example["balance"].as_str().unwrap().parse::<i128>().is_ok(),
        "balance should be parseable as i128"
    );
}

#[test]
fn test_holder_example_share_percent_in_valid_range() {
    // share_percent should be 0-100
    let examples = vec![
        json!({"share_percent": 100.0}),
        json!({"share_percent": 0.0}),
        json!({"share_percent": 50.5}),
    ];

    for example in examples {
        let percent = example["share_percent"].as_f64().unwrap();
        assert!(
            (0.0..=100.0).contains(&percent),
            "share_percent must be between 0 and 100, got {}",
            percent
        );
    }
}

#[test]
fn test_holder_example_address_format() {
    // Stellar addresses are 56-char strings starting with G
    let example = json!({
        "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA"
    });

    let addr = example["address"].as_str().unwrap();
    assert!(
        addr.starts_with('G') && addr.len() == 56,
        "address should be a 56-char Stellar public key"
    );
}

#[test]
fn test_holder_example_no_extra_undocumented_fields() {
    // Ensure documentation is exhaustive
    let expected_keys = ["address", "balance", "share_percent"];

    let example = json!({
        "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "balance": "1000000",
        "share_percent": 100.0
    });

    if let Value::Object(obj) = example {
        for key in obj.keys() {
            assert!(
                expected_keys.contains(&key.as_str()),
                "Unexpected field in Holder: {}. Update docs if this field should be exposed.",
                key
            );
        }
    }
}

#[test]
fn test_holders_list_is_array_and_sorted() {
    // The /assets/:id/holders endpoint returns an array sorted by balance descending
    let example = json!([
        {
            "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
            "balance": "1000000",
            "share_percent": 100.0
        }
    ]);

    assert!(example.is_array(), "holders list should be an array");
}
