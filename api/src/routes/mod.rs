//! HTTP routing and the shared API error type.
//!
//! # Multi-network design decision (issue #433)
//!
//! ## Context
//! The question was raised whether a single deployment should serve multiple
//! Stellar networks (e.g. Testnet + Mainnet) from one API process.
//!
//! ## Options considered
//!
//! **Option A — namespace routes under `/v1/networks/{network}/...`**
//! Each network becomes a path segment: `/v1/networks/testnet/assets`,
//! `/v1/networks/mainnet/assets`, etc. A single `AppState` map keyed by
//! network name drives all reads.
//!
//! *Tradeoffs:*
//! - Doubles URL length and breaks every existing client without a redirect.
//! - Adds a required path parameter to all data routes, complicating the
//!   common single-network case.
//! - A shared state map grows with every additional network.
//!
//! **Option B — one `AppState`/`Indexer` pair per network**
//! The router creates N `AppState` instances at startup (one per configured
//! network) and dispatches by leading path segment or a request header.
//! Each indexer task polls its own RPC endpoint independently.
//!
//! *Tradeoffs:*
//! - Requires multiplying indexer tasks and state; memory grows linearly
//!   with network count.
//! - Failures are fully isolated: a broken Testnet node cannot degrade
//!   Mainnet reads.
//! - `AppState` and `Indexer` are already `Clone`-friendly, so Option B is
//!   feasible without restructuring existing types.
//!
//! ## Decision: **Deferred — single-network is the v1 model**
//! A single-network deployment is the supported model for v1. Multi-network
//! support can be introduced as a breaking v2 change by nesting all data
//! routes under `/v2/networks/{network}/`. Backwards compatibility is
//! preserved by keeping `/v1` routes as-is; both versions can run in
//! parallel during any migration window. When multi-network is needed,
//! Option B is the preferred implementation path.

pub mod assets;
pub mod assets_query;
pub mod compliance;
pub mod dividends;
pub mod events;
pub mod field_select;
pub mod holder_position;
pub mod holders;
pub mod stats;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod assets_query_tests;
#[cfg(test)]
mod field_select_tests;
#[cfg(test)]
mod holder_position_tests;

#[cfg(test)]
mod cache_conditional_tests;

#[cfg(test)]
mod rate_limit_boundary_tests;

pub(crate) mod error_body;

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer};

use crate::indexer::{AppState, POLL_INTERVAL};
use crate::models::ApiErrorBody;

/// Sustained requests-per-second allowed per client IP, with bursting.
const RATE_LIMIT_PER_SECOND: u64 = 5;
const RATE_LIMIT_BURST: u32 = 20;
/// Global request timeout — acts as a hard ceiling for all routes including
/// non-data routes (health, metrics, version). Individual data-route groups
/// are given tighter per-route timeouts via `RWA_*_TIMEOUT_SECS` env vars.
const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Per-route timeout defaults (overridable via env vars documented on
/// `router_with_rate_limit`).
const STATS_TIMEOUT_SECS: u64 = 10;
const ASSETS_LIST_TIMEOUT_SECS: u64 = 30;
const ASSET_DETAIL_TIMEOUT_SECS: u64 = 15;
/// Timeout for aggregate endpoints (holders, compliance, dividends/distributions)
/// which legitimately take longer as they fan out over many addresses.
const AGGREGATE_TIMEOUT_SECS: u64 = 60;
const MAX_BODY_BYTES: usize = 1_048_576;
const DEFAULT_CORS_ORIGIN: &str = "http://localhost:3000";

fn env_value<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Read a per-route timeout from an env var, falling back to `default_secs`.
///
/// Used by [`router_with_rate_limit`] to apply individual timeouts to each
/// route group. The following env vars are supported (all in seconds):
///
/// | Env var                      | Default | Route group                                       |
/// |------------------------------|---------|---------------------------------------------------|
/// | `RWA_STATS_TIMEOUT_SECS`     | 10      | `GET /v1/stats`, `GET /v1/events`                 |
/// | `RWA_ASSETS_LIST_TIMEOUT_SECS` | 30    | `GET /v1/assets`                                  |
/// | `RWA_ASSET_DETAIL_TIMEOUT_SECS` | 15   | `GET /v1/assets/:id`                              |
/// | `RWA_AGGREGATE_TIMEOUT_SECS` | 60      | holders, compliance, dividends, distributions     |
/// | `RWA_REQUEST_TIMEOUT_SECS`   | 30      | global ceiling (non-data routes + safety net)     |
fn route_timeout(env_var: &str, default_secs: u64) -> Duration {
    Duration::from_secs(env_value(env_var, default_secs))
}

fn rate_limit_period(per_second: u64) -> Duration {
    if per_second == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs(1).div_f64(per_second as f64)
    }
}

