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
    use crate::models::{ApiErrorBody, Asset};

    fn test_router() -> Router {
        let state = AppState::for_test_empty();
        Router::new()
            .route("/assets/:id", get(super::detail))
            .with_state(state)
    }

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

    fn list_router(assets: Vec<Asset>) -> Router {
        let state = AppState::with_assets(assets);
        Router::new()
            .route("/assets", get(super::list))
            .with_state(state)
    }

    async fn get_assets(app: Router, uri: &str) -> Vec<Asset> {
        let resp = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "expected 200 from {uri}, got {}", resp.status());
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("failed to parse asset list JSON from {uri}: {e}"))
    }

    #[tokio::test]
    async fn get_asset_by_unknown_id_returns_404() {
        let app = test_router();
        let response = app
            .oneshot(Request::builder().uri("/assets/99999").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ApiErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error, "not_found");
        assert!(error.message.contains("99999"));
    }

    #[tokio::test]
    async fn filter_by_asset_type_returns_matching_assets() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "real_estate", false),
            stub_asset(3, "bond", true),
        ];
        let result = get_assets(list_router(assets), "/assets?asset_type=real_estate").await;
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|a| a.asset_type == "real_estate"));
    }

    #[tokio::test]
    async fn filter_by_active_returns_only_active_assets() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "bond", true),
            stub_asset(3, "real_estate", false),
        ];
        let result = get_assets(list_router(assets), "/assets?active=true").await;
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|a| a.active));
    }

    #[tokio::test]
    async fn filter_by_asset_type_and_active_combined() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "real_estate", false),
            stub_asset(3, "bond", true),
        ];
        let result = get_assets(list_router(assets), "/assets?asset_type=real_estate&active=true").await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }
}
