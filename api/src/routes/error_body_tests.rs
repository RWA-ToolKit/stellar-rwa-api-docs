//! Tests for the error_body middleware ensuring consistent error shape across all routes.

use std::net::SocketAddr;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use tower::ServiceExt as _;

use crate::indexer::AppState;

use super::router_with_rate_limit;

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
async fn error_body_empty_result_set_returns_well_formed_array() {
    // Test that routes returning empty results have proper formatting (#527)
    // An empty array is well-formed, not null or error
    let app = router_with_limits(1, 10);

    let response = get_from(&app, [10, 0, 0, 1], "/v1/assets").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array(), "empty result should be an array, not null or error");
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn error_body_field_selection_with_empty_results() {
    // Test that field selection parameter works with empty results (#528)
    let app = router_with_limits(1, 10);

    let response = get_from(&app, [10, 0, 0, 2], "/v1/assets?fields=id,symbol").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array(), "field selection should return array even when empty");
}

#[tokio::test]
async fn error_body_conditional_requests_with_empty_results() {
    // Test that ETag and If-None-Match work correctly with empty results (#529)
    let app = router_with_limits(1, 10);

    let response = get_from(&app, [10, 0, 0, 3], "/v1/stats").await;
    assert_eq!(response.status(), StatusCode::OK);

    let etag = response
        .headers()
        .get(axum::http::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(etag_val) = etag {
        // Try with matching ETag - should get 304
        let mut request = Request::builder()
            .uri("/v1/stats")
            .header(axum::http::header::IF_NONE_MATCH, &etag_val)
            .body(Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 4], 54321))));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }
}

#[tokio::test]
async fn error_body_rate_limit_returns_shared_error_shape() {
    // Test that rate limiter returns error in shared error body format (#530)
    let burst = 1;
    let app = router_with_limits(1, burst);

    // First request succeeds
    let response = get_from(&app, [10, 0, 0, 5], "/version").await;
    assert_eq!(response.status(), StatusCode::OK);

    // Second request hits rate limit
    let throttled = get_from(&app, [10, 0, 0, 5], "/version").await;
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify 429 response has proper error body with "error" and "message" fields
    let body = to_bytes(throttled.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "rate limit response must have error body");

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("error").is_some(), "error response must have 'error' field");
    assert!(
        json.get("message").is_some(),
        "error response must have 'message' field"
    );
    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("too_many_requests"),
        "error code should match status"
    );
}
