// SPDX-License-Identifier: Apache-2.0

//! `GET /assets/:id/dividends`.

use axum::{
    extract::{Path, State},
    Json,
};

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Distribution;

/// Distribution history for an asset, newest ledger first.
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Vec<Distribution>>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    let mut dists = snap.dividends.get(&id).cloned().unwrap_or_default();
    dists.sort_by_key(|d| std::cmp::Reverse(d.created_at_ledger));
    Ok(Json(dists))
}
