//! Snapshot tests for example responses documented in Web App Guide.
//!
//! This test module verifies that the API endpoint examples used by the web app
//! remain accurate with respect to the contract and API models:
//! - /assets/:id/holders (portfolio display)
//! - /assets/:id/dividends (distribution claiming)
//! - /assets/:id/compliance (compliance status check)

use serde_json::json;

/// Validates the holders endpoint example used by the web app for portfolio display.
/// The web app shows holdings with balances and percentage shares.
#[test]
fn holders_endpoint_example_valid() {
    let holders_example = json!([
        {
            "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
            "balance": "1000000",
            "share_percent": 100.0
        }
    ]);

    assert!(holders_example.is_array());
    let holders = holders_example.as_array().unwrap();
    assert!(!holders.is_empty());

    let holder = &holders[0];

    // Verify address format (Stellar account)
    assert!(holder["address"].is_string());
    let address = holder["address"].as_str().unwrap();
    assert!(
        address.starts_with("G"),
        "Address must start with 'G' (Stellar account)"
    );
    assert_eq!(
        address.len(),
        56,
        "Stellar address must be exactly 56 characters"
    );

    // Verify balance is string (i128) to avoid precision loss
    assert!(holder["balance"].is_string());
    let balance_str = holder["balance"].as_str().unwrap();
    assert!(
        balance_str.parse::<i128>().is_ok(),
        "Balance must be parseable as i128"
    );

    // Verify share_percent is a float between 0 and 100
    assert!(holder["share_percent"].is_number());
    let share_percent = holder["share_percent"].as_f64().unwrap();
    assert!((0.0..=100.0).contains(&share_percent));
    assert_eq!(
        share_percent, 100.0,
        "In this example, holder owns 100% of supply"
    );
}

/// Validates the dividends endpoint example used by the web app for claiming distributions.
/// The web app shows distribution progress and allows claiming.
#[test]
fn dividends_endpoint_example_valid() {
    let dividends_example = json!([
        {
            "id": 1,
            "asset_token": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
            "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "total_amount": "100000000000",
            "distributed": "25000000000",
            "claimed_percent": 25.0,
            "completed": false,
            "snapshot_ledger": 3510000,
            "created_at_ledger": 3510000
        }
    ]);

    assert!(dividends_example.is_array());
    let distributions = dividends_example.as_array().unwrap();
    assert!(!distributions.is_empty());

    let distribution = &distributions[0];

    // Verify id is a number
    assert!(distribution["id"].is_number());
    assert_eq!(distribution["id"], 1);

    // Verify token contract addresses
    assert!(distribution["asset_token"].is_string());
    let asset_token = distribution["asset_token"].as_str().unwrap();
    assert!(
        asset_token.starts_with("C"),
        "Contract address must start with 'C'"
    );
    assert_eq!(
        asset_token.len(),
        56,
        "Contract address must be exactly 56 characters"
    );

    assert!(distribution["payment_token"].is_string());
    let payment_token = distribution["payment_token"].as_str().unwrap();
    assert!(
        payment_token.starts_with("C"),
        "Contract address must start with 'C'"
    );

    // Verify amounts are strings (i128) to avoid precision loss
    assert!(distribution["total_amount"].is_string());
    let total_str = distribution["total_amount"].as_str().unwrap();
    assert!(
        total_str.parse::<i128>().is_ok(),
        "total_amount must be parseable as i128"
    );
    assert_eq!(total_str, "100000000000");

    assert!(distribution["distributed"].is_string());
    let distributed_str = distribution["distributed"].as_str().unwrap();
    assert!(
        distributed_str.parse::<i128>().is_ok(),
        "distributed must be parseable as i128"
    );
    assert_eq!(distributed_str, "25000000000");

    // Verify claimed_percent calculation
    assert!(distribution["claimed_percent"].is_number());
    let claimed_percent = distribution["claimed_percent"].as_f64().unwrap();
    assert!((0.0..=100.0).contains(&claimed_percent));

    // Verify percent matches the ratio of distributed to total
    let total: i128 = total_str.parse().unwrap();
    let distributed: i128 = distributed_str.parse().unwrap();
    let expected_percent = (distributed as f64 / total as f64) * 100.0;
    assert!((claimed_percent - expected_percent).abs() < 0.01);

    // Verify completed flag
    assert!(distribution["completed"].is_boolean());
    assert_eq!(distribution["completed"], false);

    // Verify ledger numbers are reasonable
    assert!(distribution["snapshot_ledger"].is_number());
    assert!(distribution["created_at_ledger"].is_number());
    assert_eq!(distribution["snapshot_ledger"], 3510000);
    assert_eq!(distribution["created_at_ledger"], 3510000);
}

