//! Request correlation: assigns each request an id, logs it with the
//! indexer state, returns it in `X-Request-Id`, and adds it to JSON error bodies.

use std::time::Instant;

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;

use crate::indexer::AppState;

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
const MAX_ERROR_BODY: usize = 64 * 1024;

fn new_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

/// Reuse a well-formed inbound `X-Request-Id`, otherwise generate one.
fn resolve_id(req: &Request) -> String {
    req.headers()
        .get(&REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 64 && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        .map(str::to_owned)
        .unwrap_or_else(new_id)
}

pub async fn layer(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let id = resolve_id(&req);
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let span = tracing::info_span!("request", request_id = %id, %method, %path);
    let started = Instant::now();

    let mut resp = next.run(req).instrument(span.clone()).await;
    let status = resp.status();

    if (status.is_client_error() || status.is_server_error())
        && resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("application/json"))
    {
        let (mut parts, body) = resp.into_parts();
        match to_bytes(body, MAX_ERROR_BODY).await {
            Ok(bytes) => {
                let out = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Ok(serde_json::Value::Object(mut map)) => {
                        map.insert("request_id".into(), serde_json::Value::String(id.clone()));
                        serde_json::to_vec(&map).unwrap_or_else(|_| bytes.to_vec())
                    }
                    _ => bytes.to_vec(),
                };
                parts.headers.remove(header::CONTENT_LENGTH);
                resp = Response::from_parts(parts, Body::from(out));
            }
            Err(_) => resp = Response::from_parts(parts, Body::empty()),
        }
    }

    if let Ok(v) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert(REQUEST_ID, v);
    }

    span.in_scope(|| {
        tracing::info!(
            status = status.as_u16(),
            latency_ms = started.elapsed().as_millis() as u64,
            indexed_ledger = state.last_indexed_ledger(),
            consecutive_failures = crate::poll_status::consecutive_failures(),
            "request completed"
        );
    });
    resp
}
