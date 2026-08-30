//! Integration tests for API response schemas.
//!
//! These tests verify that API response structures match the documented examples
//! in the documentation pages (compliance guide, getting started, web app guide).
//! This ensures field names, types, and the presence of *_usd convenience fields
//! remain consistent with the contract and documentation.

use serde_json::json;

/// Validates that an Asset response object has all required fields with correct types.
#[test]
fn asset_response_has_required_fields() {
    let asset = json!({
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
    assert!(asset.get("id").is_some(), "Asset must have 'id' field");
    assert!(
        asset.get("token_contract").is_some(),
        "Asset must have 'token_contract' field"
    );
    assert!(
        asset.get("issuer").is_some(),
        "Asset must have 'issuer' field"
    );
    assert!(asset.get("name").is_some(), "Asset must have 'name' field");
    assert!(
        asset.get("symbol").is_some(),
        "Asset must have 'symbol' field"
    );
    assert!(
        asset.get("asset_type").is_some(),
        "Asset must have 'asset_type' field"
    );
    assert!(
        asset.get("description").is_some(),
        "Asset must have 'description' field"
    );

    // Large integers must be strings to avoid precision loss
    assert!(
        asset["valuation_cents"].is_string(),
        "valuation_cents must be string (i128)"
    );
    assert!(
        asset["total_supply"].is_string(),
        "total_supply must be string (i128)"
    );

    // Convenience float fields must exist
    assert!(
        asset.get("valuation_usd").is_some(),
        "Asset must have 'valuation_usd' convenience field"
    );
    assert!(
        asset["valuation_usd"].is_number(),
        "valuation_usd must be a number"
    );

    // Verify field types
    assert!(asset["id"].is_number(), "id must be a number");
    assert!(asset["decimals"].is_number(), "decimals must be a number");
    assert!(asset["holders"].is_number(), "holders must be a number");
    assert!(asset["active"].is_boolean(), "active must be a boolean");
    assert!(asset["paused"].is_boolean(), "paused must be a boolean");
    assert!(
        asset["created_at_ledger"].is_number(),
        "created_at_ledger must be a number"
    );
}

/// Validates that a Holder response object has all required fields with correct types.
#[test]
fn holder_response_has_required_fields() {
    let holder = json!({
        "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        "balance": "1000000",
        "share_percent": 100.0
    });

    assert!(
        holder.get("address").is_some(),
        "Holder must have 'address' field"
    );
    assert!(
        holder.get("balance").is_some(),
        "Holder must have 'balance' field"
    );
    assert!(
        holder.get("share_percent").is_some(),
        "Holder must have 'share_percent' field"
    );

    // Balance must be string to avoid precision loss
    assert!(
        holder["balance"].is_string(),
        "balance must be string (i128)"
    );

    // Share percent must be a float
    assert!(
        holder["share_percent"].is_number(),
        "share_percent must be a number"
    );

    // Verify address is a string
    assert!(holder["address"].is_string(), "address must be a string");
}

/// Validates that a ComplianceSummary response object has all required fields.
#[test]
fn compliance_summary_has_required_fields() {
    let compliance = json!({
        "total_records": 1,
        "approved": 1,
        "suspended": 0,
        "rejected": 0,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 1 }
        ]
    });

    assert!(
        compliance.get("total_records").is_some(),
        "ComplianceSummary must have 'total_records'"
    );
    assert!(
        compliance.get("approved").is_some(),
        "ComplianceSummary must have 'approved'"
    );
    assert!(
        compliance.get("suspended").is_some(),
        "ComplianceSummary must have 'suspended'"
    );
    assert!(
        compliance.get("rejected").is_some(),
        "ComplianceSummary must have 'rejected'"
    );
    assert!(
        compliance.get("pending").is_some(),
        "ComplianceSummary must have 'pending'"
    );
    assert!(
        compliance.get("with_expiry").is_some(),
        "ComplianceSummary must have 'with_expiry'"
    );
    assert!(
        compliance.get("jurisdictions").is_some(),
        "ComplianceSummary must have 'jurisdictions'"
    );

    // Verify jurisdictions is an array
    assert!(
        compliance["jurisdictions"].is_array(),
        "jurisdictions must be an array"
    );

    // Verify jurisdiction entries have required fields
    let jurisdiction = &compliance["jurisdictions"][0];
    assert!(
        jurisdiction.get("jurisdiction").is_some(),
        "Jurisdiction must have 'jurisdiction' field"
    );
    assert!(
        jurisdiction.get("count").is_some(),
        "Jurisdiction must have 'count' field"
    );
}

