//! `GET /assets/:id/holders`.

use axum::{
    extract::{Path, State},
    Json,
};

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Holder;

/// Holder list for an asset, sorted by balance descending.
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Vec<Holder>>, ApiError> {
    let snap = state.snapshot();
    super::require_ready(&snap)?;
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.holders.get(&id).cloned().unwrap_or_default()))
}
