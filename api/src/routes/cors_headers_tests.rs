//! CORS tests for the caching / conditional-request contract (issue #466).
//!
//! The docs (`docs/app/docs/api/rate-limits/page.mdx`) show a browser helper
//! that reads `res.headers.get("ETag")`, sends `If-None-Match` and honours
//! `Retry-After` on `429`. Browsers only expose CORS-safelisted response
//! headers unless the server opts in via `Access-Control-Expose-Headers`, and a
//! script-set `If-None-Match` triggers an `OPTIONS` preflight that must be
//! answered with `if-none-match` in `Access-Control-Allow-Headers`.
//!
//! These tests drive the real router with a cross-origin `Origin` and a real
//! preflight, asserting exactly what the browser would see.

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Method, Request, StatusCode},
    response::Response,
    Router,
};
use tower::ServiceExt as _;

use crate::indexer::{Snapshot, POLL_INTERVAL};
use crate::models::Stats;

use super::router;
use super::test_support::state_with;
use super::DEFAULT_CORS_ORIGIN;

const LEDGER: u32 = 4242;
const ORIGIN: &str = DEFAULT_CORS_ORIGIN;

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

async fn send(app: Router, request: Request<Body>) -> Response {
    let mut request = request;
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
        .to_ascii_lowercase()
}

fn header_present(response: &Response, name: header::HeaderName) -> bool {
    response.headers().contains_key(name)
}

/// A cross-origin conditional `GET`, i.e. what the documented helper does
/// after it has stored an ETag.
async fn cross_origin_get(if_none_match: Option<&str>) -> Response {
    let mut builder = Request::builder()
        .method(Method::GET)
        .uri("/v1/stats")
        .header(header::ORIGIN, ORIGIN);
    if let Some(value) = if_none_match {
        builder = builder.header(header::IF_NONE_MATCH, value);
    }
    send(app(), builder.body(Body::empty()).unwrap()).await
}

/// The preflight a browser sends before a request carrying non-safelisted
/// headers such as `If-None-Match`.
async fn preflight(requested_method: &str, requested_headers: &str) -> Response {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/stats")
        .header(header::ORIGIN, ORIGIN)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, requested_method)
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, requested_headers)
        .body(Body::empty())
        .unwrap();
    send(app(), request).await
}

#[tokio::test]
async fn cross_origin_response_exposes_etag_retry_after_and_cache_control() {
    let response = cross_origin_get(None).await;

    assert_eq!(response.status(), StatusCode::OK);

    let exposed = header_str(&response, header::ACCESS_CONTROL_EXPOSE_HEADERS);
    for expected in ["etag", "retry-after", "cache-control"] {
        assert!(
            exposed.split(',').any(|value| value.trim() == expected),
            "Access-Control-Expose-Headers must contain {expected}, got {exposed:?}"
        );
    }

    // The headers themselves are still present; exposing them only makes them
    // readable from JavaScript.
    assert_eq!(
        header_str(&response, header::ETAG),
        format!("\"ledger-{LEDGER}\"")
    );
    assert_eq!(
        header_str(&response, header::CACHE_CONTROL),
        format!("public, max-age={}", POLL_INTERVAL.as_secs())
    );
}

#[tokio::test]
async fn preflight_allows_if_none_match() {
    let response = preflight("GET", "if-none-match").await;

    assert_eq!(response.status(), StatusCode::OK);

    let allowed = header_str(&response, header::ACCESS_CONTROL_ALLOW_HEADERS);
    assert!(
        allowed
            .split(',')
            .any(|value| value.trim() == "if-none-match"),
        "Access-Control-Allow-Headers must contain if-none-match, got {allowed:?}"
    );
    // Content-Type stays allowed so the pre-existing behaviour is preserved.
    assert!(
        allowed
            .split(',')
            .any(|value| value.trim() == "content-type"),
        "Access-Control-Allow-Headers must keep content-type, got {allowed:?}"
    );
    assert_eq!(
        header_str(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN),
        ORIGIN
    );
    assert_eq!(
        header_str(&response, header::ACCESS_CONTROL_ALLOW_METHODS),
        "get"
    );
}

#[tokio::test]
async fn preflight_content_type_still_allowed() {
    let response = preflight("GET", "content-type").await;

    assert_eq!(response.status(), StatusCode::OK);
    let allowed = header_str(&response, header::ACCESS_CONTROL_ALLOW_HEADERS);
    assert!(
        allowed
            .split(',')
            .any(|value| value.trim() == "content-type"),
        "Access-Control-Allow-Headers must keep content-type, got {allowed:?}"
    );
}

#[tokio::test]
async fn cross_origin_conditional_request_reaches_304() {
    let etag = format!("\"ledger-{LEDGER}\"");

    let fresh = cross_origin_get(Some(&etag)).await;
    assert_eq!(fresh.status(), StatusCode::NOT_MODIFIED);
    // A 304 still carries the ETag, and it must stay exposed.
    assert!(header_present(&fresh, header::ETAG));
    assert!(header_present(
        &fresh,
        header::ACCESS_CONTROL_EXPOSE_HEADERS
    ));
}

#[tokio::test]
async fn rate_limited_response_exposes_retry_after() {
    // The documented retry loop reads `Retry-After` from the 429 itself, so
    // the header must be exposed on error responses too. Drive the real
    // limiter by exhausting the burst for a single peer IP.
    let app = app();
    let peer = ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321)));
    let mut limited = None;

    for _ in 0..64 {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/version")
            .header(header::ORIGIN, ORIGIN)
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(peer);
        let response = app.clone().oneshot(request).await.unwrap();
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(response);
            break;
        }
    }

    let response = limited.expect("rate limiter should reject a flood from one IP");
    let exposed = header_str(&response, header::ACCESS_CONTROL_EXPOSE_HEADERS);
    assert!(
        exposed
            .split(',')
            .any(|value| value.trim() == "retry-after"),
        "Access-Control-Expose-Headers must contain retry-after, got {exposed:?}"
    );
    assert!(header_present(&response, header::RETRY_AFTER));
}
