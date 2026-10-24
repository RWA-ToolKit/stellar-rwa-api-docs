//! One error body for every route.
//!
//! Handlers return [`ApiError`](super::ApiError), which renders the shared
//! [`ApiErrorBody`] (`{"error": <code>, "message": <text>}`). Errors produced
//! outside handlers (router 404/405, query-string rejections, the request
//! timeout, the body limit, the rate limiter and the metrics auth check) are
//! plain-text or empty; [`normalize`] rewrites them into the same shape.

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::models::ApiErrorBody;

/// Longest upstream plain-text message we will forward to the client.
const MAX_MESSAGE_BYTES: usize = 1024;

/// Machine-readable `error` code for a status.
pub(crate) fn code_for(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::REQUEST_TIMEOUT => "request_timeout",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::TOO_MANY_REQUESTS => "too_many_requests",
        StatusCode::SERVICE_UNAVAILABLE => "service_unavailable",
        s if s.is_server_error() => "internal_server_error",
        _ => "bad_request",
    }
}

fn is_json(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"))
}

/// Rewrite any non-JSON `4xx`/`5xx` response into the shared error body,
/// keeping the status and other headers (e.g. `Retry-After`).
pub(crate) async fn normalize(req: Request<Body>, next: Next) -> Response {
    let response = next.run(req).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) || is_json(&response) {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let bytes = to_bytes(body, MAX_MESSAGE_BYTES).await.unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    let message = if text.is_empty() {
        status
            .canonical_reason()
            .unwrap_or("request failed")
            .to_string()
    } else {
        text
    };

    let mut rewritten = (
        status,
        Json(ApiErrorBody {
            error: code_for(status).to_string(),
            message,
        }),
    )
        .into_response();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.remove(header::CONTENT_TYPE);
    for (name, value) in parts.headers.iter() {
        rewritten.headers_mut().insert(name.clone(), value.clone());
    }
    if rewritten.headers().get(header::CONTENT_TYPE).is_none() {
        rewritten.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    rewritten
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::{
        body::to_bytes,
        extract::ConnectInfo,
        http::{header, Request, StatusCode},
        response::Response,
        Router,
    };
    use tower::ServiceExt as _;

    use crate::indexer::AppState;
    use crate::models::ApiErrorBody;
    use crate::routes::router;

    async fn get(app: &Router, uri: &str, method: &str) -> Response {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));
        app.clone().oneshot(request).await.unwrap()
    }

    async fn assert_error_shape(response: Response, status: StatusCode, code: &str) {
        assert_eq!(response.status(), status);
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(content_type.to_str().unwrap().starts_with("application/json"));
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 2, "exactly error+message");
        let body: ApiErrorBody = serde_json::from_value(value).unwrap();
        assert_eq!(body.error, code);
        assert!(!body.message.is_empty());
    }

    #[tokio::test]
    async fn handler_not_found_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        let response = get(&app, "/v1/assets/99999", "GET").await;
        assert_error_shape(response, StatusCode::NOT_FOUND, "not_found").await;
    }

    #[tokio::test]
    async fn unknown_route_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        let response = get(&app, "/no/such/route", "GET").await;
        assert_error_shape(response, StatusCode::NOT_FOUND, "not_found").await;
    }

    #[tokio::test]
    async fn wrong_method_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        let response = get(&app, "/v1/stats", "POST").await;
        assert_error_shape(response, StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed").await;
    }

    #[tokio::test]
    async fn bad_query_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        // The assets list validates its own query parameters and reports
        // them as `invalid_parameter` (documented in the assets-query page).
        let response = get(&app, "/v1/assets?limit=abc", "GET").await;
        assert_error_shape(response, StatusCode::BAD_REQUEST, "invalid_parameter").await;
    }

    #[tokio::test]
    async fn assets_malformed_query_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        // Malformed and out-of-range query parameters must be rejected with a
        // clear 4xx (never a 500 or panic) and rendered in the shared error
        // body shape. Each case exercises a different validation path on the
        // assets route.
        let cases = [
            "/v1/assets?limit=abc",
            "/v1/assets?limit=-1",
            "/v1/assets?limit=0",
            "/v1/assets?limit=999999999999999999999",
            "/v1/assets?offset=notanumber",
            "/v1/assets?offset=-5",
            "/v1/assets?limit=abc&offset=xyz",
        ];
        for uri in cases {
            let response = get(&app, uri, "GET").await;
            let status = response.status();
            assert!(
                status.is_client_error(),
                "{uri} should return a 4xx, got {status}"
            );
            assert_error_shape(response, status, "invalid_parameter").await;
        }
    }

    #[tokio::test]
    async fn metrics_auth_failure_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        let response = get(&app, "/metrics", "GET").await;
        assert_error_shape(response, StatusCode::UNAUTHORIZED, "unauthorized").await;
    }

    #[tokio::test]
    async fn malformed_query_parameters_use_shared_shape() {
        let app = router(AppState::for_test_empty());
        // Each of these is rejected by query validation (or the router's
        // query-string rejection) and must surface as a clear 4xx in the
        // shared error body, never a 500 or a panic.
        let cases = [
            "/v1/assets?limit=abc",
            "/v1/assets?limit=999999",
            "/v1/assets?offset=-1",
            "/v1/holders/abc/position",
        ];
        for uri in cases {
            let response = get(&app, uri, "GET").await;
            let status = response.status();
            assert!(
                status.is_client_error(),
                "{uri} returned {status}, expected a 4xx"
            );
            assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR, "{uri}");
            let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
            assert!(
                content_type.to_str().unwrap().starts_with("application/json"),
                "{uri} did not return JSON"
            );
            let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                value.as_object().unwrap().len(),
                2,
                "{uri} body was not exactly error+message"
            );
            let body: ApiErrorBody = serde_json::from_value(value).unwrap();
            assert!(!body.error.is_empty(), "{uri} had an empty error code");
            assert!(!body.message.is_empty(), "{uri} had an empty message");
        }
    }

    #[tokio::test]
    async fn stats_rate_limit_boundary_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        // Drive the stats route up to and past the configured rate-limit
        // boundary. The limiter admits requests until the budget is spent and
        // then rejects with 429; we keep issuing requests until we observe the
        // first rejection so the test tracks the configured limit rather than
        // hard-coding it.
        let mut limited: Option<Response> = None;
        for _ in 0..10_000 {
            let response = get(&app, "/v1/stats", "GET").await;
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                limited = Some(response);
                break;
            }
            assert!(
                response.status().is_success(),
                "stats route should succeed before the rate-limit boundary, got {}",
                response.status()
            );
        }
        let response = limited.expect("stats route should hit the rate-limit boundary");
        assert_error_shape(response, StatusCode::TOO_MANY_REQUESTS, "too_many_requests").await;
    }
}
