//! Drives the rate limiter to its boundary through the full `router()`.
//!
//! Each test configures the limits explicitly through
//! `router_with_rate_limit` instead of the `RWA_RATE_LIMIT_*` env vars, so
//! nothing here can leak into other tests running in parallel.

use std::net::SocketAddr;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use serde_json::Value;
use tower::ServiceExt as _;

use crate::indexer::AppState;

use super::router_with_rate_limit;

/// Build a router whose limiter allows `burst` requests and then refills one
/// request every `per_second`-th of a second (a very slow refill here, so the
/// boundary is deterministic). The limits are passed explicitly rather than
/// through env vars, which other concurrently running tests would also read.
fn router_with_limits(per_second: u64, burst: u32) -> Router {
    router_with_rate_limit(AppState::for_test_empty(), per_second, burst)
}

async fn get_from(app: &Router, ip: [u8; 4], uri: &str) -> Response {
    let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((ip, 54321))));
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn requests_up_to_the_burst_succeed_and_the_next_is_throttled() {
    let burst = 3;
    let app = router_with_limits(1, burst);

    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 1], "/version").await;
        assert_eq!(response.status(), StatusCode::OK, "request {n} within burst");
    }

    let throttled = get_from(&app, [10, 0, 0, 1], "/version").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn throttled_response_has_status_and_a_non_empty_body() {
    let app = router_with_limits(1, 1);

    assert_eq!(
        get_from(&app, [10, 0, 0, 2], "/version").await.status(),
        StatusCode::OK
    );
    let throttled = get_from(&app, [10, 0, 0, 2], "/version").await;

    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "a throttled response explains itself");
}

#[tokio::test]
async fn limit_is_tracked_per_client_ip() {
    let app = router_with_limits(1, 1);

    assert_eq!(
        get_from(&app, [10, 0, 0, 3], "/version").await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get_from(&app, [10, 0, 0, 3], "/version").await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    // A different client still has its own full bucket.
    assert_eq!(
        get_from(&app, [10, 0, 0, 4], "/version").await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn indexer_route_is_throttled_at_the_burst_boundary() {
    let burst = 2;
    let app = router_with_limits(1, burst);

    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], "/indexer").await;
        assert_eq!(response.status(), StatusCode::OK, "request {n} within burst");
    }

    let throttled = get_from(&app, [10, 0, 0, 5], "/indexer").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn indexer_throttled_response_uses_the_shared_error_body_shape() {
    let app = router_with_limits(1, 1);

    assert_eq!(
        get_from(&app, [10, 0, 0, 6], "/indexer").await.status(),
        StatusCode::OK
    );
    let throttled = get_from(&app, [10, 0, 0, 6], "/indexer").await;

    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).expect("429 body is JSON");
    let error = json
        .get("error")
        .and_then(Value::as_object)
        .expect("429 body carries the shared `error` object");
    assert!(
        error.get("code").and_then(Value::as_str).is_some(),
        "shared error body has a string `code`"
    );
    assert!(
        error.get("message").and_then(Value::as_str).is_some(),
        "shared error body has a string `message`"
    );
}
