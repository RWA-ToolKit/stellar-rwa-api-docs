//! `GET /assets` and `GET /assets/:id`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::{ApiError, Pagination};
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
/// Supports optional `?asset_type=` and `?active=` query filters, and
/// `?limit=&offset=` pagination (see [`Pagination`]).
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AssetQuery>,
    Query(page): Query<Pagination>,
) -> Json<Vec<Asset>> {
    let snap = state.snapshot();
    let assets = snap
        .assets
        .iter()
        .filter(|a| {
            query
                .asset_type
                .as_deref()
                .is_none_or(|t| a.asset_type == t)
        })
        .filter(|a| query.active.is_none_or(|active| a.active == active))
        .cloned()
        .collect();
    Json(page.apply(assets))
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
