//! Shared helpers for route-level tests: build an [`AppState`] pre-populated
//! with a fixed [`Snapshot`] so handlers can be called directly, without
//! running the indexer or its background poll loop.

use metrics_exporter_prometheus::PrometheusBuilder;

use crate::indexer::{AppState, Config, Snapshot};
use crate::models::{Asset, Distribution};

pub(crate) fn state_with(snapshot: Snapshot) -> AppState {
    let config = Config {
        rpc_url: "http://127.0.0.1:0".to_string(),
        registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3".to_string(),
        dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX".to_string(),
        read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
    };
    // `build()` (not `install_recorder()`) skips the global recorder: each
    // test in this binary needs its own handle, and the global recorder can
    // only be installed once per process.
    let metrics = PrometheusBuilder::new().build_recorder().handle();
    AppState::for_test(config, metrics, snapshot)
}

pub(crate) fn asset(id: u64) -> Asset {
    Asset {
        id,
        token_contract: "CONTRACT".to_string(),
        issuer: "ISSUER".to_string(),
        name: "Test Asset".to_string(),
        symbol: "TST".to_string(),
        asset_type: "real_estate".to_string(),
        description: String::new(),
        valuation_cents: "0".to_string(),
        valuation_usd: 0.0,
        decimals: 7,
        total_supply: "0".to_string(),
        holders: 0,
        active: true,
        paused: false,
        compliance_contract: "COMPLIANCE".to_string(),
        created_at_ledger: 1,
        indexed_at_ledger: 1,
        index_error: None,
    }
}

pub(crate) fn distribution(id: u64, created_at_ledger: u32) -> Distribution {
    Distribution {
        id,
        asset_token: "TOKEN".to_string(),
        payment_token: "PAY".to_string(),
        total_amount: "1000".to_string(),
        distributed: "0".to_string(),
        claimed_percent: 0.0,
        overflow_detected: false,
        completed: false,
        snapshot_ledger: created_at_ledger,
        created_at_ledger,
    }
}
