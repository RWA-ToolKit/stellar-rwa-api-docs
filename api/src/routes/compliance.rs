//! `GET /assets/:id/compliance`.

use axum::{
    extract::{Path, State},
    Json,
};

use super::ApiError;
use crate::indexer::AppState;
use crate::models::ComplianceSummary;

/// Aggregate compliance summary for an asset (counts only — no addresses/PII).
pub async fn summary(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<ComplianceSummary>, ApiError> {
    let snap = state.snapshot();
    super::require_ready(&snap)?;
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.compliance.get(&id).cloned().unwrap_or_default()))
}