/// Errors surfaced to API clients as a JSON body with an appropriate status.
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
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

/// Build the application router with CORS enabled for the docs/web app.
pub fn router(state: AppState) -> Router {
    router_with_rate_limit(
        state,
        env_value("RWA_RATE_LIMIT_PER_SECOND", RATE_LIMIT_PER_SECOND),
        env_value("RWA_RATE_LIMIT_BURST", RATE_LIMIT_BURST),
    )
}

/// [`router`] with the rate limit given explicitly instead of read from the
/// environment, so tests can pick limits without mutating process-wide env
/// vars that concurrently running tests would also observe.
///
/// Per-route timeouts can be tuned via environment variables. See
/// [`route_timeout`] for the full list. The global `RWA_REQUEST_TIMEOUT_SECS`
/// (default 30 s) remains as a hard ceiling for all routes.
pub(crate) fn router_with_rate_limit(state: AppState, per_second: u64, burst: u32) -> Router {
    let origins = std::env::var("RWA_CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| DEFAULT_CORS_ORIGIN.into())
        .split(',')
        .map(str::trim)
        .map(|origin| origin.parse::<HeaderValue>().expect("valid CORS origin"))
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET])
        .allow_headers([header::CONTENT_TYPE]);

    // Each request clones the in-memory snapshot, so cap how fast a single
    // client can drive that cost. Checked before `cache_headers`, which
    // itself touches shared state, so a throttled request stays cheap.
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .period(rate_limit_period(per_second))
            .burst_size(burst)
            .finish()
            .expect("rate limit config: period and burst size are non-zero"),
    );

    // Snapshot-backed endpoints: cacheable and safe to answer with 304 when
    // the client's ETag still matches the last indexed ledger.
    //
    // All data routes are nested under `/v1` so future breaking changes can
    // be introduced as `/v2` without disturbing existing clients.
    //
    // Each route group is given its own per-route TimeoutLayer so that slow
    // aggregate queries (holders/compliance/dividends) do not eat into the
    // budget of lightweight stats calls. The global TimeoutLayer applied to
    // the outer router acts as a hard ceiling for all routes.

    // Stats and events — lightweight reads that should be fast.
    let stats_routes = Router::new()
        .route("/stats", get(stats::get))
        .route("/events", get(events::list))
        .layer(TimeoutLayer::new(route_timeout(
            "RWA_STATS_TIMEOUT_SECS",
            STATS_TIMEOUT_SECS,
        )));

    // Asset list — potentially larger payload but no per-address fan-out.
    let assets_list_routes = Router::new()
        .route("/assets", get(assets_query::list))
        .layer(TimeoutLayer::new(route_timeout(
            "RWA_ASSETS_LIST_TIMEOUT_SECS",
            ASSETS_LIST_TIMEOUT_SECS,
        )));

    // Asset detail — single-asset read.
    let asset_detail_routes = Router::new()
        .route("/assets/:id", get(assets::detail))
        .layer(TimeoutLayer::new(route_timeout(
            "RWA_ASSET_DETAIL_TIMEOUT_SECS",
            ASSET_DETAIL_TIMEOUT_SECS,
        )));

    // Aggregate endpoints — fan out over many addresses, legitimately slower.
    let aggregate_routes = Router::new()
        .route("/assets/:id/holders", get(holders::list))
        .route("/assets/:id/compliance", get(compliance::summary))
        .route("/assets/:id/dividends", get(dividends::list))
        .route("/assets/:id/distributions/:did", get(dividends::get_one))
        .route("/holders/:address", get(holders::by_address))
        .route(
            "/holders/:address/compliance",
            get(holders::by_address_compliance),
        )
        .route("/holders/:address/position", get(holder_position::get))
        .route("/compliance/:address", get(compliance::for_address))
        .layer(TimeoutLayer::new(route_timeout(
            "RWA_AGGREGATE_TIMEOUT_SECS",
            AGGREGATE_TIMEOUT_SECS,
        )));

    let data_routes = Router::new()
        .merge(stats_routes)
        .merge(assets_list_routes)
        .merge(asset_detail_routes)
        .merge(aggregate_routes)
        .layer(middleware::from_fn(field_select::field_select))
        .layer(middleware::from_fn_with_state(state.clone(), cache_headers));

    Router::new()
        .route("/", get(index))
        .route("/version", get(version))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/poll-history", get(poll_history))
        .nest("/v1", data_routes)
        .with_state(state)
        .layer(middleware::from_fn(crate::stale_guard::stale_headers))
        .layer(TimeoutLayer::new(Duration::from_secs(env_value(
            "RWA_REQUEST_TIMEOUT_SECS",
            REQUEST_TIMEOUT_SECS,
        ))))
        .layer(RequestBodyLimitLayer::new(env_value(
            "RWA_MAX_BODY_BYTES",
            MAX_BODY_BYTES,
        )))
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(middleware::from_fn(error_body::normalize))
        .layer(cors)
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
        .is_some_and(|value| if_none_match_matches(value, &etag));

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