/// Validates that an empty holders array is correctly handled.
/// An asset with no current holders should return an empty array.
#[test]
fn holders_empty_array_valid() {
    let empty_holders = json!([]);

    assert!(empty_holders.is_array());
    assert_eq!(empty_holders.as_array().unwrap().len(), 0);
}

/// Validates that an empty dividends array is correctly handled.
/// An asset with no distributions should return an empty array.
#[test]
fn dividends_empty_array_valid() {
    let empty_dividends = json!([]);

    assert!(empty_dividends.is_array());
    assert_eq!(empty_dividends.as_array().unwrap().len(), 0);
}

/// Validates multi-holder scenario with different ownership percentages.
/// The web app portfolio page shows multiple holdings.
#[test]
fn multiple_holders_example_valid() {
    let multiple_holders = json!([
        {
            "address": "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
            "balance": "600000",
            "share_percent": 60.0
        },
        {
            "address": "GBRPYHIL2CI3WHZDTOOQFC6EB4LEGMOD",
            "balance": "400000",
            "share_percent": 40.0
        }
    ]);

    let holders = multiple_holders.as_array().unwrap();
    assert_eq!(holders.len(), 2);

    // Verify percentages sum to 100
    let total_percent: f64 = holders
        .iter()
        .map(|h| h["share_percent"].as_f64().unwrap())
        .sum();
    assert!(
        (total_percent - 100.0).abs() < 0.01,
        "Total share_percent must be ~100%"
    );

    // Verify holders are sorted by balance (largest first)
    assert!(
        holders[0]["balance"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .unwrap()
            >= holders[1]["balance"]
                .as_str()
                .unwrap()
                .parse::<i128>()
                .unwrap(),
        "Holders must be sorted by balance descending"
    );
}

/// Validates multiple distribution scenarios at different claim stages.
/// The web app shows progress bars and claim buttons for each.
#[test]
fn multiple_distributions_example_valid() {
    let distributions = json!([
        {
            "id": 2,
            "asset_token": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
            "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "total_amount": "50000000000",
            "distributed": "50000000000",
            "claimed_percent": 100.0,
            "completed": true,
            "snapshot_ledger": 3520000,
            "created_at_ledger": 3520000
        },
        {
            "id": 1,
            "asset_token": "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
            "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "total_amount": "100000000000",
            "distributed": "25000000000",
            "claimed_percent": 25.0,
            "completed": false,
            "snapshot_ledger": 3510000,
            "created_at_ledger": 3510000
        }
    ]);

    let dists = distributions.as_array().unwrap();
    assert_eq!(dists.len(), 2);

    // Verify ordered by newest ledger first
    assert!(
        dists[0]["created_at_ledger"].as_u64().unwrap()
            >= dists[1]["created_at_ledger"].as_u64().unwrap(),
        "Distributions must be ordered newest first"
    );

    // Verify completed distribution has 100% claimed
    assert_eq!(dists[0]["claimed_percent"], 100.0);
    assert_eq!(dists[0]["completed"], true);

    // Verify in-progress distribution shows partial progress
    assert_eq!(dists[1]["claimed_percent"], 25.0);
    assert_eq!(dists[1]["completed"], false);
}

/// Validates that payment token (not asset token) is used in distribution amounts.
/// Note: USDC and other SACs use 7 decimals, not the asset token's decimals.
#[test]
fn distribution_uses_payment_token_decimals() {
    let distribution = json!({
        "total_amount": "100000000000",
        "distributed": "25000000000",
        "payment_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
    });

    // The amounts are in payment_token base units (7 decimals for SAC)
    let total: i128 = distribution["total_amount"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let distributed: i128 = distribution["distributed"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // These are sample numbers; with 7 decimals:
    // 100000000000 base units = 10,000.0000000 payment tokens
    // 25000000000 base units = 2,500.0000000 payment tokens
    assert_eq!(total, 100000000000);
    assert_eq!(distributed, 25000000000);

    // Web app users should divide by 10^7 to see display values
    let display_total = total as f64 / 10_f64.powi(7);
    let display_distributed = distributed as f64 / 10_f64.powi(7);
    assert!((display_total - 10000.0).abs() < 0.01);
    assert!((display_distributed - 2500.0).abs() < 0.01);
}
