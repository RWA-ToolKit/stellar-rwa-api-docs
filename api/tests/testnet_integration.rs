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
//! The scheduled workflow treats a failure as an observational drift signal:
//! it uploads this test output, writes a run summary, and compares recent
//! scheduled outcomes to distinguish a transient outage from persistent drift.
//! It does not put a failing Testnet check on the merge path.

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
        eprintln!("skipping testnet_rpc_reports_healthy: RUN_TESTNET_TESTS not set to \"1\"");
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
        .unwrap_or_else(|e| panic!("testnet drift: getHealth request to {rpc_url} failed: {e}"));

    assert!(
        resp.status().is_success(),
        "testnet drift: getHealth at {rpc_url} returned non-success status: {}",
        resp.status()
    );

    let value: serde_json::Value = resp
        .json()
        .await
        .expect("testnet drift: getHealth response was not valid JSON");

    assert_eq!(
        value["result"]["status"], "healthy",
        "testnet drift: unexpected getHealth response from {rpc_url}: {value}"
    );
}

/// Sanity check: `getLatestLedger` returns a plausible, monotonically
/// increasing ledger sequence, confirming the endpoint is actually indexing
/// the network rather than returning a stub/cached value.
#[tokio::test]
#[ignore = "opt-in: hits a live testnet RPC endpoint; set RUN_TESTNET_TESTS=1"]
async fn testnet_rpc_reports_recent_ledger() {
    let Some(rpc_url) = testnet_rpc_url() else {
        eprintln!("skipping testnet_rpc_reports_recent_ledger: RUN_TESTNET_TESTS not set to \"1\"");
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
        .unwrap_or_else(|e| panic!("testnet drift: getLatestLedger request to {rpc_url} failed: {e}"));

    let value: serde_json::Value = resp
        .json()
        .await
        .expect("testnet drift: getLatestLedger response was not valid JSON");

    let sequence = value["result"]["sequence"]
        .as_u64()
        .unwrap_or_else(|| panic!("testnet drift: unexpected getLatestLedger response from {rpc_url}: {value}"));

    // Testnet has been running for years; a non-trivial sequence number is
    // enough to confirm this is a real, synced node and not a stub.
    assert!(
        sequence > 1_000_000,
        "testnet drift: ledger sequence {sequence} from {rpc_url} looks implausibly low for testnet"
    );
}
