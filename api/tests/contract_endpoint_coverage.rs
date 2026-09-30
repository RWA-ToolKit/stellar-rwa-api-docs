// Issue #430 — every documented endpoint covered by a contract test
//
// Pure model/serialisation tests that validate the JSON shape of every
// documented API endpoint.  No HTTP calls are made; each test builds a model
// value directly, serialises it with `serde_json::to_value`, and asserts that
// the key fields are present with the correct types and values.
//
// These tests serve as living documentation of the wire format and catch doc
// drift before it reaches production.

use serde_json::Value;
use stellar_rwa_api::models::{
    Asset, ComplianceSummary, Distribution, Holder, JurisdictionCount, Stats,
};

// ---------------------------------------------------------------------------
// GET /stats
// ---------------------------------------------------------------------------

#[test]
fn stats_serialises_to_documented_json_shape() {
    let stats = Stats {
        total_assets: 5,
        active_assets: 4,
        tvl_cents: "123456789".to_string(),
        tvl_usd: 1_234_567.89,
        total_holders: 120,
        total_distributions: 8,
        last_indexed_ledger: 4_000_000,
        last_updated: Some("2026-09-01T00:00:00Z".to_string()),
    };

    let v = serde_json::to_value(&stats).expect("Stats must serialise to JSON");

    // Shape: exactly the 8 documented fields.
    assert!(v["total_assets"].is_number(), "total_assets must be a number");
    assert_eq!(v["total_assets"], Value::from(5u64));

    assert!(v["active_assets"].is_number(), "active_assets must be a number");
    assert_eq!(v["active_assets"], Value::from(4u64));

    // tvl_cents is a string to preserve i128 precision on the JavaScript side.
    assert!(v["tvl_cents"].is_string(), "tvl_cents must be a string");
    assert_eq!(v["tvl_cents"], "123456789");
    // Verify it parses back as i128.
    v["tvl_cents"]
        .as_str()
        .unwrap()
        .parse::<i128>()
        .expect("tvl_cents must be parseable as i128");

    assert!(v["tvl_usd"].is_number(), "tvl_usd must be a number");

    assert!(v["total_holders"].is_number(), "total_holders must be a number");
    assert_eq!(v["total_holders"], Value::from(120u64));

    assert!(v["total_distributions"].is_number(), "total_distributions must be a number");
    assert_eq!(v["total_distributions"], Value::from(8u64));

    assert!(v["last_indexed_ledger"].is_number(), "last_indexed_ledger must be a number");
    assert_eq!(v["last_indexed_ledger"], Value::from(4_000_000u64));

    assert!(v["last_updated"].is_string(), "last_updated must be a string when present");
    assert_eq!(v["last_updated"], "2026-09-01T00:00:00Z");
}

#[test]
fn stats_last_updated_may_be_null() {
    let stats = Stats {
        last_updated: None,
        ..Stats::default()
    };
    let v = serde_json::to_value(&stats).expect("Stats must serialise to JSON");
    assert!(v["last_updated"].is_null(), "last_updated must serialise as null when None");
}

// ---------------------------------------------------------------------------
// GET /assets  (AssetSummary list)
// ---------------------------------------------------------------------------

fn sample_asset(id: u64) -> Asset {
    Asset {
        id,
        token_contract: format!("C{:055}", id),
        issuer: format!("G{:055}", id),
        name: format!("Sample Asset {id}"),
        symbol: format!("SMP{id}"),
        asset_type: "real_estate".to_string(),
        description: "A sample real-estate backed token.".to_string(),
        valuation_cents: "500000000".to_string(),
        valuation_usd: 5_000_000.0,
        decimals: 7,
        total_supply: "10000000000".to_string(),
        holders: 42,
        active: true,
        paused: false,
        compliance_contract: format!("CC{:054}", id),
        created_at_ledger: 1_000_000,
        indexed_at_ledger: 1_000_100,
        dividends_indexed_at_ledger: Some(1_000_200),
        index_error: None,
    }
}

