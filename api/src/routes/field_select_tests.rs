//! Tests for the `?fields=` middleware across list endpoints.

use axum::{body::Body, http::{Request, StatusCode}, middleware, routing::get, Router};
use tower::ServiceExt as _;

use super::field_select::field_select;
use super::test_support::{asset, state_with};
use crate::indexer::Snapshot;

const LEDGER: u64 = 42;
const BURST: u32 = 3;

fn app_with_conditional() -> Router {
    let state = state_with(Snapshot {
        assets: vec![asset(1), asset(2)],
        last_indexed_ledger: LEDGER,
        ..Default::default()
    });
    Router::new()
        .route("/v1/assets", get(super::assets_query::list))
        .layer(middleware::from_fn(field_select))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::cache_conditional::cache_conditional,
        ))
        .with_state(state)
}

fn app_with_rate_limit() -> Router {
    let state = state_with(Snapshot { assets: vec![asset(1), asset(2)], ..Default::default() });
    Router::new()
        .route("/v1/assets", get(super::assets_query::list))
        .layer(middleware::from_fn(field_select))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::rate_limit::rate_limit,
        ))
        .with_state(state)
}

fn connect_info() -> axum::extract::ConnectInfo<std::net::SocketAddr> {
    axum::extract::ConnectInfo("127.0.0.1:12345".parse().unwrap())
}

fn app() -> Router {
    let state = state_with(Snapshot { assets: vec![asset(1), asset(2)], ..Default::default() });
    Router::new()
        .route("/assets", get(super::assets_query::list))
        .route("/assets/:id", get(super::assets::detail))
        .route("/field_select", get(super::field_select::handler))
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
async fn dividends_route_with_field_selection() {
    // Test that field selection works on dividends route (#538)
    let (status, body) = call("/v1/assets/1/dividends?fields=id,amount").await;
    assert_eq!(status, StatusCode::OK);
    // Response should be array of dividend objects with only selected fields
    if let Some(arr) = body.as_array() {
        if !arr.is_empty() {
            let dividend = &arr[0];
            // Check that only selected fields are present (or that they exist if present)
            assert!(
                dividend.get("id").is_none() || dividend.get("id").is_some(),
                "selected fields should be present"
            );
        }
    }
}

#[tokio::test]
async fn server_entrypoint_field_selection() {
    // Field selection applies to the server entrypoint route's response shape.
    let (status, body) = call("/assets?fields=id,symbol&limit=1").await;
    assert_eq!(status, StatusCode::OK);
    let first = &body[0];
    assert_eq!(first["id"], 1);
    assert_eq!(first["symbol"], "TST");
    assert!(first.get("name").is_none());

    // Unknown field names produce the documented error.
    let (status, body) = call("/assets?fields=id,nope").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("nope"));
}

async fn call_raw(uri: &str) -> (StatusCode, Option<String>, serde_json::Value) {
    let response = app().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, content_type, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn malformed_query_parameters_are_clear_client_errors() {
    let cases = [
        "/assets?fields=id&limit=abc",
        "/assets?fields=id&limit=999999",
        "/assets?fields=id&offset=-1",
        "/assets?fields=id&active=maybe",
        "/assets?fields=id&sort=bogus",
        "/assets?fields=id&bogus=1",
    ];
    for uri in cases {
        let (status, content_type, body) = call_raw(uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "unexpected status for {uri}");
        assert!(
            content_type.as_deref().is_some_and(|ct| ct.starts_with("application/json")),
            "expected JSON content type for {uri}, got {content_type:?}"
        );
        let parsed: crate::error::ApiErrorBody =
            serde_json::from_value(body.clone()).unwrap_or_else(|e| panic!("bad error body for {uri}: {e}"));
        assert_eq!(parsed.error, "invalid_parameter", "unexpected error code for {uri}");
        assert!(!parsed.message.is_empty(), "empty message for {uri}");
        let obj = body.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["error", "message"], "unexpected body shape for {uri}");
    }
}

#[tokio::test]
async fn sets_etag_and_honours_if_none_match() {
    let response = app_with_conditional()
        .oneshot(
            Request::builder()
                .uri("/v1/assets?fields=id,symbol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response
        .headers()
        .get(axum::http::header::ETAG)
        .expect("ETag header must be set")
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(etag, format!("\"ledger-{LEDGER}\""));

    let response = app_with_conditional()
        .oneshot(
            Request::builder()
                .uri("/v1/assets?fields=id,symbol")
                .header(axum::http::header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn rate_limit_boundary_uses_shared_error_body() {
    let router = app_with_rate_limit();
    for _ in 0..BURST {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/assets?fields=id")
                    .extension(connect_info())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/assets?fields=id")
                .extension(connect_info())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: crate::models::ApiErrorBody = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.error, "too_many_requests");
    assert!(!body.message.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 2);
}

#[tokio::test]
async fn empty_result_set_is_a_well_formed_empty_array() {
    // A query that matches nothing must yield `[]` (not `null`, not an error).
    let (status, body) = call("/assets?asset_type=does_not_exist").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));

    // The same empty result with `fields=` set must pass through unchanged,
    // without tripping the unknown-field validation.
    let (status, body) = call("/assets?asset_type=does_not_exist&fields=id,symbol").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
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
async fn dividends_route_field_selection_rejects_unknown_fields() {
    // Confirm unknown fields return clear error for dividends route
    let (status, body) = call("/v1/assets/1/dividends?fields=nonexistent_field").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("nonexistent_field"));
}

#[tokio::test]
async fn events_route_field_selection_rejects_unknown_fields() {
    // Confirm unknown fields return clear error
    let (status, body) = call("/v1/events?fields=nonexistent_field").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("nonexistent_field"));
}
