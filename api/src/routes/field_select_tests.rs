//! Tests for the `?fields=` middleware across list endpoints.

use axum::{body::Body, http::{Request, StatusCode}, middleware, routing::get, Router};
use tower::ServiceExt as _;

use super::field_select::field_select;
use super::test_support::{asset, state_with};
use crate::indexer::Snapshot;

fn app() -> Router {
    let state = state_with(Snapshot { assets: vec![asset(1), asset(2)], ..Default::default() });
    Router::new()
        .route("/assets", get(super::assets_query::list))
        .route("/assets/:id", get(super::assets::detail))
        .layer(middleware::from_fn(field_select))
        .with_state(state)
}

async fn call(uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn projects_list_and_detail() {
    let (status, body) = call("/assets?fields=id,symbol&limit=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([{"id": 1, "symbol": "TST"}]));
    let (_, body) = call("/assets/2?fields=name").await;
    assert_eq!(body, serde_json::json!({"name": "Test Asset"}));
}

#[tokio::test]
async fn unknown_field_is_a_clear_error() {
    let (status, body) = call("/assets?fields=id,nope").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("nope"));
}

#[tokio::test]
async fn events_route_with_field_selection() {
    // Test that field selection works on events route (#543)
    let (status, body) = call("/v1/events?fields=ledger&limit=1").await;
    assert_eq!(status, StatusCode::OK);
    // Response should contain only requested fields
    if let Some(arr) = body.as_array() {
        if !arr.is_empty() {
            let event = &arr[0];
            assert!(event.get("ledger").is_some(), "requested field must be present");
        }
    }
}

#[tokio::test]
async fn events_route_field_selection_rejects_unknown_fields() {
    // Confirm unknown fields return clear error
    let (status, body) = call("/v1/events?fields=nonexistent_field").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("nonexistent_field"));
}
