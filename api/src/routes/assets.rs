//! `GET /assets` and `GET /assets/:id`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Asset;

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 100;

/// Optional filters and pagination for the asset list.
#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    /// Filter by asset class, e.g. `real_estate`.
    pub asset_type: Option<String>,
    /// Filter by active status.
    pub active: Option<bool>,
    /// Skip the first `offset` matching assets.
    #[serde(default)]
    pub offset: Option<usize>,
    /// Limit the number of matching assets returned. Defaults to 50 and is capped at 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// All tokenized assets with valuation, supply and holder counts.
///
/// Supports optional `?asset_type=`, `?active=`, `?offset=` and `?limit=` query filters.
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AssetQuery>,
) -> Json<Vec<Asset>> {
    let snap = state.snapshot();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);
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
        .skip(offset)
        .take(limit)
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
            indexed_at_ledger: 1,
            index_error: None,
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

    #[tokio::test]
    async fn get_asset_by_unknown_id_returns_404() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/99999")
                    .body(Body::empty())
                    .unwrap(),
            )
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
        let result = get_assets(
            list_router(assets),
            "/assets?asset_type=real_estate&active=true",
        )
        .await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[tokio::test]
    async fn limit_and_offset_paginate_assets() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "real_estate", true),
            stub_asset(3, "real_estate", true),
            stub_asset(4, "real_estate", true),
        ];
        let result = get_assets(list_router(assets), "/assets?offset=1&limit=2").await;
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 2);
        assert_eq!(result[1].id, 3);
    }

    // Iterator::skip handles an offset larger than the collection size
    // gracefully today (empty array, still 200 OK), but nothing pinned that
    // behavior — a future refactor of the pagination logic could silently
    // regress it into an error or a panic.
    #[tokio::test]
    async fn offset_beyond_collection_size_returns_empty_array() {
        let assets = vec![
            stub_asset(1, "real_estate", true),
            stub_asset(2, "real_estate", true),
            stub_asset(3, "real_estate", true),
        ];
        let result = get_assets(list_router(assets), "/assets?offset=1000").await;
        assert!(
            result.is_empty(),
            "offset past the end of the collection should return an empty array, not an error"
        );
    }

    #[tokio::test]
    async fn limit_is_clamped_to_max_page_size() {
        let assets = (1..=150)
            .map(|id| stub_asset(id, "real_estate", true))
            .collect();
        let result = get_assets(list_router(assets), "/assets?limit=1000").await;
        assert_eq!(result.len(), 100);
    }

    // #194 – non-numeric asset id returns 400, not 404
    #[tokio::test]
    async fn get_asset_by_non_numeric_id_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "non-numeric id 'abc' should return 400, not {}",
            response.status()
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        // The body must mention the offending parameter so clients can diagnose
        // the error without consulting the API documentation.
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("id") || text.contains("abc"),
            "400 body should name the offending parameter; got: {text}"
        );
    }

    // #195 – boundary ids (0 and u64::MAX) return 404 cleanly without panicking
    #[tokio::test]
    async fn get_asset_by_id_zero_returns_404() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "id=0 should return 404, not {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn get_asset_by_id_u64_max_returns_404() {
        let app = test_router();
        let uri = format!("/assets/{}", u64::MAX);
        let response = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "id=u64::MAX should return 404, not {}",
            response.status()
        );
    }

    // #196 – GET /assets/:id response field set matches the OpenAPI schema
    #[tokio::test]
    async fn get_asset_by_id_returns_stable_field_set() {
        use crate::indexer::Snapshot;
        use crate::routes::test_support;
        use std::collections::BTreeSet;

        let asset = stub_asset(1, "real_estate", true);
        let state = test_support::state_with(Snapshot {
            assets: vec![asset],
            ..Snapshot::default()
        });
        let app = Router::new()
            .route("/assets/:id", get(super::detail))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response must be valid JSON");

        let actual_keys: BTreeSet<String> = value
            .as_object()
            .expect("response must be a JSON object")
            .keys()
            .cloned()
            .collect();

        // Required fields defined in the OpenAPI schema for an Asset.
        let expected_keys: BTreeSet<String> = [
            "id",
            "token_contract",
            "issuer",
            "name",
            "symbol",
            "asset_type",
            "description",
            "valuation_cents",
            "valuation_usd",
            "decimals",
            "total_supply",
            "holders",
            "active",
            "paused",
            "compliance_contract",
            "created_at_ledger",
            "indexed_at_ledger",
            "index_error",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let missing: Vec<_> = expected_keys.difference(&actual_keys).collect();
        let extra: Vec<_> = actual_keys.difference(&expected_keys).collect();
        assert!(
            missing.is_empty(),
            "response is missing required fields: {missing:?}"
        );
        assert!(
            extra.is_empty(),
            "response contains unexpected fields (schema drift): {extra:?}"
        );
    }

    // #197 – GET /assets returns [] (empty array) not null when no assets exist
    #[tokio::test]
    async fn list_assets_returns_empty_array_not_null_when_no_assets() {
        let app = list_router(vec![]);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response must be valid JSON");

        assert!(
            value.is_array(),
            "empty asset list must serialize as a JSON array, not null or object; got: {value}"
        );
        assert_eq!(
            value.as_array().unwrap().len(),
            0,
            "empty asset list must be an empty array []"
        );
    }
}
