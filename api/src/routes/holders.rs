//! `GET /assets/:id/holders`.

use axum::{
    extract::{Path, Query, State},
    Json,
};

use super::{ApiError, Pagination};
use crate::indexer::AppState;
use crate::models::Holder;

/// Holder list for an asset, sorted by balance descending.
///
/// Supports `?limit=&offset=` pagination (see [`Pagination`]).
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(page): Query<Pagination>,
) -> Result<Json<Vec<Holder>>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    let holders = snap.holders.get(&id).cloned().unwrap_or_default();
    Ok(Json(page.apply(holders)))
}
