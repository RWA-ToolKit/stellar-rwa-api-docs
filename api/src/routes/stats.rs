//! `GET /stats`.

use axum::{extract::State, Json};

use crate::indexer::AppState;
use crate::models::Stats;

/// Platform-wide statistics: asset count, TVL, holders, distributions.
pub async fn get(State(state): State<AppState>) -> Json<Stats> {
    let snap = state.snapshot();
    Json(snap.stats.clone())
}
