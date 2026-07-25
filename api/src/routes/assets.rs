//! `GET /assets` and `GET /assets/:id`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Asset;

/// Optional filters for the asset list.
#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    /// Filter by asset class, e.g. `real_estate`.
    pub asset_type: Option<String>,
    /// Filter by active status.
    pub active: Option<bool>,
}

/// All tokenized assets with valuation, supply and holder counts.
///
/// Supports optional `?asset_type=` and `?active=` query filters.
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AssetQuery>,
) -> Json<Vec<Asset>> {
    let snap = state.snapshot();
    let assets = snap
        .assets
        .into_iter()
        .filter(|a| {
            query
                .asset_type
                .as_deref()
                .is_none_or(|t| a.asset_type == t)
        })
        .filter(|a| query.active.is_none_or(|active| a.active == active))
        .collect();
    Json(assets)
}

/// Full detail for a single asset by its registry id.
pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Asset>, ApiError> {
    let snap = state.snapshot();
    snap.asset(id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("no asset with id {id}")))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt as _;

    use crate::indexer::AppState;
    use crate::models::Asset;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Return a minimal but fully-populated [`Asset`] with controllable
    /// `asset_type` and `active` fields. All other fields are filled with
    /// dummy values that do not affect the filter logic.
    fn stub_asset(id: u64, asset_type: &str, active: bool) -> Asset {
        Asset {
            id,
            token_contract: format!("CONTRACT{id}"),
            issuer: format!("ISSUER{id}"),
            name: format!("Asset {id}"),
            symbol: format!("TKN{id}"),
            asset_type: asset_type.to_string(),
            description: String::new(),
            valuation_cents: "0".to_string(),
            valuation_usd: 0.0,
            decimals: 7,
            total_supply: "0".to_string(),
            holders: 0,
            active,
            paused: false,
            compliance_contract: format!("COMPLIANCE{id}"),
            created_at_ledger: 1,
        }
    }

    /// Build a router for `GET /assets` seeded with the supplied assets.
    fn list_router(assets: Vec<Asset>) -> Router {
        let state = AppState::with_assets(assets);
        Router::new()
            .route("/assets", get(super::list))
            .with_state(state)
    }

    /// Send `GET {uri}` through `app`, assert the status, and decode the JSON
    /// body as a `Vec<Asset>`.  Panics on any parse / transport error so test
    /// failures surface a clear message rather than an `unwrap` backtrace.
    async fn get_assets(app: Router, uri: &str) -> Vec<Asset> {
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "expected 200 from {uri}, got {}",
            resp.status()
        );

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("failed to parse asset list JSON from {uri}: {e}"))
    }

    // -----------------------------------------------------------------------
    // Filter tests
    // -----------------------------------------------------------------------

    /// `?asset_type=real_estate` must return only the assets whose
    /// `asset_type` field equals `"real_estate"`, regardless of `active`.
    #[tokio::test]
    async fn filter_by_asset_type_returns_matching_assets() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "real_estate", false),
            stub_asset(3, "bond", true),
            stub_asset(4, "commodity", false),
        ];
        let app = list_router(assets);

        let result = get_assets(app, "/assets?asset_type=real_estate").await;

        assert_eq!(result.len(), 2, "expected 2 real_estate assets, got {}", result.len());
        assert!(
            result.iter().all(|a| a.asset_type == "real_estate"),
            "all returned assets must be real_estate; got: {:?}",
            result.iter().map(|a| &a.asset_type).collect::<Vec<_>>(),
        );
    }

    /// `?active=true` must return only assets where `active == true`,
    /// regardless of `asset_type`.
    #[tokio::test]
    async fn filter_by_active_returns_only_active_assets() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "bond", true),
            stub_asset(3, "real_estate", false),
            stub_asset(4, "commodity", false),
        ];
        let app = list_router(assets);

        let result = get_assets(app, "/assets?active=true").await;

        assert_eq!(result.len(), 2, "expected 2 active assets, got {}", result.len());
        assert!(
            result.iter().all(|a| a.active),
            "all returned assets must be active; got ids: {:?}",
            result.iter().map(|a| a.id).collect::<Vec<_>>(),
        );
    }

    /// `?asset_type=real_estate&active=true` must return only assets that
    /// satisfy *both* predicates simultaneously.
    #[tokio::test]
    async fn filter_by_asset_type_and_active_combined() {
        let assets = vec![
            stub_asset(1, "real_estate", true),   // ✓ matches both
            stub_asset(2, "real_estate", false),  // ✗ wrong active
            stub_asset(3, "bond", true),           // ✗ wrong type
            stub_asset(4, "commodity", false),    // ✗ wrong both
        ];
        let app = list_router(assets);

        let result =
            get_assets(app, "/assets?asset_type=real_estate&active=true").await;

        assert_eq!(
            result.len(),
            1,
            "expected exactly 1 asset matching both filters, got {}: {:?}",
            result.len(),
            result.iter().map(|a| a.id).collect::<Vec<_>>(),
        );
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].asset_type, "real_estate");
        assert!(result[0].active);
    }
}
