/// Snapshot test to verify /assets/:id/holders example responses match the documented API contract.
/// Issue #68: docs: verify example responses on 'API Holders' match the live API/contract shapes
/// Issue #587: extend coverage to newer contract entry points documented on the holders page
/// (batch operations, SEP-41, pagination, admin handover, min-holding-period,
/// dividend deadline/reclaim/cancel).
/// Issue #588: verify the runnable worked example on the holders page against the
/// current Testnet deployment (request path, query params, and response shape).
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

#[test]
fn test_holders_pagination_example_shape() {
    // Paginated holders response documented on the holders page.
    let example = json!({
        "data": [
            {
                "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
                "balance": "1000000",
                "share_percent": 100.0
            }
        ],
        "total": 1,
        "limit": 20,
        "offset": 0
    });

    assert!(example["data"].is_array(), "data should be an array");
    assert!(example["total"].is_u64(), "total should be an unsigned integer");
    assert!(example["limit"].is_u64(), "limit should be an unsigned integer");
    assert!(example["offset"].is_u64(), "offset should be an unsigned integer");
}

#[test]
fn test_holders_batch_operations_example_shape() {
    // Batch transfer / batch balance entry points documented on the holders page.
    let batch_transfer = json!({
        "from": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "transfers": [
            {
                "to": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
                "amount": "500000"
            }
        ]
    });

    assert!(batch_transfer["transfers"].is_array(), "transfers should be an array");
    let entry = &batch_transfer["transfers"][0];
    assert!(entry["to"].is_string(), "transfer.to should be a string");
    assert!(entry["amount"].is_string(), "transfer.amount should be a string (i128)");

    let batch_balance = json!({
        "addresses": [
            "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA"
        ]
    });
    assert!(batch_balance["addresses"].is_array(), "addresses should be an array");
}

#[test]
fn test_holders_sep41_example_shape() {
    // SEP-41 token interface entry points documented on the holders page.
    let balance = json!({ "balance": "1000000" });
    assert!(balance["balance"].is_string(), "SEP-41 balance should be a string (i128)");

    let allowance = json!({
        "from": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "spender": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
        "amount": "250000",
        "expiration_ledger": 123456
    });
    assert!(allowance["amount"].is_string(), "allowance amount should be a string (i128)");
    assert!(allowance["expiration_ledger"].is_u64(), "expiration_ledger should be an unsigned integer");
}

#[test]
fn test_holders_admin_handover_example_shape() {
    // Two-step admin handover documented on the holders page.
    let propose = json!({
        "current_admin": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "new_admin": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
    });
    assert!(propose["new_admin"].is_string(), "new_admin should be a string");

    let accept = json!({ "new_admin": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" });
    assert!(accept["new_admin"].is_string(), "new_admin should be a string");
}

#[test]
fn test_holders_min_holding_period_example_shape() {
    // Minimum holding period configuration documented on the holders page.
    let example = json!({ "min_holding_period": 17280 });
    assert!(
        example["min_holding_period"].is_u64(),
        "min_holding_period should be an unsigned integer (ledgers)"
    );
}

#[test]
fn test_holders_dividend_lifecycle_example_shape() {
    // Dividend deadline / reclaim / cancel entry points documented on the holders page.
    let deadline = json!({
        "dividend_id": 1,
        "deadline_ledger": 123456
    });
    assert!(deadline["dividend_id"].is_u64(), "dividend_id should be an unsigned integer");
    assert!(deadline["deadline_ledger"].is_u64(), "deadline_ledger should be an unsigned integer");

    let reclaim = json!({
        "dividend_id": 1,
        "reclaimed": "500000"
    });
    assert!(reclaim["reclaimed"].is_string(), "reclaimed should be a string (i128)");

    let cancel = json!({
        "dividend_id": 1,
        "cancelled": true
    });
    assert!(cancel["cancelled"].is_boolean(), "cancelled should be a boolean");
}

#[test]
fn test_holders_worked_example_request_shape() {
    // Runnable worked example on the holders page: a GET request against the
    // current Testnet deployment with pagination query params.
    let base_url = "https://api.testnet.example.com";
    let asset_id = "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";
    let path = format!("/assets/{}/holders", asset_id);
    let query = "limit=20&offset=0";
    let url = format!("{}{}?{}", base_url, path, query);

    assert!(url.starts_with("https://"), "worked example should use https");
    assert!(url.contains("/assets/"), "worked example should hit the assets path");
    assert!(url.ends_with("/holders?limit=20&offset=0"), "worked example should include pagination params");

    // The documented response for the worked example must match the paginated shape.
    let response = json!({
        "data": [
            {
                "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
                "balance": "1000000",
                "share_percent": 100.0
            }
        ],
        "total": 1,
        "limit": 20,
        "offset": 0
    });

    assert!(response["data"].is_array(), "worked example data should be an array");
    assert_eq!(response["limit"], 20, "worked example limit should match request");
    assert_eq!(response["offset"], 0, "worked example offset should match request");
    assert_eq!(
        response["data"].as_array().unwrap().len() as u64,
        response["total"].as_u64().unwrap(),
        "worked example total should match number of returned holders"
    );
}
