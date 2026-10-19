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
    async fn metrics_auth_failure_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        let response = get(&app, "/metrics", "GET").await;
        assert_error_shape(response, StatusCode::UNAUTHORIZED, "unauthorized").await;
    }

    #[tokio::test]
    async fn indexer_rate_limit_boundary_uses_shared_shape() {
        let app = router(AppState::for_test_empty());
        // Drive the indexer route up to its configured rate-limit boundary.
        // Requests within the limit succeed; the first request past the
        // boundary is rejected with 429 and must use the shared error body.
        let mut limited: Option<Response> = None;
        for _ in 0..=crate::indexer::TEST_RATE_LIMIT {
            let response = get(&app, "/v1/indexer", "GET").await;
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                limited = Some(response);
                break;
            }
        }
        let response = limited.expect("indexer should hit its rate-limit boundary");
        assert_error_shape(response, StatusCode::TOO_MANY_REQUESTS, "too_many_requests").await;
    }
}
