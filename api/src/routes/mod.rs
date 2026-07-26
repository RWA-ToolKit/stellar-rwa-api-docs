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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::{Snapshot, Config, AppState, PrometheusHandle};
    use crate::models::{Asset, Stats, Holder, ComplianceSummary, Distribution};

    pub fn create_test_snapshot() -> Snapshot {
        let mut snapshot = Snapshot::default();
        snapshot.stats = Stats {
            total_assets: 2,
            active_assets: 1,
            tvl_cents: "1000000".to_string(),
            tvl_usd: 10000.0,
            total_holders: 2,
            total_distributions: 1,
            last_indexed_ledger: 12345,
            last_updated: Some("2024-01-01T00:00:00Z".to_string()),
        };

        snapshot.assets = vec![
            Asset {
                id: 1,
                token_contract: "CBALT5MZNYBMDWQKSMDZPXC5QVZZ4Y76WMBKBW3UEE7ZCBPB6XTHQ5LX".to_string(),
                issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                name: "Test Asset 1".to_string(),
                symbol: "TST1".to_string(),
                asset_type: "real_estate".to_string(),
                description: "A test asset for integration testing".to_string(),
                valuation_cents: "1000000".to_string(),
                valuation_usd: 10000.0,
                decimals: 7,
                total_supply: "1000000000000".to_string(),
                holders: 2,
                active: true,
                paused: false,
                compliance_contract: "CBJNPIXJFQGZRYNZV42DZQWJ53ZRXC4SYPQVNWZ7EOYKQ4JMNQRZ72LF".to_string(),
                created_at_ledger: 100,
            },
            Asset {
                id: 2,
                token_contract: "CBALT5MZNYBMDWQKSMDZPXC5QVZZ4Y76WMBKBW3UEE7ZCBPB6XTHQ5LY".to_string(),
                issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                name: "Test Asset 2".to_string(),
                symbol: "TST2".to_string(),
                asset_type: "bonds".to_string(),
                description: "An inactive test asset".to_string(),
                valuation_cents: "500000".to_string(),
                valuation_usd: 5000.0,
                decimals: 2,
                total_supply: "500000000".to_string(),
                holders: 0,
                active: false,
                paused: true,
                compliance_contract: "CBJNPIXJFQGZRYNZV42DZQWJ53ZRXC4SYPQVNWZ7EOYKQ4JMNQRZ72LG".to_string(),
                created_at_ledger: 200,
            }
        ];

        let holders = vec![
            Holder {
                address: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                balance: "500000000000".to_string(),
                share_percent: 50.0,
            },
            Holder {
                address: "GBUQWP3BOUZX34ULNQG23RQ6F5DUBYA4DSRVBJVXQ5DTOPJlvtj".to_string(),
                balance: "500000000000".to_string(),
                share_percent: 50.0,
            },
        ];
        snapshot.holders.insert(1, holders);

        let compliance = ComplianceSummary {
            total_records: 100,
            approved: 80,
            suspended: 10,
            rejected: 5,
            pending: 5,
            with_expiry: 20,
            jurisdictions: vec![],
        };
        snapshot.compliance.insert(1, compliance);

        let distributions = vec![
            Distribution {
                id: 1,
                asset_token: "CBALT5MZNYBMDWQKSMDZPXC5QVZZ4Y76WMBKBW3UEE7ZCBPB6XTHQ5LX".to_string(),
                payment_token: "CDZST3XVCDTUJ76ZAV2HA72KYYGZEY5RJVHDQKKEKCHCCDUPQGWCFBNA".to_string(),
                total_amount: "100000000".to_string(),
                distributed: "50000000".to_string(),
                claimed_percent: 50.0,
                completed: false,
                snapshot_ledger: 11000,
                created_at_ledger: 10000,
            }
        ];
        snapshot.dividends.insert(1, distributions);

        snapshot
    }

    pub fn create_test_app_state() -> AppState {
        let config = Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3".to_string(),
            dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX".to_string(),
            read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
        };

        let mut state = AppState::new(config, PrometheusHandle);
        let snapshot = create_test_snapshot();
        state.replace(snapshot);
        state
    }

    #[test]
    fn integration_full_route_smoke_test() {
        let state = create_test_app_state();
        let snap = state.snapshot();

        assert_eq!(snap.stats.total_assets, 2);
        assert_eq!(snap.assets.len(), 2);
        assert!(snap.holders.contains_key(&1));
        assert!(snap.compliance.contains_key(&1));
        assert!(snap.dividends.contains_key(&1));
    }

    #[test]
    fn route_snapshot_structure() {
        let snapshot = create_test_snapshot();
        assert_eq!(snapshot.stats.total_assets, 2);
        assert_eq!(snapshot.assets.len(), 2);
        assert!(snapshot.holders.contains_key(&1));
        assert!(snapshot.compliance.contains_key(&1));
        assert!(snapshot.dividends.contains_key(&1));
    }

    #[test]
    fn route_assets_with_filters() {
        let snapshot = create_test_snapshot();
        let active_assets: Vec<_> = snapshot
            .assets
            .iter()
            .filter(|a| a.active)
            .collect();
        assert_eq!(active_assets.len(), 1);
        assert_eq!(active_assets[0].id, 1);

        let real_estate: Vec<_> = snapshot
            .assets
            .iter()
            .filter(|a| a.asset_type == "real_estate")
            .collect();
        assert_eq!(real_estate.len(), 1);
    }

    #[test]
    fn route_asset_detail_lookup() {
        let snapshot = create_test_snapshot();
        let asset = snapshot.asset(1);
        assert!(asset.is_some());
        assert_eq!(asset.unwrap().symbol, "TST1");

        let missing = snapshot.asset(999);
        assert!(missing.is_none());
    }

    #[test]
    fn route_holders_for_asset() {
        let snapshot = create_test_snapshot();
        let holders = snapshot.holders.get(&1);
        assert!(holders.is_some());
        let holders = holders.unwrap();
        assert_eq!(holders.len(), 2);
        assert!(holders[0].share_percent <= 100.0);
    }

    #[test]
    fn route_compliance_for_asset() {
        let snapshot = create_test_snapshot();
        let compliance = snapshot.compliance.get(&1);
        assert!(compliance.is_some());
        let compliance = compliance.unwrap();
        assert_eq!(compliance.total_records, 100);
        assert_eq!(compliance.approved, 80);
        assert!(compliance.approved + compliance.suspended + compliance.rejected + compliance.pending <= compliance.total_records);
    }

    #[test]
    fn route_dividends_for_asset() {
        let snapshot = create_test_snapshot();
        let distributions = snapshot.dividends.get(&1);
        assert!(distributions.is_some());
        let dists = distributions.unwrap();
        assert_eq!(dists.len(), 1);
        assert_eq!(dists[0].id, 1);
        assert!(dists[0].claimed_percent <= 100.0);
    }

    #[test]
    fn route_stats_from_seeded_snapshot() {
        let state = create_test_app_state();
        let snap = state.snapshot();
        assert_eq!(snap.stats.total_assets, 2);
        assert_eq!(snap.stats.active_assets, 1);
        assert_eq!(snap.stats.last_indexed_ledger, 12345);
        assert_eq!(snap.stats.total_distributions, 1);
    }
}
