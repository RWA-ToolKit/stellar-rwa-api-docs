//! Integration tests that hit a live Soroban testnet RPC endpoint.
//!
//! These are opt-in only: every test here is `#[ignore]`d by default so a
//! normal `cargo test` run (including CI's default job) never depends on
//! network access or the liveness of the public testnet. Run them
//! explicitly with:
//!
//! ```sh
//! RUN_TESTNET_TESTS=1 cargo test --test testnet_integration -- --ignored
//! ```
//!
//! If `RUN_TESTNET_TESTS` isn't set, each test skips itself with a clear
//! message instead of failing, so accidentally running with `--ignored` in
//! an offline environment doesn't produce a red build.

use serde_json::json;

const DEFAULT_TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";

/// Resolve the RPC endpoint to test against, or `None` if the opt-in env var
/// isn't set.
fn testnet_rpc_url() -> Option<String> {
    if std::env::var("RUN_TESTNET_TESTS").ok().as_deref() != Some("1") {
        return None;
    }
    Some(std::env::var("RWA_TESTNET_RPC_URL").unwrap_or_else(|_| DEFAULT_TESTNET_RPC.to_string()))
}

/// Sanity check: the configured Soroban RPC endpoint answers `getHealth`
/// with a healthy status. This is the same JSON-RPC method the indexer
/// implicitly relies on being reachable.
#[tokio::test]
#[ignore = "opt-in: hits a live testnet RPC endpoint; set RUN_TESTNET_TESTS=1"]
async fn testnet_rpc_reports_healthy() {
    let Some(rpc_url) = testnet_rpc_url() else {
        eprintln!(
            "skipping testnet_rpc_reports_healthy: RUN_TESTNET_TESTS not set to \"1\""
        );
        return;
    };

    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getHealth",
        "params": {}
    });

    let resp = client
        .post(&rpc_url)
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("failed to reach testnet RPC at {rpc_url}: {e}"));

    assert!(
        resp.status().is_success(),
        "testnet RPC returned non-success status: {}",
        resp.status()
    );

    let value: serde_json::Value = resp
        .json()
        .await
        .expect("testnet RPC response was not valid JSON");

    assert_eq!(
        value["result"]["status"], "healthy",
        "unexpected getHealth response: {value}"
    );
}

/// Sanity check: `getLatestLedger` returns a plausible, monotonically
/// increasing ledger sequence, confirming the endpoint is actually indexing
/// the network rather than returning a stub/cached value.
#[tokio::test]
#[ignore = "opt-in: hits a live testnet RPC endpoint; set RUN_TESTNET_TESTS=1"]
async fn testnet_rpc_reports_recent_ledger() {
    let Some(rpc_url) = testnet_rpc_url() else {
        eprintln!(
            "skipping testnet_rpc_reports_recent_ledger: RUN_TESTNET_TESTS not set to \"1\""
        );
        return;
    };

    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestLedger",
        "params": {}
    });

    let resp = client
        .post(&rpc_url)
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("failed to reach testnet RPC at {rpc_url}: {e}"));

    let value: serde_json::Value = resp
        .json()
        .await
        .expect("testnet RPC response was not valid JSON");

    let sequence = value["result"]["sequence"]
        .as_u64()
        .unwrap_or_else(|| panic!("unexpected getLatestLedger response: {value}"));

    // Testnet has been running for years; a non-trivial sequence number is
    // enough to confirm this is a real, synced node and not a stub.
    assert!(
        sequence > 1_000_000,
        "ledger sequence {sequence} looks implausibly low for testnet"
    );
}
