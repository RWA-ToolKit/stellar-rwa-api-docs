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
) -> Result<Json<Vec<Asset>>, ApiError> {
    let snap = state.snapshot();
    super::require_ready(&snap)?;
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
    Ok(Json(assets))
}

/// Full detail for a single asset by its registry id.
pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Asset>, ApiError> {
    let snap = state.snapshot();
    super::require_ready(&snap)?;
    snap.asset(id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("no asset with id {id}")))
}
