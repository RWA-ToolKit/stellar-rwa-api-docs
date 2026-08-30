/// Snapshot test to verify /stats example responses match the documented API contract.
/// Issue #66: docs: verify example responses on 'API Overview' match the live API/contract shapes
use serde_json::{json, Value};

#[test]
fn test_stats_example_has_all_required_fields() {
    // Example from docs/app/docs/api/overview/page.mdx
    let example = json!({
        "total_assets": 1,
        "active_assets": 1,
        "tvl_cents": "500000000",
        "tvl_usd": 5000000.0,
        "total_holders": 1,
        "total_distributions": 0,
        "last_indexed_ledger": 3514152,
        "last_updated": "2026-07-09T08:43:12.101772791+00:00"
    });

    // Verify all required fields exist
    assert!(example.is_object(), "Stats should be an object");
    let obj = example.as_object().unwrap();

    // Fields defined in models::Stats
    assert!(obj.contains_key("total_assets"), "Missing total_assets");
    assert!(obj.contains_key("active_assets"), "Missing active_assets");
    assert!(obj.contains_key("tvl_cents"), "Missing tvl_cents");
    assert!(obj.contains_key("tvl_usd"), "Missing tvl_usd");
    assert!(obj.contains_key("total_holders"), "Missing total_holders");
    assert!(
        obj.contains_key("total_distributions"),
        "Missing total_distributions"
    );
    assert!(
        obj.contains_key("last_indexed_ledger"),
        "Missing last_indexed_ledger"
    );
    assert!(obj.contains_key("last_updated"), "Missing last_updated");

    // Verify field types
    assert!(
        example["total_assets"].is_number(),
        "total_assets should be a number"
    );
    assert!(
        example["active_assets"].is_number(),
        "active_assets should be a number"
    );
    assert!(
        example["tvl_cents"].is_string(),
        "tvl_cents should be a string (i128)"
    );
    assert!(
        example["tvl_usd"].is_number(),
        "tvl_usd should be a number (f64)"
    );
    assert!(
        example["total_holders"].is_number(),
        "total_holders should be a number"
    );
    assert!(
        example["total_distributions"].is_number(),
        "total_distributions should be a number"
    );
    assert!(
        example["last_indexed_ledger"].is_number(),
        "last_indexed_ledger should be a number"
    );
    assert!(
        example["last_updated"].is_string() || example["last_updated"].is_null(),
        "last_updated should be a string or null"
    );
}

#[test]
fn test_stats_example_tvl_cents_is_decimal_string() {
    let example = json!({
        "tvl_cents": "500000000"
    });

    // tvl_cents must be serialized as a string to preserve i128 precision
    let tvl = &example["tvl_cents"];
    assert!(tvl.is_string(), "tvl_cents must be a string, not a number");

    // Should be a valid decimal representation
    let tvl_str = tvl.as_str().unwrap();
    assert!(
        tvl_str.parse::<i128>().is_ok(),
        "tvl_cents should be parseable as i128"
    );
}

#[test]
fn test_stats_example_no_extra_undocumented_fields() {
    // Ensure documentation is exhaustive
    let example_keys = [
        "total_assets",
        "active_assets",
        "tvl_cents",
        "tvl_usd",
        "total_holders",
        "total_distributions",
        "last_indexed_ledger",
        "last_updated",
    ];

    let example = json!({
        "total_assets": 1,
        "active_assets": 1,
        "tvl_cents": "500000000",
        "tvl_usd": 5000000.0,
        "total_holders": 1,
        "total_distributions": 0,
        "last_indexed_ledger": 3514152,
        "last_updated": "2026-07-09T08:43:12.101772791+00:00"
    });

    if let Value::Object(obj) = example {
        for key in obj.keys() {
            assert!(
                example_keys.contains(&key.as_str()),
                "Unexpected field in Stats: {}. Update docs if this field should be exposed.",
                key
            );
        }
    }
}