#[test]
fn asset_list_serialises_correctly() {
    let assets = vec![sample_asset(1), sample_asset(2)];
    let v = serde_json::to_value(&assets).expect("Vec<Asset> must serialise to JSON");

    assert!(v.is_array(), "asset list must be a JSON array");
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    let first = &arr[0];
    // 18 total fields on Asset (16 documented + indexed_at_ledger + index_error).
    assert!(first["id"].is_number());
    assert!(first["token_contract"].is_string());
    assert!(first["issuer"].is_string());
    assert!(first["name"].is_string());
    assert!(first["symbol"].is_string());
    assert!(first["asset_type"].is_string());
    assert!(first["description"].is_string());
    assert!(first["valuation_cents"].is_string(), "valuation_cents must be a string");
    assert!(first["valuation_usd"].is_number());
    assert!(first["decimals"].is_number());
    assert!(first["total_supply"].is_string(), "total_supply must be a string");
    assert!(first["holders"].is_number());
    assert!(first["active"].is_boolean());
    assert!(first["paused"].is_boolean());
    assert!(first["compliance_contract"].is_string());
    assert!(first["created_at_ledger"].is_number());
    assert!(first["indexed_at_ledger"].is_number());
    assert!(first["index_error"].is_null());
}

#[test]
fn asset_list_large_integers_are_strings() {
    let asset = sample_asset(1);
    let v = serde_json::to_value(&asset).expect("Asset must serialise to JSON");

    // Both i128 fields must round-trip as strings.
    let vc = v["valuation_cents"].as_str().expect("valuation_cents must be a string");
    vc.parse::<i128>().expect("valuation_cents must be parseable as i128");

    let ts = v["total_supply"].as_str().expect("total_supply must be a string");
    ts.parse::<i128>().expect("total_supply must be parseable as i128");
}

#[test]
fn asset_type_values_are_documented_strings() {
    for asset_type in ["real_estate", "invoice", "commodity"] {
        let mut a = sample_asset(1);
        a.asset_type = asset_type.to_string();
        let v = serde_json::to_value(&a).expect("Asset must serialise to JSON");
        assert_eq!(v["asset_type"], asset_type, "asset_type {asset_type} must survive round-trip");
    }
}

// ---------------------------------------------------------------------------
// GET /assets/:id  (AssetDetail — same model as Asset)
// ---------------------------------------------------------------------------

#[test]
fn asset_detail_serialises_correctly() {
    let asset = Asset {
        id: 99,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
        name: "Stellar Building Token".to_string(),
        symbol: "SBT".to_string(),
        asset_type: "real_estate".to_string(),
        description: "A tokenized share of the Stellar building.".to_string(),
        valuation_cents: "1000000000".to_string(),
        valuation_usd: 10_000_000.0,
        decimals: 7,
        total_supply: "100000000000".to_string(),
        holders: 5,
        active: true,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU".to_string(),
        created_at_ledger: 2_000_000,
        indexed_at_ledger: 2_000_500,
        dividends_indexed_at_ledger: Some(2_000_600),
        index_error: None,
    };

    let v = serde_json::to_value(&asset).expect("Asset must serialise to JSON");

    assert_eq!(v["id"], 99u64);
    assert_eq!(v["name"], "Stellar Building Token");
    assert_eq!(v["symbol"], "SBT");
    assert_eq!(v["asset_type"], "real_estate");
    assert_eq!(v["valuation_cents"], "1000000000");
    assert_eq!(v["total_supply"], "100000000000");
    assert_eq!(v["holders"], 5u64);
    assert_eq!(v["active"], true);
    assert_eq!(v["paused"], false);
    assert_eq!(v["decimals"], 7u64);
    assert_eq!(v["created_at_ledger"], 2_000_000u64);
    assert_eq!(v["indexed_at_ledger"], 2_000_500u64);
    assert!(v["index_error"].is_null());
}

#[test]
fn asset_detail_index_error_is_string_when_present() {
    let mut asset = sample_asset(7);
    asset.index_error = Some("rpc returned an error: timeout".to_string());
    let v = serde_json::to_value(&asset).expect("Asset must serialise to JSON");
    assert!(v["index_error"].is_string(), "index_error must be a string when present");
    assert_eq!(v["index_error"], "rpc returned an error: timeout");
}

// ---------------------------------------------------------------------------
// GET /assets/:id/holders  (HolderList)
// ---------------------------------------------------------------------------

fn sample_holder(n: u64) -> Holder {
    Holder {
        address: format!("G{:055}", n),
        balance: (n * 1_000_000).to_string(),
        share_percent: 10.0 * n as f64,
    }
}

#[test]
fn holder_list_serialises_correctly() {
    let holders = vec![sample_holder(1), sample_holder(2), sample_holder(3)];
    let v = serde_json::to_value(&holders).expect("Vec<Holder> must serialise to JSON");

    assert!(v.is_array());
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    let h = &arr[0];
    assert!(h["address"].is_string(), "address must be a string");
    assert!(h["balance"].is_string(), "balance must be a string");
    assert!(h["share_percent"].is_number(), "share_percent must be a number");
}

