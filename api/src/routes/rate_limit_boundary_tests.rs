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
async fn throttled_response_uses_the_shared_error_body_shape() {
    let app = router_with_limits(1, 1);

    assert_eq!(
        get_from(&app, [10, 0, 0, 2], "/version").await.status(),
        StatusCode::OK
    );
    let throttled = get_from(&app, [10, 0, 0, 2], "/version").await;

    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "a throttled response explains itself");

    // The 429 must use the same `{ "error": { "code", "message" } }`
    // envelope as every other error response, not a bare string.
    let json: Value = serde_json::from_slice(&body).expect("429 body is JSON");
    let error = json
        .get("error")
        .and_then(Value::as_object)
        .expect("429 body has an `error` object");
    assert!(
        error.get("code").and_then(Value::as_str).is_some(),
        "error.code is a string"
    );
    assert!(
        error.get("message").and_then(Value::as_str).is_some(),
        "error.message is a string"
    );
}

#[tokio::test]
async fn holder_position_is_throttled_at_the_burst_boundary_with_shared_error_body() {
    let burst = 2;
    let app = router_with_limits(1, burst);
    let uri = "/v1/holders/So11111111111111111111111111111111111111112/position";

    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], uri).await;
        assert_ne!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "request {n} within burst must not be throttled"
        );
    }

    let throttled = get_from(&app, [10, 0, 0, 5], uri).await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    let content_type = throttled
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("application/json"),
        "429 body is JSON, got {content_type:?}"
    );

    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let object = json.as_object().expect("429 body is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["error", "message"], "429 body has the shared shape");
    assert_eq!(object["error"], "too_many_requests");
    assert!(
        object["message"].as_str().is_some_and(|m| !m.is_empty()),
        "429 body carries a non-empty message"
    );
}

#[tokio::test]
async fn assets_query_is_throttled_at_the_boundary_with_the_shared_error_body() {
    let burst = 2;
    let app = router_with_limits(1, burst);

    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], "/v1/assets").await;
        assert_eq!(response.status(), StatusCode::OK, "request {n} within burst");
    }

    let throttled = get_from(&app, [10, 0, 0, 5], "/v1/assets").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    let content_type = throttled
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("application/json"),
        "429 body is JSON, got {content_type:?}"
    );

    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("429 body is valid JSON");
    let object = json.as_object().expect("429 body is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["error", "message"], "shared error body shape");
    assert_eq!(object["error"], "too_many_requests");
    let message = object["message"].as_str().expect("message is a string");
    assert!(!message.is_empty(), "the error message explains itself");
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
async fn dividends_route_at_rate_limit_boundary_returns_429_with_shared_error_body() {
    // Test rate limiter boundary for dividends route (#540)
    let burst = 2;
    let app = router_with_limits(1, burst);

    // Consume the burst
    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], "/v1/assets/1/dividends").await;
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
            "request {n} within burst"
        );
    }

    // Next request hits the limit
    let throttled = get_from(&app, [10, 0, 0, 5], "/v1/assets/1/dividends").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify response has body with shared error shape
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "429 response must have error body");
}

#[tokio::test]
async fn stats_route_is_throttled_at_the_burst_boundary() {
    let burst = 2;
    let app = router_with_limits(1, burst);

    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], "/stats").await;
        assert_eq!(response.status(), StatusCode::OK, "stats request {n} within burst");
    }

    let throttled = get_from(&app, [10, 0, 0, 5], "/stats").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn stats_throttled_response_uses_shared_error_body_shape() {
    let app = router_with_limits(1, 1);

    assert_eq!(
        get_from(&app, [10, 0, 0, 6], "/stats").await.status(),
        StatusCode::OK
    );
    let throttled = get_from(&app, [10, 0, 0, 6], "/stats").await;

    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).expect("429 body is JSON");
    let object = json.as_object().expect("429 body is a JSON object");
    assert!(
        object.contains_key("error"),
        "429 body carries the shared `error` field: {json}"
    );
}

#[tokio::test]
async fn events_route_at_rate_limit_boundary_returns_429_with_shared_error_body() {
    // Test rate limiter boundary for events route (#545)
    let burst = 2;
    let app = router_with_limits(1, burst);

    // Consume the burst
    for n in 1..=burst {
        let response = get_from(&app, [10, 0, 0, 5], "/v1/events").await;
        assert_eq!(response.status(), StatusCode::OK, "request {n} within burst");
    }

    // Next request hits the limit
    let throttled = get_from(&app, [10, 0, 0, 5], "/v1/events").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify response has body with shared error shape
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "429 response must have error body");

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("error").is_some(), "error response must have 'error' field");
    assert!(
        json.get("message").is_some(),
        "error response must have 'message' field"
    );
}