fn if_none_match_matches(value: &str, etag: &str) -> bool {
    let bytes = value.as_bytes();
    let mut position = 0;
    skip_ows(bytes, &mut position);
    if bytes.get(position) == Some(&b'*') {
        position += 1;
        skip_ows(bytes, &mut position);
        return position == bytes.len();
    }

    let mut matched = false;
    loop {
        skip_ows(bytes, &mut position);
        if bytes.get(position..position + 2) == Some(b"W/") {
            position += 2;
        }
        if bytes.get(position) != Some(&b'"') {
            return false;
        }
        let tag_start = position;
        position += 1;
        while let Some(byte) = bytes.get(position) {
            if *byte == b'"' {
                break;
            }
            if !(*byte == 0x21 || (0x23..=0x7e).contains(byte) || *byte >= 0x80) {
                return false;
            }
            position += 1;
        }
        if bytes.get(position) != Some(&b'"') {
            return false;
        }
        position += 1;
        if bytes.get(tag_start..position) == Some(etag.as_bytes()) {
            matched = true;
        }
        skip_ows(bytes, &mut position);
        if position == bytes.len() {
            return matched;
        }
        if bytes.get(position) != Some(&b',') {
            return false;
        }
        position += 1;
        if position == bytes.len() {
            return false;
        }
    }
}

fn skip_ows(bytes: &[u8], position: &mut usize) {
    while matches!(bytes.get(*position), Some(b' ' | b'\t')) {
        *position += 1;
    }
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
            "GET /version",
            "GET /v1/stats",
            "GET /v1/events",
            "GET /v1/assets",
            "GET /v1/assets/:id",
            "GET /v1/assets/:id/holders",
            "GET /v1/assets/:id/compliance",
            "GET /v1/assets/:id/dividends",
            "GET /v1/assets/:id/distributions/:did",
            "GET /v1/holders/:address",
            "GET /v1/holders/:address/compliance",
            "GET /v1/holders/:address/position",
            "GET /v1/compliance/:address",
            "GET /health/live",
            "GET /health/ready",
            "GET /health",
            "GET /metrics",
            "GET /poll-history"
        ],
        "docs": "https://github.com/your-org/stellar-rwa-api-docs"
    }))
}

/// Machine-readable version endpoint.
///
/// Returns the crate version from `Cargo.toml` and the API release label so
/// clients can negotiate compatibility without parsing the root index body.
async fn version() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "release": concat!("v", env!("CARGO_PKG_VERSION")),
    }))
}

/// Liveness probe — always returns 200 if the process is running.
async fn liveness() -> Json<serde_json::Value> {
    Json(json!({
        "status": "live"
    }))
}