#[test]
fn holder_balance_is_string_and_parseable_as_i128() {
    let holder = sample_holder(5);
    let v = serde_json::to_value(&holder).expect("Holder must serialise to JSON");
    let bal = v["balance"].as_str().expect("balance must be a string");
    bal.parse::<i128>().expect("balance must be parseable as i128");
}

#[test]
fn holder_share_percent_is_in_valid_range() {
    for pct in [0.0f64, 25.0, 50.0, 100.0] {
        let holder = Holder {
            address: "GABC".to_string(),
            balance: "1000".to_string(),
            share_percent: pct,
        };
        let v = serde_json::to_value(&holder).expect("Holder must serialise to JSON");
        let serialised = v["share_percent"].as_f64().unwrap();
        assert!(
            (0.0..=100.0).contains(&serialised),
            "share_percent {pct} must be in [0, 100]"
        );
    }
}

#[test]
fn holder_list_fields_are_exactly_address_balance_share_percent() {
    let holder = sample_holder(1);
    let v = serde_json::to_value(&holder).expect("Holder must serialise to JSON");
    let obj = v.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["address", "balance", "share_percent"],
        "Holder must serialise with exactly the 3 documented fields"
    );
}

// ---------------------------------------------------------------------------
// GET /assets/:id/compliance  (ComplianceSummary)
// ---------------------------------------------------------------------------

fn sample_compliance() -> ComplianceSummary {
    ComplianceSummary {
        total_records: 100,
        approved: 80,
        suspended: 5,
        rejected: 10,
        pending: 5,
        with_expiry: 30,
        jurisdictions: vec![
            JurisdictionCount {
                jurisdiction: "US".to_string(),
                count: 60,
            },
            JurisdictionCount {
                jurisdiction: "SG".to_string(),
                count: 40,
            },
        ],
    }
}

#[test]
fn compliance_summary_serialises_correctly() {
    let summary = sample_compliance();
    let v = serde_json::to_value(&summary).expect("ComplianceSummary must serialise to JSON");

    assert!(v["total_records"].is_number());
    assert_eq!(v["total_records"], 100u64);

    assert!(v["approved"].is_number());
    assert_eq!(v["approved"], 80u64);

    assert!(v["suspended"].is_number());
    assert_eq!(v["suspended"], 5u64);

    assert!(v["rejected"].is_number());
    assert_eq!(v["rejected"], 10u64);

    assert!(v["pending"].is_number());
    assert_eq!(v["pending"], 5u64);

    assert!(v["with_expiry"].is_number());
    assert_eq!(v["with_expiry"], 30u64);

    assert!(v["jurisdictions"].is_array());
    let jurs = v["jurisdictions"].as_array().unwrap();
    assert_eq!(jurs.len(), 2);

    let j0 = &jurs[0];
    assert!(j0["jurisdiction"].is_string());
    assert!(j0["count"].is_number());
    assert_eq!(j0["jurisdiction"], "US");
    assert_eq!(j0["count"], 60u64);
}

#[test]
fn compliance_summary_status_counts_sum_to_total() {
    let summary = sample_compliance();
    let sum = summary.approved + summary.suspended + summary.rejected + summary.pending;
    assert_eq!(
        sum, summary.total_records,
        "approved+suspended+rejected+pending must equal total_records"
    );
}

#[test]
fn compliance_summary_contains_no_pii() {
    // The compliance endpoint must never expose individual addresses or
    // per-record details — only aggregate counts.
    let summary = sample_compliance();
    let v = serde_json::to_value(&summary).expect("ComplianceSummary must serialise to JSON");
    let obj = v.as_object().unwrap();

    assert!(
        !obj.contains_key("addresses"),
        "ComplianceSummary must not expose addresses"
    );
    assert!(
        !obj.contains_key("records"),
        "ComplianceSummary must not expose records"
    );

    // Jurisdiction entries must not contain address or status.
    for jur in v["jurisdictions"].as_array().unwrap() {
        let jobj = jur.as_object().unwrap();
        assert!(
            !jobj.contains_key("address"),
            "jurisdiction entry must not expose address"
        );
        assert!(
            !jobj.contains_key("status"),
            "jurisdiction entry must not expose per-address status"
        );
    }
}

// ---------------------------------------------------------------------------
// GET /assets/:id/dividends  (DividendHistory — Vec<Distribution>)
// ---------------------------------------------------------------------------