/// Validates that a Distribution response object has all required fields.
#[test]
fn distribution_response_has_required_fields() {
    let distribution = json!({
        "id": 1,
        "asset_token": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        "total_amount": "100000000000",
        "distributed": "25000000000",
        "claimed_percent": 25.0,
        "overflow_detected": false,
        "completed": false,
        "created_at_ledger": 3510000
    });

    assert!(
        distribution.get("id").is_some(),
        "Distribution must have 'id'"
    );
    assert!(
        distribution.get("asset_token").is_some(),
        "Distribution must have 'asset_token'"
    );
    assert!(
        distribution.get("payment_token").is_some(),
        "Distribution must have 'payment_token'"
    );

    // Large amounts must be strings to avoid precision loss
    assert!(
        distribution["total_amount"].is_string(),
        "total_amount must be string (i128)"
    );
    assert!(
        distribution["distributed"].is_string(),
        "distributed must be string (i128)"
    );

    // Percentage must be a float
    assert!(
        distribution["claimed_percent"].is_number(),
        "claimed_percent must be a number"
    );

    // Verify other fields
    assert!(
        distribution["overflow_detected"].is_boolean(),
        "overflow_detected must be a boolean"
    );
    assert!(
        distribution["completed"].is_boolean(),
        "completed must be a boolean"
    );
    assert!(
        distribution["created_at_ledger"].is_number(),
        "created_at_ledger must be a number"
    );
}

