//! Integration tests for GET /assets/:id/holders endpoint.
//! Tests that require the full HTTP stack and path parameter extraction.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use stellar_rwa_api::{indexer::Snapshot, routes, routes::test_support::state_with};
use tower::ServiceExt;

#[tokio::test]
async fn non_numeric_asset_id_returns_400_error() {
    // Test that a non-numeric asset ID is rejected with 400 Bad Request
    // The Path extractor will fail to parse "abc" as u64
    let state = state_with(Snapshot::default());
    let app = routes::router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/assets/abc/holders")
        .body(Body::empty())
        .expect("request should be valid");

    let response = app.oneshot(request).await.expect("request should complete");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Non-numeric asset ID should return 400 Bad Request"
    );

    let (_, body) = response.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("body should be readable");

    let error_body: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid JSON");

    assert!(
        error_body.get("error").is_some(),
        "Error response must have 'error' field"
    );
    assert!(
        error_body.get("message").is_some(),
        "Error response must have 'message' field"
    );
    assert!(error_body["message"].is_string());
}
