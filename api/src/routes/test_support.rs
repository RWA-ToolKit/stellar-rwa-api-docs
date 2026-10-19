//! Shared helpers for route-level tests: build an [`AppState`] pre-populated
//! with a fixed [`Snapshot`] so handlers can be called directly, without
//! running the indexer or its background poll loop.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use metrics_exporter_prometheus::PrometheusBuilder;
use tower::ServiceExt;

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
        dividends_indexed_at_ledger: None,
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
        created_at_ledger,
    }
}

/// Send a single request through `router` and return the response.
///
/// Shared by route tests that need to exercise the full middleware stack
/// (e.g. conditional-request handling) rather than calling a handler directly.
pub(crate) async fn send(router: axum::Router, request: Request<Body>) -> Response {
    router.oneshot(request).await.expect("router call failed")
}

/// Assert that `router` sets an `ETag` on a plain `GET` and that replaying
/// that `ETag` via `If-None-Match` yields `304 Not Modified`.
///
/// Returns the `ETag` value observed on the initial response so callers can
/// make further assertions if needed.
pub(crate) async fn assert_conditional_get(router: axum::Router, uri: &str) -> String {
    let first = send(
        router.clone(),
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("build request"),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK, "initial GET should succeed");
    let etag = first
        .headers()
        .get(axum::http::header::ETAG)
        .expect("response should set an ETag")
        .to_str()
        .expect("ETag should be valid ASCII")
        .to_string();

    let second = send(
        router,
        Request::builder()
            .uri(uri)
            .header(axum::http::header::IF_NONE_MATCH, &etag)
            .body(Body::empty())
            .expect("build request"),
    )
    .await;
    assert_eq!(
        second.status(),
        StatusCode::NOT_MODIFIED,
        "matching If-None-Match should return 304"
    );

    etag
}

/// Assert that `router` rejects a request with a clear 4xx status (never a
/// 500 or panic) and that the response body uses the shared error shape.
///
/// The shared error body is a JSON object with an `error` field (see
/// `crate::routes::error_body`). Callers pass the malformed `uri` to exercise
/// query-parameter parsing on a given route.
pub(crate) async fn assert_malformed_query_rejected(router: axum::Router, uri: &str) {
    let response = send(
        router,
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("build request"),
    )
    .await;

    let status = response.status();
    assert!(
        status.is_client_error(),
        "malformed query `{uri}` should yield a 4xx, got {status}"
    );
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR, "must not be a 500");

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("error body should be JSON");
    assert!(
        body.get("error").is_some(),
        "error body should use the shared shape with an `error` field, got {body}"
    );
}