/// Readiness probe — returns 200 only when at least one successful snapshot
/// poll has completed. Returns 503 while waiting for the first poll to finish.
async fn readiness() -> Response {
    let has_completed_poll = crate::poll_status::last_poll_age_seconds().is_some();
    let status = if has_completed_poll {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = if has_completed_poll {
        json!({ "status": "ready" })
    } else {
        json!({
            "status": "not-ready",
            "reason": "no-snapshot-yet"
        })
    };
    (status, Json(body)).into_response()
}

/// Combined health check with detailed status.
async fn health(State(state): State<AppState>) -> Response {
    let updated = state
        .snapshot()
        .stats
        .last_updated
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok());
    let age = updated.map(|time| {
        (chrono::Utc::now() - time.with_timezone(&chrono::Utc))
            .num_seconds()
            .max(0)
    });
    let max_age = crate::poll_status::max_poll_age(POLL_INTERVAL);
    let poll_age = crate::poll_status::last_poll_age_seconds();
    let healthy = age.is_some_and(|seconds| seconds <= max_age)
        && poll_age.is_some_and(|seconds| seconds <= max_age);
    let status = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if healthy { "ok" } else { "degraded" },
            "snapshot_age_seconds": age,
            "max_age_seconds": max_age,
            "consecutive_failures": crate::poll_status::consecutive_failures(),
            "last_poll_at": crate::poll_status::last_poll_at(),
            "last_poll_age_seconds": poll_age,
            "last_indexed_ledger": crate::poll_status::last_ledger(),
            "ledger_lag": crate::poll_status::estimated_ledger_lag(),
        })),
    )
        .into_response()
}

/// Prometheus scrape endpoint: indexer refresh latency, failure counts, last
/// success timestamp, and per-asset read errors.
async fn metrics(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let expected = std::env::var("RWA_METRICS_TOKEN").ok();
    let authorized = expected
        .as_deref()
        .filter(|token| !token.is_empty())
        .zip(supplied)
        .is_some_and(|(expected, supplied)| expected == supplied);
    if !authorized {
        return (StatusCode::UNAUTHORIZED, "metrics authentication required").into_response();
    }
    crate::indexer_metrics::refresh_scrape_gauges();
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
        .into_response()
}

/// Operator endpoint: returns the bounded ring-buffer of the last
/// [`crate::indexer::MAX_POLL_HISTORY`] indexer poll records as JSON.
///
/// Protected by the same bearer token as `/metrics` (`RWA_METRICS_TOKEN`).
/// Returns `401 Unauthorized` when the token is configured but missing or
/// wrong; when `RWA_METRICS_TOKEN` is not set, the endpoint is open.
async fn poll_history(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let expected = std::env::var("RWA_METRICS_TOKEN").ok();
    let authorized = expected
        .as_deref()
        .filter(|token| !token.is_empty())
        .zip(supplied)
        .is_some_and(|(expected, supplied)| expected == supplied);
    if !authorized {
        return (StatusCode::UNAUTHORIZED, "poll-history authentication required").into_response();
    }
    Json(state.poll_history_records()).into_response()
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{header, Request, StatusCode},
        Router,
    };
    use tower::ServiceExt as _;

    use crate::indexer::AppState;

    use super::{rate_limit_period, router};

    #[test]
    fn rate_limit_setting_is_requests_per_second() {
        assert_eq!(
            rate_limit_period(5),
            std::time::Duration::from_millis(200)
        );
        assert_eq!(rate_limit_period(0), std::time::Duration::ZERO);
    }

    async fn assert_json_content_type(app: Router, uri: &str, status: StatusCode) {
        // This is the only test that exercises the full `router()`, rate
        // limiter included. The governor keys on the peer IP, which `serve`
        // supplies via into_make_service_with_connect_info; without it here
        // the extractor fails and every route answers 500.
        let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), status, "unexpected status for {uri}");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("JSON responses should set a content type");
        assert!(
            content_type
                .to_str()
                .expect("content type is UTF-8")
                .starts_with("application/json"),
            "expected application/json for {uri}, got {content_type:?}"
        );
    }

    #[tokio::test]
    async fn json_responses_set_application_json_content_type() {
        let app = router(AppState::for_test_empty());

        assert_json_content_type(app.clone(), "/", StatusCode::OK).await;
        assert_json_content_type(app.clone(), "/health/live", StatusCode::OK).await;
        assert_json_content_type(app.clone(), "/health/ready", StatusCode::SERVICE_UNAVAILABLE).await;
        assert_json_content_type(app.clone(), "/health", StatusCode::SERVICE_UNAVAILABLE).await;
        assert_json_content_type(app.clone(), "/version", StatusCode::OK).await;
        assert_json_content_type(app.clone(), "/v1/stats", StatusCode::OK).await;
        assert_json_content_type(app.clone(), "/v1/assets", StatusCode::OK).await;
        assert_json_content_type(app, "/v1/assets/99999", StatusCode::NOT_FOUND).await;
    }
}
