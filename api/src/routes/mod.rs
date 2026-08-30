//! HTTP routing and the shared API error type.

pub mod assets;
pub mod compliance;
pub mod dividends;
pub mod holders;
pub mod stats;

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{Any, CorsLayer};

use crate::indexer::{AppState, POLL_INTERVAL};
use crate::models::ApiErrorBody;

/// Sustained requests-per-second allowed per client IP, with bursting.
const RATE_LIMIT_PER_SECOND: u64 = 5;
const RATE_LIMIT_BURST: u32 = 20;

/// Snapshot-backed data routes, mounted under [`API_VERSION_PREFIX`]. Single
/// source of truth for the router, the root index, and the
/// docs/public/openapi.json-vs-router test — keep in sync with the
/// `.route(...)` calls in [`router`] and with the spec.
const DATA_ROUTE_PATHS: &[&str] = &[
    "/stats",
    "/assets",
    "/assets/:id",
    "/assets/:id/holders",
    "/assets/:id/compliance",
    "/assets/:id/dividends",
    "/assets/:id/dividends/:did",
];

/// Path prefix all data routes are nested under.
const API_VERSION_PREFIX: &str = "/v1";

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
        .route("/assets/:id/dividends/:did", get(dividends::get_one))
        .layer(middleware::from_fn_with_state(state.clone(), cache_headers))
        .layer(GovernorLayer {
            config: governor_conf,
        });

    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .nest(API_VERSION_PREFIX, data_routes)
        .with_state(state)
        .layer(cors)
}

/// Attach `Cache-Control` and `ETag` to snapshot-backed responses, and answer
/// `If-None-Match` with `304 Not Modified` when the snapshot hasn't advanced.
///
/// The snapshot only changes once per [`POLL_INTERVAL`], so the ETag is
/// derived from `last_indexed_ledger`: two requests against the same indexed
/// ledger are guaranteed to have identical bodies.
async fn cache_headers(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let ledger = state.last_indexed_ledger().await;
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
    let endpoints: Vec<String> = DATA_ROUTE_PATHS
        .iter()
        .map(|path| format!("GET {API_VERSION_PREFIX}{path}"))
        .chain(["GET /health".to_string(), "GET /metrics".to_string()])
        .collect();
    Json(json!({
        "name": "Stellar RWA API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Read-only index of tokenized real-world asset activity on Stellar.",
        "endpoints": endpoints,
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

/// Shared helpers for building an [`AppState`] in router-level tests, without
/// a live Soroban RPC or the process-global Prometheus recorder.
#[cfg(test)]
pub(crate) mod test_support {
    use metrics_exporter_prometheus::PrometheusBuilder;

    use crate::indexer::{AppState, Config, Snapshot};
    use crate::models::Asset;

    fn test_config() -> Config {
        Config {
            rpc_url: "https://example.invalid".to_string(),
            registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3".to_string(),
            dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX".to_string(),
            read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
        }
    }

    /// An `AppState` with an empty snapshot — no assets, no data.
    pub(crate) fn test_state() -> AppState {
        // `build_recorder` (as opposed to `install_recorder`) does not touch
        // the process-global recorder, so it's safe to call from many tests.
        let handle = PrometheusBuilder::new().build_recorder().handle();
        AppState::new(test_config(), handle)
    }

    /// An `AppState` seeded with a single bare-minimum asset at `id`, and no
    /// holders/compliance/dividends data.
    pub(crate) fn test_state_with_asset(id: u64) -> AppState {
        let state = test_state();
        state.seed_for_test(Snapshot {
            assets: vec![Asset {
                id,
                token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ"
                    .to_string(),
                issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                name: "Test Asset".to_string(),
                symbol: "TEST".to_string(),
                asset_type: "real_estate".to_string(),
                description: String::new(),
                valuation_cents: "100".to_string(),
                valuation_usd: 1.0,
                decimals: 2,
                total_supply: "100".to_string(),
                holders: 0,
                active: true,
                paused: false,
                compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU"
                    .to_string(),
                created_at_ledger: 1,
            }],
            ..Snapshot::default()
        });
        state
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    /// docs/public/openapi.json must document exactly the paths the router
    /// mounts under /v1 (see #262: a spec/router mismatch 404s anyone who
    /// follows the docs or generates a client from the spec), and every
    /// documented path must actually resolve on the live router.
    #[tokio::test]
    async fn openapi_paths_match_router() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let spec_path = format!("{manifest_dir}/../docs/public/openapi.json");
        let raw = std::fs::read_to_string(&spec_path)
            .unwrap_or_else(|e| panic!("failed to read {spec_path}: {e}"));
        let spec: serde_json::Value =
            serde_json::from_str(&raw).expect("docs/public/openapi.json should be valid JSON");

        let spec_paths: BTreeSet<String> = spec["paths"]
            .as_object()
            .expect("openapi.json should have a top-level \"paths\" object")
            .keys()
            .cloned()
            .collect();

        let router_paths: BTreeSet<String> = DATA_ROUTE_PATHS
            .iter()
            .map(|path| {
                format!(
                    "{API_VERSION_PREFIX}{}",
                    path.replace(":id", "{id}").replace(":did", "{did}")
                )
            })
            .collect();

        assert_eq!(
            spec_paths, router_paths,
            "docs/public/openapi.json \"paths\" keys must match the router's mounted /v1 routes"
        );

        let app = router(test_support::test_state());
        for path in &router_paths {
            let concrete = path.replace("{id}", "1").replace("{did}", "1");
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(concrete.clone())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            assert!(
                serde_json::from_slice::<serde_json::Value>(&body).is_ok(),
                "documented path {path} (requested as {concrete}) has no live route on the router"
            );
        }
    }
}
