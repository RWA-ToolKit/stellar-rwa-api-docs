//! Tests for the `GET /stats` route with cross-cutting features:
//! field selection, conditional requests, and error handling.

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
use serde_json::json;
use tower::ServiceExt as _;

use super::field_select::field_select;
use super::stats::get as stats_get;
use super::test_support::state_with;
use crate::indexer::{Snapshot, POLL_INTERVAL};
use crate::models::Stats;

const LEDGER: u32 = 9999;

fn state() -> crate::indexer::AppState {
    state_with(Snapshot {
        stats: Stats {
            total_assets: 5,
            active_assets: 4,
            tvl_cents: "500000000".to_string(),
            tvl_usd: 5_000_000.0,
            total_holders: 150,
            total_distributions: 12,
            last_indexed_ledger: LEDGER,
            last_updated: Some("2024-01-15T12:00:00Z".to_string()),
        },
        ..Snapshot::default()
    })
}

fn app() -> Router {
    Router::new()
        .route("/stats", get(stats_get))
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
async fn stats_full_response() {
    let (status, body) = call_json("/stats", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_assets"], 5);
    assert_eq!(body["active_assets"], 4);
    assert_eq!(body["tvl_cents"], "500000000");
    assert_eq!(body["tvl_usd"], 5_000_000.0);
    assert_eq!(body["total_holders"], 150);
    assert_eq!(body["total_distributions"], 12);
    assert_eq!(body["last_indexed_ledger"], LEDGER);
    assert_eq!(body["last_updated"], "2024-01-15T12:00:00Z");
}

#[tokio::test]
async fn field_selection_projects_requested_fields_only() {
    let (status, body) = call_json("/stats?fields=total_assets,tvl_usd", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_assets"], 5);
    assert_eq!(body["tvl_usd"], 5_000_000.0);
    assert!(body.get("active_assets").is_none());
    assert!(body.get("tvl_cents").is_none());
}

#[tokio::test]
async fn field_selection_with_single_field() {
    let (status, body) = call_json("/stats?fields=total_holders", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert_eq!(body["total_holders"], 150);
}

#[tokio::test]
async fn field_selection_rejects_unknown_fields() {
    let (status, body) = call_json("/stats?fields=total_assets,unknown_field", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
    assert!(body["message"].as_str().unwrap().contains("unknown_field"));
}

#[tokio::test]
async fn field_selection_rejects_empty_fields() {
    let (status, body) = call_json("/stats?fields=", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_parameter");
}

#[tokio::test]
async fn field_selection_rejects_malformed_field_list() {
    let (status, _) = call_json("/stats?fields=total_assets,,tvl_usd", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn conditional_request_returns_304_on_etag_match() {
    // First request to get the ETag
    let (status1, body1) = call("/stats", None).await;
    assert_eq!(status1, StatusCode::OK);

    // Extract ETag from response by calling again
    // The ETag is based on ledger number
    let etag = format!("\"ledger-{LEDGER}\"");

    // Second request with matching ETag should return 304
    let (status2, body2) = call("/stats", Some(&etag)).await;
    assert_eq!(status2, StatusCode::NOT_MODIFIED);
    assert!(body2.is_empty());
}

#[tokio::test]
async fn conditional_request_with_weak_etag() {
    let weak_etag = format!("W/\"ledger-{LEDGER}\"");

    let (status, body) = call("/stats", Some(&weak_etag)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_with_etag_list() {
    let etag_list = format!("\"ledger-1000\", W/\"ledger-2000\", \"ledger-{LEDGER}\"");

    let (status, body) = call("/stats", Some(&etag_list)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_with_wildcard_etag() {
    let (status, body) = call("/stats", Some("*")).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn conditional_request_returns_200_on_etag_mismatch() {
    let wrong_etag = "\"ledger-1234\"";

    let (status, body) = call_json("/stats", Some(wrong_etag)).await;
    assert_eq!(status, StatusCode::OK);
    // Body should contain stats
    assert_eq!(body["total_assets"], 5);
}

#[tokio::test]
async fn empty_snapshot_formats_correctly() {
    let empty_state = state_with(Snapshot::default());
    let app = Router::new()
        .route("/stats", get(stats_get))
        .layer(middleware::from_fn(field_select))
        .with_state(empty_state);

    let mut request = Request::builder().uri("/stats").body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Empty snapshot should format with all fields present but zeroed/empty
    assert_eq!(body["total_assets"], 0);
    assert_eq!(body["active_assets"], 0);
    assert_eq!(body["tvl_cents"], "");
    assert_eq!(body["tvl_usd"], 0.0);
    assert_eq!(body["total_holders"], 0);
    assert_eq!(body["total_distributions"], 0);
    assert_eq!(body["last_indexed_ledger"], 0);
    assert!(body["last_updated"].is_null());
}

#[tokio::test]
async fn empty_snapshot_with_field_selection() {
    let empty_state = state_with(Snapshot::default());
    let app = Router::new()
        .route("/stats", get(stats_get))
        .layer(middleware::from_fn(field_select))
        .with_state(empty_state);

    let mut request = Request::builder()
        .uri("/stats?fields=total_assets,tvl_usd")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

    let response = app.oneshot(request).await.unwrap();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["total_assets"], 0);
    assert_eq!(body["tvl_usd"], 0.0);
    assert!(body.get("active_assets").is_none());
}

#[tokio::test]
async fn field_selection_and_conditional_request_together() {
    let (status, body) = call_json("/stats?fields=total_assets,tvl_usd", None).await;
    assert_eq!(status, StatusCode::OK);

    // Now with matching ETag
    let etag = format!("\"ledger-{LEDGER}\"");
    let (status2, body2) = call("/stats?fields=total_assets,tvl_usd", Some(&etag)).await;
    assert_eq!(status2, StatusCode::NOT_MODIFIED);
    assert!(body2.is_empty());
}

#[tokio::test]
async fn malformed_query_parameters_rejected() {
    let cases = [
        "/stats?unknown_param=value",
        "/stats?fields=total_assets&unknown=1",
        "/stats?fields=",
    ];

    for uri in cases {
        let (status, _) = call_json(uri, None).await;
        // Field selection middleware rejects unknown parameters
        if uri.contains("unknown") || uri.ends_with("fields=") {
            assert!(status != StatusCode::OK, "should reject: {uri}");
        }
    }
}