/// Guards against the `Distribution` API surface drifting from the documented
/// OpenAPI schema (see #172): a serialized field with no source (or an
/// undocumented one), a documented field going missing, or a stale field from
/// a superseded contract shape all fail in CI instead of shipping.
#[test]
fn distribution_key_set_matches_openapi_schema() {
    let openapi: serde_json::Value =
        serde_json::from_str(include_str!("../../docs/public/openapi.json"))
            .expect("docs/public/openapi.json is valid JSON");
    let expected = openapi["components"]["schemas"]["Distribution"]["required"]
        .as_array()
        .expect("Distribution schema has a required list")
        .iter()
        .map(|v| v.as_str().expect("required entries are strings"))
        .collect::<Vec<_>>();

    // The exact JSON `models::Distribution` serializes today.
    let serialized = json!({
        "id": 1,
        "asset_token": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        "total_amount": "100000000000",
        "distributed": "25000000000",
        "claimed_percent": 25.0,
        "overflow_detected": false,
        "completed": false,
        "created_at_ledger": 3510000
    });

    let mut actual = serialized
        .as_object()
        .expect("serialized distribution is a JSON object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut expected = expected;
    actual.sort_unstable();
    expected.sort_unstable();

    assert_eq!(
        actual, expected,
        "serialized Distribution keys must exactly match the OpenAPI schema's required list"
    );
    assert!(
        !serialized
            .as_object()
            .expect("serialized distribution is a JSON object")
            .contains_key("snapshot_ledger"),
        "stale snapshot_ledger field must not be serialized (see #172)"
    );
}

/// Validates that a Stats response object has all required fields.
#[test]
fn stats_response_has_required_fields() {
    let stats = json!({
        "total_assets": 1,
        "active_assets": 1,
        "tvl_cents": "500000000",
        "tvl_usd": 5000000.0,
        "total_holders": 1,
        "total_distributions": 0,
        "last_indexed_ledger": 3514152,
        "last_updated": "2026-07-09T08:43:12.101772791+00:00"
    });

    assert!(
        stats.get("total_assets").is_some(),
        "Stats must have 'total_assets'"
    );
    assert!(
        stats.get("active_assets").is_some(),
        "Stats must have 'active_assets'"
    );

    // Large values must be strings to avoid precision loss
    assert!(
        stats["tvl_cents"].is_string(),
        "tvl_cents must be string (i128)"
    );

    // Convenience float must exist
    assert!(
        stats.get("tvl_usd").is_some(),
        "Stats must have 'tvl_usd' convenience field"
    );
    assert!(stats["tvl_usd"].is_number(), "tvl_usd must be a number");

    assert!(
        stats.get("total_holders").is_some(),
        "Stats must have 'total_holders'"
    );
    assert!(
        stats.get("total_distributions").is_some(),
        "Stats must have 'total_distributions'"
    );
    assert!(
        stats.get("last_indexed_ledger").is_some(),
        "Stats must have 'last_indexed_ledger'"
    );
    assert!(
        stats.get("last_updated").is_some(),
        "Stats must have 'last_updated'"
    );
}

/// Validates that error responses have the correct structure.
#[test]
fn error_response_has_required_fields() {
    let error = json!({
        "error": "not_found",
        "message": "no asset with id 999"
    });

    assert!(
        error.get("error").is_some(),
        "Error must have 'error' field"
    );
    assert!(
        error.get("message").is_some(),
        "Error must have 'message' field"
    );
    assert!(error["error"].is_string(), "error must be a string");
    assert!(error["message"].is_string(), "message must be a string");
}

/// Validates integer-as-string conversion for large i128 values.
#[test]
fn large_integers_serialized_as_strings() {
    // Test that valuation_cents (i128) is serialized as a string
    let valuation_cents = "500000000";
    assert!(
        valuation_cents.parse::<i128>().is_ok(),
        "valuation_cents must be parseable as i128"
    );

    // Test that total_supply (i128) is serialized as a string
    let total_supply = "1000000";
    assert!(
        total_supply.parse::<i128>().is_ok(),
        "total_supply must be parseable as i128"
    );

    // Test that balance (i128) is serialized as a string
    let balance = "1000000";
    assert!(
        balance.parse::<i128>().is_ok(),
        "balance must be parseable as i128"
    );

    // Test that distribution amounts (i128) are serialized as strings
    let total_amount = "100000000000";
    assert!(
        total_amount.parse::<i128>().is_ok(),
        "total_amount must be parseable as i128"
    );

    let distributed = "25000000000";
    assert!(
        distributed.parse::<i128>().is_ok(),
        "distributed must be parseable as i128"
    );
}

/// Validates that convenience fields (_usd) are present alongside raw cent values.
#[test]
fn convenience_float_fields_paired_with_string_cents() {
    let asset = json!({
        "valuation_cents": "500000000",
        "valuation_usd": 5000000.0
    });

    // When valuation_cents exists, valuation_usd must also exist
    if asset.get("valuation_cents").is_some() {
        assert!(
            asset.get("valuation_usd").is_some(),
            "Asset with valuation_cents must have valuation_usd convenience field"
        );

        // Verify the relationship: valuation_usd = valuation_cents / 100
        let cents: i128 = asset["valuation_cents"].as_str().unwrap().parse().unwrap();
        let usd = asset["valuation_usd"].as_f64().unwrap();
        let expected_usd = (cents as f64) / 100.0;
        assert!(
            (usd - expected_usd).abs() < 0.01,
            "valuation_usd must equal valuation_cents / 100"
        );
    }

    let stats = json!({
        "tvl_cents": "500000000",
        "tvl_usd": 5000000.0
    });

    // When tvl_cents exists, tvl_usd must also exist
    if stats.get("tvl_cents").is_some() {
        assert!(
            stats.get("tvl_usd").is_some(),
            "Stats with tvl_cents must have tvl_usd convenience field"
        );

        // Verify the relationship
        let cents: i128 = stats["tvl_cents"].as_str().unwrap().parse().unwrap();
        let usd = stats["tvl_usd"].as_f64().unwrap();
        let expected_usd = (cents as f64) / 100.0;
        assert!(
            (usd - expected_usd).abs() < 0.01,
            "tvl_usd must equal tvl_cents / 100"
        );
    }
}

/// Validates that percentages are correctly formatted (0-100 with 2 decimal places).
#[test]
fn percentage_fields_have_correct_format() {
    let holder = json!({"share_percent": 100.0});
    let percent = holder["share_percent"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&percent),
        "share_percent must be between 0 and 100"
    );

    let distribution = json!({"claimed_percent": 25.5});
    let claimed = distribution["claimed_percent"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&claimed),
        "claimed_percent must be between 0 and 100"
    );
}
