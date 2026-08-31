//! HTTP routing and the shared API error type.

pub mod assets;
pub mod compliance;
pub mod dividends;
pub mod holders;
pub mod stats;

#[cfg(test)]
mod test_support;

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
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_BODY_BYTES: usize = 1_048_576;
const DEFAULT_CORS_ORIGIN: &str = "http://localhost:3000";

fn env_value<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
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

    let per_second = env_value("RWA_RATE_LIMIT_PER_SECOND", RATE_LIMIT_PER_SECOND);
    let burst = env_value("RWA_RATE_LIMIT_BURST", RATE_LIMIT_BURST);

    // Each request clones the in-memory snapshot, so cap how fast a single
    // client can drive that cost. Checked before `cache_headers`, which
    // itself touches shared state, so a throttled request stays cheap.
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .finish()
            .expect("rate limit config: period and burst size are non-zero"),
    );

    // Snapshot-backed endpoints: cacheable and safe to answer with 304 when
    // the client's ETag still matches the last indexed ledger.
    //
    // All data routes are nested under `/v1` so future breaking changes can
    // be introduced as `/v2` without disturbing existing clients.
    let data_routes = Router::new()
        .route("/stats", get(stats::get))
        .route("/assets", get(assets::list))
        .route("/assets/:id", get(assets::detail))
        .route("/assets/:id/holders", get(holders::list))
        .route("/assets/:id/compliance", get(compliance::summary))
        .route("/assets/:id/dividends", get(dividends::list))
        .route("/assets/:id/distributions/:did", get(dividends::get_one))
        .layer(middleware::from_fn_with_state(state.clone(), cache_headers));

    Router::new()
        .route("/", get(index))
        .route("/version", get(version))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .nest("/v1", data_routes)
        .with_state(state)
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
            "GET /version",
            "GET /v1/stats",
            "GET /v1/assets",
            "GET /v1/assets/:id",
            "GET /v1/assets/:id/holders",
            "GET /v1/assets/:id/compliance",
            "GET /v1/assets/:id/dividends",
            "GET /v1/assets/:id/distributions/:did",
            "GET /health",
            "GET /metrics"
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

/// Liveness probe.
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
    let max_age = (POLL_INTERVAL * 3).as_secs() as i64;
    let healthy = age.is_some_and(|seconds| seconds <= max_age);
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
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
        .into_response()
}

#[cfg(test)]
mod router_tests {
    //! Router-level tests for the defensive layers (issue #321, #320, #319):
    //! the request-body limit, the governor rate limiter (env overrides), and
    //! the CORS default/override.

    use std::net::SocketAddr;

    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{header, Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::router;
    use crate::indexer::Snapshot;
    use crate::routes::test_support::state_with;

    fn version_request(client_ip: SocketAddr) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri("/version")
            .extension(ConnectInfo(client_ip))
            .body(Body::empty())
            .expect("version request is well-formed")
    }

