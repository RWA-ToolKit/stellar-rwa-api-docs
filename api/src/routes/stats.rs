//! `GET /stats`.

use axum::{extract::State, Json};

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Stats;

/// Platform-wide statistics: asset count, TVL, holders, distributions.
pub async fn get(State(state): State<AppState>) -> Result<Json<Stats>, ApiError> {
    let snap = state.snapshot();
    super::require_ready(&snap)?;
    Ok(Json(snap.stats))
}