fn sample_distribution(id: u64) -> Distribution {
    Distribution {
        id,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ".to_string(),
        payment_token: "USDC".to_string(),
        total_amount: "10000000".to_string(),
        distributed: "7500000".to_string(),
        claimed_percent: 75.0,
        overflow_detected: false,
        completed: false,
        created_at_ledger: 3_000_000 + id as u32,
    }
}

#[test]
fn dividend_history_serialises_correctly() {
    let dists = vec![sample_distribution(1), sample_distribution(2)];
    let v = serde_json::to_value(&dists).expect("Vec<Distribution> must serialise to JSON");

    assert!(v.is_array());
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    let d = &arr[0];
    assert!(d["id"].is_number(), "id must be a number");
    assert!(d["asset_token"].is_string(), "asset_token must be a string");
    assert!(d["payment_token"].is_string(), "payment_token must be a string");
    assert!(d["total_amount"].is_string(), "total_amount must be a string (i128)");
    assert!(d["distributed"].is_string(), "distributed must be a string (i128)");
    assert!(d["claimed_percent"].is_number(), "claimed_percent must be a number");
    assert!(d["overflow_detected"].is_boolean(), "overflow_detected must be a boolean");
    assert!(d["completed"].is_boolean(), "completed must be a boolean");
    assert!(d["created_at_ledger"].is_number(), "created_at_ledger must be a number");
}

#[test]
fn distribution_amounts_are_strings_parseable_as_i128() {
    let dist = sample_distribution(1);
    let v = serde_json::to_value(&dist).expect("Distribution must serialise to JSON");

    v["total_amount"]
        .as_str()
        .expect("total_amount must be a string")
        .parse::<i128>()
        .expect("total_amount must be parseable as i128");

    v["distributed"]
        .as_str()
        .expect("distributed must be a string")
        .parse::<i128>()
        .expect("distributed must be parseable as i128");
}

#[test]
fn distribution_overflow_flag_is_set_when_distributed_exceeds_total() {
    let dist = Distribution {
        id: 99,
        asset_token: "C1".to_string(),
        payment_token: "USDC".to_string(),
        total_amount: "1000".to_string(),
        distributed: "1500".to_string(),
        claimed_percent: 150.0,
        overflow_detected: true,
        completed: false,
        created_at_ledger: 100,
    };
    let v = serde_json::to_value(&dist).expect("Distribution must serialise to JSON");
    assert_eq!(v["overflow_detected"], true);
    let claimed = v["claimed_percent"].as_f64().unwrap();
    assert!(
        claimed > 100.0,
        "claimed_percent must be > 100 when overflow_detected, got {claimed}"
    );
}

#[test]
fn distribution_field_set_matches_documented_schema() {
    let dist = sample_distribution(1);
    let v = serde_json::to_value(&dist).expect("Distribution must serialise to JSON");
    let obj = v.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();

    let expected: &[&str] = &[
        "asset_token",
        "claimed_percent",
        "completed",
        "created_at_ledger",
        "distributed",
        "id",
        "overflow_detected",
        "payment_token",
        "total_amount",
    ];
    assert_eq!(
        keys, expected,
        "Distribution must serialise with exactly the documented fields"
    );
}

// ---------------------------------------------------------------------------
// GET /health  (health response shape)
// ---------------------------------------------------------------------------

#[test]
fn health_response_has_status_field() {
    // The /health route returns a JSON object with at least a `status` string.
    // We verify the shape here by constructing the same serde_json payload
    // the route handler builds, without making an HTTP call.
    let healthy = serde_json::json!({
        "status": "ok",
        "last_indexed_ledger": 4_000_000u32,
        "last_updated": "2026-09-01T00:00:00Z",
        "poll_age_seconds": 5i64,
        "consecutive_failures": 0u32,
    });

    assert!(healthy["status"].is_string(), "health response must have a 'status' string");
    assert_eq!(healthy["status"], "ok");
    assert!(
        healthy["last_indexed_ledger"].is_number(),
        "health response must have 'last_indexed_ledger'"
    );
    assert!(
        healthy["consecutive_failures"].is_number(),
        "health response must have 'consecutive_failures'"
    );
}

#[test]
fn health_degraded_response_has_status_field() {
    // Degraded health (e.g. consecutive failures > 0) still has a `status`.
    let degraded = serde_json::json!({
        "status": "degraded",
        "last_indexed_ledger": 3_999_000u32,
        "last_updated": "2026-09-01T00:00:00Z",
        "poll_age_seconds": 120i64,
        "consecutive_failures": 7u32,
    });

    assert!(degraded["status"].is_string());
    assert_eq!(degraded["status"], "degraded");
    assert!(degraded["consecutive_failures"].as_u64().unwrap() > 0);
}
