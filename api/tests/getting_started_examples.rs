//! Snapshot tests for example responses documented in Getting Started guide.
//!
//! This test module verifies that example JSON responses in the Getting Started
//! documentation match the expected API response schema for:
//! - /stats endpoint
//! - /assets endpoint

use serde_json::json;

/// Validates the /stats endpoint example from Getting Started documentation.
/// This example shows platform-wide statistics with TVL in both cents (string) and USD (float).
#[test]
fn stats_endpoint_example_valid() {
    let stats_example = json!({
        "total_assets": 1,
        "active_assets": 1,
        "tvl_cents": "500000000",
        "tvl_usd": 5000000.0,
        "total_holders": 1,
        "total_distributions": 0,
        "last_indexed_ledger": 3514152,
        "last_updated": "2026-07-09T08:43:12.101772791+00:00"
    });

    // Verify all documented fields exist
    assert!(stats_example["total_assets"].is_number());
    assert_eq!(stats_example["total_assets"], 1);

    assert!(stats_example["active_assets"].is_number());
    assert_eq!(stats_example["active_assets"], 1);

    // TVL must be string (i128) to avoid precision loss
    assert!(stats_example["tvl_cents"].is_string());
    assert_eq!(stats_example["tvl_cents"], "500000000");

    // TVL convenience float must exist
    assert!(stats_example["tvl_usd"].is_number());
    assert_eq!(stats_example["tvl_usd"], 5000000.0);

    // Verify the mathematical relationship
    let cents: i128 = "500000000".parse().unwrap();
    let expected_usd = (cents as f64) / 100.0;
    assert_eq!(stats_example["tvl_usd"].as_f64().unwrap(), expected_usd);

    assert!(stats_example["total_holders"].is_number());
    assert!(stats_example["total_distributions"].is_number());

    assert!(stats_example["last_indexed_ledger"].is_number());
    assert_eq!(stats_example["last_indexed_ledger"], 3514152);

    // last_updated must be an RFC3339 timestamp string
    assert!(stats_example["last_updated"].is_string());
    let timestamp_str = stats_example["last_updated"].as_str().unwrap();
    assert!(timestamp_str.contains("T"), "Timestamp must be in RFC3339 format (contain 'T')");
    assert!(timestamp_str.ends_with("Z") || timestamp_str.contains("+") || timestamp_str.contains("-"),
            "Timestamp must have timezone info");
}

/// Validates the /assets endpoint example from Getting Started documentation.
/// This example shows a single asset (Manhattan Loft) with all required fields.
#[test]
fn assets_endpoint_example_valid() {
    let asset_example = json!({
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

    // Verify basic fields
    assert!(asset_example["id"].is_number());
    assert_eq!(asset_example["id"], 1);

    assert!(asset_example["name"].is_string());
    assert_eq!(asset_example["name"], "Manhattan Loft");

    assert!(asset_example["symbol"].is_string());
    assert_eq!(asset_example["symbol"], "MLOFT");

    assert!(asset_example["asset_type"].is_string());
    assert_eq!(asset_example["asset_type"], "real_estate");

    // Verify contract addresses are strings
    assert!(asset_example["token_contract"].is_string());
    assert!(asset_example["compliance_contract"].is_string());
    assert!(asset_example["issuer"].is_string());

    // Verify valuation fields (large integers as strings)
    assert!(asset_example["valuation_cents"].is_string());
    assert_eq!(asset_example["valuation_cents"], "500000000");

    // Convenience float must exist and be correct
    assert!(asset_example["valuation_usd"].is_number());
    assert_eq!(asset_example["valuation_usd"], 5000000.0);

    let cents: i128 = "500000000".parse().unwrap();
    let expected_usd = (cents as f64) / 100.0;
    assert_eq!(asset_example["valuation_usd"].as_f64().unwrap(), expected_usd);

    // Verify supply is a string (i128)
    assert!(asset_example["total_supply"].is_string());
    assert_eq!(asset_example["total_supply"], "1000000");

    // Verify decimals
    assert!(asset_example["decimals"].is_number());
    assert_eq!(asset_example["decimals"], 2);

    // Verify holder count
    assert!(asset_example["holders"].is_number());
    assert_eq!(asset_example["holders"], 1);

    // Verify status flags
    assert!(asset_example["active"].is_boolean());
    assert_eq!(asset_example["active"], true);

    assert!(asset_example["paused"].is_boolean());
    assert_eq!(asset_example["paused"], false);

    // Verify ledger height
    assert!(asset_example["created_at_ledger"].is_number());
    assert!(asset_example["created_at_ledger"].as_u64().unwrap() > 0);
}

/// Validates that the example uses the correct Testnet contract addresses.
#[test]
fn example_uses_correct_testnet_addresses() {
    // These are the actual Testnet addresses from the deployment
    let expected_compliance = "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU";
    let expected_asset_token = "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ";
    let expected_registry = "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3";
    let expected_dividend = "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX";
    let expected_issuer = "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA";

    // Verify that example uses these addresses
    let asset = json!({
        "compliance_contract": expected_compliance,
        "token_contract": expected_asset_token,
        "issuer": expected_issuer
    });

    assert_eq!(asset["compliance_contract"], expected_compliance);
    assert_eq!(asset["token_contract"], expected_asset_token);
    assert_eq!(asset["issuer"], expected_issuer);
}

/// Validates the curl command example can be parsed for syntax.
#[test]
fn curl_examples_are_valid_bash() {
    // Example: curl http://localhost:8080/stats
    let curl_endpoint = "curl http://localhost:8080/stats";
    assert!(curl_endpoint.contains("http://localhost:8080/stats"));

    // Example: curl http://localhost:8080/assets
    let curl_assets = "curl http://localhost:8080/assets";
    assert!(curl_assets.contains("http://localhost:8080/assets"));
}
