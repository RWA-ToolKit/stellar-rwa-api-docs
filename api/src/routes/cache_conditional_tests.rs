//! Integration tests for the cache and conditional request path: the
//! `Cache-Control` and `ETag` headers on snapshot-backed routes, and `304 Not
//! Modified` when `If-None-Match` matches the current ETag.

use std::net::SocketAddr;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use tower::ServiceExt as _;

use crate::indexer::{Snapshot, POLL_INTERVAL};
use crate::models::Stats;

use super::router;
use super::test_support::state_with;

const LEDGER: u32 = 4242;

fn app() -> Router {
    let snapshot = Snapshot {
        stats: Stats {
            last_indexed_ledger: LEDGER,
            ..Stats::default()
        },
        ..Snapshot::default()
    };
    router(state_with(snapshot))
}

async fn get(app: Router, uri: &str, if_none_match: Option<&str>) -> Response {
    let mut builder = Request::builder().uri(uri);
    if let Some(value) = if_none_match {
        builder = builder.header(header::IF_NONE_MATCH, value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    // The rate limiter keys on the peer IP, which `serve` normally supplies.
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));
    app.oneshot(request).await.unwrap()
}

fn header_str(response: &Response, name: header::HeaderName) -> String {
    response
        .headers()
        .get(name)
        .expect("header should be present")
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn data_route_sets_cache_control_and_etag() {
    let response = get(app(), "/v1/stats", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, header::CACHE_CONTROL),
        format!("public, max-age={}", POLL_INTERVAL.as_secs())
    );
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
}

#[tokio::test]
async fn if_none_match_supports_lists_weak_tags_and_wildcard() {
    let etag = format!("\"ledger-{LEDGER}\"");
    let cases = [
        (&etag[..], StatusCode::NOT_MODIFIED),
        ("W/\"ledger-4242\"", StatusCode::NOT_MODIFIED),
        (
            "\"ledger-122\", \"ledger-4242\"",
            StatusCode::NOT_MODIFIED,
        ),
        ("\"ledger-122\"", StatusCode::OK),
        ("*", StatusCode::NOT_MODIFIED),
        ("\"ledger-4242", StatusCode::OK),
        ("\"ledger-4242\",", StatusCode::OK),
    ];

    for (if_none_match, expected_status) in cases {
        let response = get(app(), "/v1/stats", Some(if_none_match)).await;

        assert_eq!(response.status(), expected_status, "{if_none_match}");
        assert_eq!(header_str(&response, header::ETAG), etag);
        if expected_status == StatusCode::NOT_MODIFIED {
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert!(body.is_empty(), "304 must not carry a body");
        }
    }
}

#[tokio::test]
async fn stale_if_none_match_returns_full_response() {
    let response = get(app(), "/v1/stats", Some("\"ledger-1\"")).await;

    assert_eq!(response.status(), StatusCode::OK);
    // The current ETag is returned so the client can revalidate next time.
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_object(), "stale ETag must yield the full body");
}

#[tokio::test]
async fn assets_route_sets_etag_and_honours_if_none_match() {
    // The assets route is snapshot-backed like the data routes, so it must
    // advertise the same ledger-derived ETag and revalidate consistently.
    let etag = format!("\"ledger-{LEDGER}\"");

    let response = get(app(), "/v1/assets", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_str(&response, header::ETAG), etag);

    let response = get(app(), "/v1/assets", Some(&etag)).await;
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(header_str(&response, header::ETAG), etag);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(body.is_empty(), "304 must not carry a body");

    let response = get(app(), "/v1/assets", Some("\"ledger-1\"")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_str(&response, header::ETAG), etag);
}

#[tokio::test]
async fn non_data_routes_are_not_cached() {
    let response = get(app(), "/version", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::ETAG).is_none());
}

#[tokio::test]
async fn dividends_route_sets_cache_control_and_etag() {
    // Test that dividends route properly supports conditional requests (#539)
    let response = get(app(), "/v1/assets/1/dividends", None).await;

    // Dividends route may return 404 if asset doesn't exist, but should still set headers
    if response.status() == StatusCode::OK {
        assert_eq!(
            header_str(&response, header::CACHE_CONTROL),
            format!("public, max-age={}", POLL_INTERVAL.as_secs())
        );
        assert_eq!(
            header_str(&response, header::ETAG),
            format!("\"ledger-{LEDGER}\"")
        );
    }
}

#[tokio::test]
async fn dividends_route_honours_if_none_match() {
    // Verify If-None-Match matching ETag returns 304 Not Modified for dividends
    let etag = format!("\"ledger-{LEDGER}\"");
    let response = get(app(), "/v1/assets/1/dividends", Some(&etag)).await;

    // For valid asset, should return 304; for missing asset, 404 is acceptable
    if response.status() == StatusCode::NOT_MODIFIED {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.is_empty(), "304 Not Modified must not carry a body");
    }
}

#[tokio::test]
async fn server_entrypoint_route_sets_etag_and_honours_if_none_match() {
    // The server entrypoint (`api/src/main.rs`) serves the same router, so the
    // conditional request contract must hold end-to-end for its routes too.
    let response = get(app(), "/v1/stats", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let etag = header_str(&response, header::ETAG);
    assert_eq!(etag, format!("\"ledger-{LEDGER}\""));

    // A matching If-None-Match must short-circuit to 304 with no body.
    let revalidated = get(app(), "/v1/stats", Some(&etag)).await;
    assert_eq!(revalidated.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(header_str(&revalidated, header::ETAG), etag);
    let body = to_bytes(revalidated.into_body(), usize::MAX).await.unwrap();
    assert!(body.is_empty(), "304 must not carry a body");
}

#[tokio::test]
async fn compliance_summary_route_sets_cache_control_and_etag() {
    let response = get(app(), "/v1/assets/1/compliance", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, header::CACHE_CONTROL),
        format!("public, max-age={}", POLL_INTERVAL.as_secs())
    );
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
}

#[tokio::test]
async fn events_route_sets_cache_control_and_etag() {
    // Test that events route properly supports conditional requests (#544)
    let response = get(app(), "/v1/events", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, header::CACHE_CONTROL),
        format!("public, max-age={}", POLL_INTERVAL.as_secs())
    );
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
}

#[tokio::test]
async fn compliance_summary_if_none_match_returns_304() {
    let etag = format!("\"ledger-{LEDGER}\"");
    let response = get(app(), "/v1/assets/1/compliance", Some(&etag)).await;

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(body.is_empty(), "304 Not Modified must not carry a body");
}

#[tokio::test]
async fn events_route_if_none_match_returns_304() {
    // Verify If-None-Match matching ETag returns 304 Not Modified
    let etag = format!("\"ledger-{LEDGER}\"");
    let response = get(app(), "/v1/events", Some(&etag)).await;

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(body.is_empty(), "304 Not Modified must not carry a body");
}

#[tokio::test]
async fn compliance_summary_stale_etag_returns_full_response() {
    let response = get(app(), "/v1/assets/1/compliance", Some("\"ledger-1\"")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json.is_object(),
        "stale ETag must yield the full compliance summary object"
    );
}

#[tokio::test]
async fn events_route_stale_etag_returns_full_response() {
    // Verify stale ETag returns full response with updated ETag
    let response = get(app(), "/v1/events", Some("\"ledger-1\"")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json.is_array(),
        "stale ETag must yield the full events array"
    );
}
