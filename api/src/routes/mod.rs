//! HTTP routing and the shared API error type.

pub mod assets;
pub mod compliance;
pub mod dividends;
pub mod holders;
pub mod stats;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    BoxError, Json, Router,
};
use serde_json::json;
use tower::{
    limit::ConcurrencyLimitLayer, load_shed::LoadShedLayer, timeout::TimeoutLayer, ServiceBuilder,
};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};

use crate::indexer::{AppState, Snapshot, POLL_INTERVAL};
use crate::models::ApiErrorBody;

/// Sustained requests-per-second allowed per client IP, with bursting.
const RATE_LIMIT_PER_SECOND: u64 = 5;
const RATE_LIMIT_BURST: u32 = 20;

/// Hard ceiling on how long a single request may take before it's aborted.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Max requests handled concurrently across the whole server. Once this many
/// are in flight, `LoadShedLayer` rejects new requests with 503 immediately
/// instead of queuing them, so a burst of slow or abusive clients can't pile
/// up unbounded work.
const MAX_CONCURRENT_REQUESTS: usize = 256;
/// No route accepts a request body; cap it small rather than unbounded.
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024;

/// Errors surfaced to API clients as a JSON body with an appropriate status.
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    /// The indexer hasn't produced a usable snapshot (yet), or a downstream
    /// dependency is temporarily unavailable. Clients should retry later.
    ServiceUnavailable(String),
    /// An unexpected, non-retryable server-side failure.
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable", msg)
            }
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", msg),
        };
        (
            status,
            Json(ApiErrorBody {
                error: error.to_string(),
                message,
            }),
        )
            .into_response()
    }
}

/// Fail fast with 503 if the indexer hasn't completed its first successful
/// refresh yet, rather than letting routes serve an empty/zero-value
/// snapshot as if it were real data.
fn require_ready(snap: &Snapshot) -> Result<(), ApiError> {
    if snap.is_ready() {
        Ok(())
    } else {
        Err(ApiError::ServiceUnavailable(
            "indexer has not completed an initial refresh yet".into(),
        ))
    }
}

/// Build the application router with CORS enabled for the docs/web app.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Each request clones the in-memory snapshot, so cap how fast a single
    // client can drive that cost. Checked before `cache_headers`, which
    // itself touches shared state, so a throttled request stays cheap.
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(RATE_LIMIT_PER_SECOND)
            .burst_size(RATE_LIMIT_BURST)
            .finish()
            .expect("rate limit config: period and burst size are non-zero"),
    );

    // Snapshot-backed endpoints: cacheable and safe to answer with 304 when
    // the client's ETag still matches the last indexed ledger.
    let data_routes = Router::new()
        .route("/stats", get(stats::get))
        .route("/assets", get(assets::list))
        .route("/assets/:id", get(assets::detail))
        .route("/assets/:id/holders", get(holders::list))
        .route("/assets/:id/compliance", get(compliance::summary))
        .route("/assets/:id/dividends", get(dividends::list))
        .layer(middleware::from_fn_with_state(state.clone(), cache_headers))
        .layer(GovernorLayer {
            config: governor_conf,
        });

    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .merge(data_routes)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_middleware_error))
                .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
                .layer(LoadShedLayer::new())
                .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
                .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
                .layer(cors),
        )
}

/// Converts errors from the timeout/load-shed layers into the same
/// structured JSON body the rest of the API uses, rather than letting tower
/// terminate the connection or return a bare status code.
async fn handle_middleware_error(err: BoxError) -> ApiError {
    if err.is::<tower::timeout::error::Elapsed>() {
        ApiError::ServiceUnavailable("request exceeded the timeout".into())
    } else if err.is::<tower::load_shed::error::Overloaded>() {
        ApiError::ServiceUnavailable("server is at capacity; try again shortly".into())
    } else {
        ApiError::Internal(format!("unhandled middleware error: {err}"))
    }
}

/// Attach `Cache-Control` and `ETag` to snapshot-backed responses, and answer
/// `If-None-Match` with `304 Not Modified` when the snapshot hasn't advanced.
///
/// The snapshot only changes once per [`POLL_INTERVAL`], so the ETag is
/// derived from `last_indexed_ledger`: two requests against the same indexed
/// ledger are guaranteed to have identical bodies.
async fn cache_headers(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let ledger = state.last_indexed_ledger();
    let etag = format!("\"ledger-{ledger}\"");

    let fresh = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == etag);

    let mut resp = if fresh {
        Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .expect("static 304 response is well-formed")
    } else {
        next.run(req).await
    };

    insert_cache_headers(resp.headers_mut(), &etag);
    resp
}

fn insert_cache_headers(headers: &mut HeaderMap, etag: &str) {
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&format!("public, max-age={}", POLL_INTERVAL.as_secs()))
            .expect("max-age value is a valid header value"),
    );
    if let Ok(v) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, v);
    }
}

/// Root — a small self-describing index of the available endpoints.
async fn index() -> Json<serde_json::Value> {
    Json(json!({
        "name": "Stellar RWA API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Read-only index of tokenized real-world asset activity on Stellar.",
        "endpoints": [
            "GET /stats",
            "GET /assets",
            "GET /assets/:id",
            "GET /assets/:id/holders",
            "GET /assets/:id/compliance",
            "GET /assets/:id/dividends",
            "GET /health",
            "GET /metrics"
        ],
        "docs": "https://github.com/your-org/stellar-rwa-api-docs"
    }))
}

/// Liveness probe.
async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Prometheus scrape endpoint: indexer refresh latency, failure counts, last
/// success timestamp, and per-asset read errors.
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
}
