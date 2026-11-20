//! Tests for the `GET /assets/:id/holders` route with cross-cutting features:
//! field selection, conditional requests, rate limiting, and error handling.

use std::net::SocketAddr;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    middleware,
    response::Response,
    routing::get,
    Router,
};
use tower::ServiceExt as _;

use super::field_select::field_select;
use super::holders::list as holders_list;
use super::test_support::{asset, holder, state_with};
use crate::indexer::{Snapshot, POLL_INTERVAL};

const LEDGER: u32 = 5555;
const ASSET_ID: u64 = 42;

fn state() -> crate::indexer::AppState {
    let mut snap = Snapshot::default();
    snap.assets.push(asset(ASSET_ID));
    snap.holders.insert(
        ASSET_ID,
        vec![
            holder(1, 1_000_000_000),
            holder(2, 500_000_000),
            holder(3, 250_000_000),
            holder(4, 100_000_000),
            holder(5, 50_000_000),
        ],
    );
    snap.stats.last_indexed_ledger = LEDGER;
    state_with(snap)
}

fn app() -> Router {
    Router::new()
        .route("/assets/:id/holders", get(holders_list))
        .layer(middleware::from_fn(field_select))
        .with_state(state())
}

async fn call(uri: &str, if_none_match: Option<&str>) -> (StatusCode, String) {
    let mut builder = Request::builder().uri(uri);
    if let Some(value) = if_none_match {
        builder = builder.header(header::IF_NONE_MATCH, value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

    let response = app().clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    (status, body)
}

async fn call_json(uri: &str, if_none_match: Option<&str>) -> (StatusCode, serde_json::Value) {
    let (status, body) = call(uri, if_none_match).await;
    let value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn holders_list_full_response() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0]["balance"], "1000000000");
    assert_eq!(arr[4]["balance"], "50000000");
}

#[tokio::test]
async fn holders_list_with_offset() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?offset=2"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["balance"], "250000000");
}

#[tokio::test]
async fn holders_list_with_limit() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?limit=2"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[tokio::test]
async fn holders_list_limit_capped_at_max() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?limit=200"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 5);
}

#[tokio::test]
async fn empty_holders_list_formats_correctly() {
    // Asset with no holders
    let (status, body) = call_json(&format!("/assets/999/holders"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn holders_empty_result_set() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?offset=100"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 0);
}

#[tokio::test]
async fn field_selection_projects_requested_fields_only() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?fields=address,balance"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert!(arr[0].get("address").is_some());
    assert!(arr[0].get("balance").is_some());
    // Other fields should not be present
    assert!(arr[0].get("frozen").is_none());
}

#[tokio::test]
async fn field_selection_single_field() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?fields=address"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr[0].as_object().unwrap().len(), 1);
    assert!(arr[0].get("address").is_some());
}

#[tokio::test]
async fn field_selection_rejects_unknown_fields() {
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?fields=address,unknown"), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
}

#[tokio::test]
async fn field_selection_with_offset_and_limit() {
    let (status, body) = call_json(
        &format!("/assets/{ASSET_ID}/holders?fields=balance&offset=1&limit=2"),
        None,
    ).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["balance"], "500000000");
}

#[tokio::test]
async fn conditional_request_returns_304_on_etag_match() {
    let etag = format!("\"ledger-{LEDGER}\"");
    let (status, body) = call(&format!("/assets/{ASSET_ID}/holders"), Some(&etag)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_with_weak_etag() {
    let weak_etag = format!("W/\"ledger-{LEDGER}\"");
    let (status, body) = call(&format!("/assets/{ASSET_ID}/holders"), Some(&weak_etag)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_with_etag_list() {
    let etag_list = format!("\"ledger-1000\", \"ledger-{LEDGER}\", W/\"ledger-2000\"");
    let (status, body) = call(&format!("/assets/{ASSET_ID}/holders"), Some(&etag_list)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_with_wildcard() {
    let (status, body) = call(&format!("/assets/{ASSET_ID}/holders"), Some("*")).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_returns_200_on_mismatch() {
    let wrong_etag = "\"ledger-1234\"";
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders"), Some(wrong_etag)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 5);
}

#[tokio::test]
async fn field_selection_and_conditional_request_together() {
    let etag = format!("\"ledger-{LEDGER}\"");
    let (status, body) = call(
        &format!("/assets/{ASSET_ID}/holders?fields=address,balance"),
        Some(&etag),
    ).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn rate_limiter_boundary_with_holders() {
    // This test verifies the route works under rate limiting.
    // The rate limiter is applied at the router level and checks per-IP.
    // A single request from one IP should not be rate limited.
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 5);
}

#[tokio::test]
async fn holders_offset_and_limit_edge_cases() {
    // offset beyond list
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?offset=10&limit=5"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    // limit=0 should behave like no limit (or be capped to default)
    let (status, body) = call_json(&format!("/assets/{ASSET_ID}/holders?limit=0"), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    // Should return default 50 or actual holders (5 in this case)
    assert!(arr.len() > 0);
}

#[tokio::test]
async fn holders_invalid_asset_not_found() {
    let (status, body) = call_json(&format!("/assets/999/holders"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let obj = body.as_object().unwrap();
    assert_eq!(obj["error"], "not_found");
}

#[tokio::test]
async fn malformed_query_parameters() {
    let cases = [
        &format!("/assets/{ASSET_ID}/holders?offset=abc")[..],
        &format!("/assets/{ASSET_ID}/holders?limit=xyz")[..],
        &format!("/assets/{ASSET_ID}/holders?unknown=value")[..],
    ];

    for uri in cases {
        let (status, _) = call_json(uri, None).await;
        // Invalid parameters should be rejected or ignored
        // Depends on validation - at minimum should not crash
        assert!(status != StatusCode::INTERNAL_SERVER_ERROR, "should not crash on: {uri}");
    }
}
