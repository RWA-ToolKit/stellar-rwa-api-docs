//! Smoke test: builds and starts the actual `stellar-rwa-api` binary as a
//! subprocess with default configuration, waits for it to come up, and
//! probes `/health` and `/version` before killing it.
//!
//! Unlike the rest of the suite, this exercises the real `main()` — process
//! startup, config parsing, port binding, and the indexer spawn — instead of
//! calling handlers directly against an in-memory `AppState`. It doesn't
//! need network access to pass: the default `Config::from_env()` config
//! talks to the public testnet RPC, but `/health` and `/version` both
//! respond before (or independent of) the indexer completing its first
//! successful refresh. `/health` may report `"degraded"` if the indexer
//! hasn't gotten a good snapshot yet — that's still `200 OK` with the
//! expected shape, which is all this test checks.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Kills the child process on drop so a failed assertion doesn't leave a
/// zombie `stellar-rwa-api` bound to the port.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn binary_boots_and_serves_health_and_version() {
    let port = 18080u16;
    let base_url = format!("http://127.0.0.1:{port}");

    let bin = env!("CARGO_BIN_EXE_stellar-rwa-api");
    let _child = ChildGuard(
        Command::new(bin)
            .env("PORT", port.to_string())
            .env("RUST_LOG", "error")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn stellar-rwa-api binary"),
    );

    let client = reqwest::Client::new();
    wait_until_ready(&client, &base_url, Duration::from_secs(15)).await;

    let version: serde_json::Value = client
        .get(format!("{base_url}/version"))
        .send()
        .await
        .expect("GET /version failed")
        .json()
        .await
        .expect("/version body was not JSON");
    assert!(
        version["version"].as_str().is_some(),
        "/version missing `version` field: {version}"
    );
    assert!(
        version["release"].as_str().is_some(),
        "/version missing `release` field: {version}"
    );

    let health_resp = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("GET /health failed");
    let status = health_resp.status();
    assert!(
        status == 200 || status == 503,
        "unexpected /health status: {status}"
    );
    let health: serde_json::Value = health_resp.json().await.expect("/health body was not JSON");
    assert!(
        health["status"].as_str().is_some(),
        "/health missing `status` field: {health}"
    );

    // `_child` is dropped here, killing and reaping the subprocess.
}

/// Poll `/health` until it responds (any status) or the timeout elapses.
async fn wait_until_ready(client: &reqwest::Client, base_url: &str, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if client.get(format!("{base_url}/health")).send().await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("stellar-rwa-api did not become ready within {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
