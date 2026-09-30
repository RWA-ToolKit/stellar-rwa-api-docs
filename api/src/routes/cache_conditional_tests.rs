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
async fn non_data_routes_are_not_cached() {
    let response = get(app(), "/version", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::ETAG).is_none());
}