    /// Tests that mutate process-global env vars read by `router()` must not
    /// run concurrently — the vars are fixed (e.g. `RWA_RATE_LIMIT_*`), so a
    /// parallel test could observe another test's setting. All env-mutating
    /// tests hold this lock for their full duration.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Restore an env var to its prior value (or remove it) after the test.
    struct EnvGuard(
        &'static str,
        std::result::Result<String, std::env::VarError>,
    );
    impl EnvGuard {
        fn set(name: &'static str, value: &str) -> Self {
            let previous = std::env::var(name);
            std::env::set_var(name, value);
            EnvGuard(name, previous)
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.1 {
                Ok(v) => std::env::set_var(self.0, v),
                Err(_) => std::env::remove_var(self.0),
            }
        }
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// #321 — `RWA_MAX_BODY_BYTES` (default 1 MiB) is enforced: an oversized
    /// request body is rejected with 413 Payload Too Large before routing.
    #[tokio::test]
    async fn oversized_body_rejected_with_413() {
        // The env vars are only read when the router is built, so the lock and
        // the env guard live in a synchronous setup block (never across await).
        let app = {
            let _lock = env_lock();
            let _guard = EnvGuard::set("RWA_MAX_BODY_BYTES", "128");
            router(state_with(Snapshot::default()))
        };

        let oversized = vec![b'x'; 512];
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/version")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::CONTENT_LENGTH, oversized.len().to_string())
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 4100))))
                    .body(Body::from(oversized))
                    .expect("request is well-formed"),
            )
            .await
            .expect("router responds");

        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "oversized request body must be rejected with 413"
        );
    }

    /// Bodies under the limit still reach the handlers.
    #[tokio::test]
    async fn body_under_limit_served() {
        let app = {
            let _lock = env_lock();
            let _guard = EnvGuard::set("RWA_MAX_BODY_BYTES", "128");
            router(state_with(Snapshot::default()))
        };

        let response = app
            .oneshot(version_request(SocketAddr::from(([127, 0, 0, 1], 4101))))
            .await
            .expect("router responds");

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// #320 — the `RWA_RATE_LIMIT_PER_SECOND` / `RWA_RATE_LIMIT_BURST` env
    /// overrides actually change enforcement: with a burst of 1 the second
    /// immediate request from the same client is throttled with 429.
    #[tokio::test]
    async fn rate_limit_env_override_burst_of_one_rejects_immediate_second_request() {
        let app = {
            let _lock = env_lock();
            let _guard_per = EnvGuard::set("RWA_RATE_LIMIT_PER_SECOND", "1");
            let _guard_burst = EnvGuard::set("RWA_RATE_LIMIT_BURST", "1");
            router(state_with(Snapshot::default()))
        };
        let client = SocketAddr::from(([127, 0, 0, 1], 4001));

        let first = app
            .clone()
            .oneshot(version_request(client))
            .await
            .expect("router responds");
        assert_eq!(first.status(), StatusCode::OK, "first request passes");

        let second = app
            .oneshot(version_request(client))
            .await
            .expect("router responds");
        assert_eq!(
            second.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "second immediate request must be throttled with the burst-of-1 override"
        );
    }

    /// The compile-time defaults still allow a small burst (sanity check that
    /// the layer is active and the override test is testing the override).
    #[tokio::test]
    async fn default_rate_limit_allows_small_burst() {
        let app = {
            let _lock = env_lock();
            let _guard_per = EnvGuard::set("RWA_RATE_LIMIT_PER_SECOND", "10");
            let _guard_burst = EnvGuard::set("RWA_RATE_LIMIT_BURST", "5");
            router(state_with(Snapshot::default()))
        };
        let client = SocketAddr::from(([127, 0, 0, 1], 4002));

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(version_request(client))
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    fn cors_request(origin: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri("/version")
            .header(header::ORIGIN, origin)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 4200))))
            .body(Body::empty())
            .expect("cors request is well-formed")
    }

    /// #319 — CORS defaults to `http://localhost:3000` for a matching origin.
    #[tokio::test]
    async fn cors_defaults_to_localhost_3000_for_matching_origin() {
        let app = router(state_with(Snapshot::default()));

        let response = app
            .oneshot(cors_request("http://localhost:3000"))
            .await
            .expect("router responds");

        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|v| v.to_str().unwrap_or_default()),
            Some("http://localhost:3000")
        );
    }

    #[tokio::test]
    async fn cors_default_is_http_localhost_3000() {
        // No env override set → the compile-time default applies.
        let app = {
            let _lock = env_lock();
            std::env::remove_var("RWA_CORS_ALLOWED_ORIGINS");
            router(state_with(Snapshot::default()))
        };

        let response = app
            .oneshot(cors_request("http://localhost:3000"))
            .await
            .expect("router responds");
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|v| v.to_str().unwrap_or_default()),
            Some("http://localhost:3000")
        );
    }

    #[tokio::test]
    async fn cors_env_override_reflects_configured_origin() {
        let app = {
            let _lock = env_lock();
            let _guard = EnvGuard::set(
                "RWA_CORS_ALLOWED_ORIGINS",
                "https://app.example.com,https://admin.example.com",
            );
            router(state_with(Snapshot::default()))
        };

        let response = app
            .oneshot(cors_request("https://admin.example.com"))
            .await
            .expect("router responds");

        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|v| v.to_str().unwrap_or_default()),
            Some("https://admin.example.com")
        );
    }

    /// Non-allowlisted origins receive no `Access-Control-Allow-Origin` header.
    #[tokio::test]
    async fn cors_non_matching_origin_gets_no_allow_header() {
        let app = {
            let _lock = env_lock();
            let _guard = EnvGuard::set("RWA_CORS_ALLOWED_ORIGINS", "http://localhost:3000");
            router(state_with(Snapshot::default()))
        };

        let response = app
            .oneshot(cors_request("https://evil.example.com"))
            .await
            .expect("router responds");

        assert!(
            !response
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            "non-allowlisted origin must not receive a CORS allow header"
        );
    }
}
