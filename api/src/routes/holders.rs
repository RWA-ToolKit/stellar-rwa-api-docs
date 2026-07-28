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
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.holders.get(&id).cloned().unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, State};

    use super::list;
    use crate::indexer::Snapshot;
    use crate::routes::test_support::{asset, state_with};
    use crate::routes::ApiError;

    #[tokio::test]
    async fn missing_asset_is_404() {
        let state = state_with(Snapshot::default());

        let err = list(State(state), Path(42)).await.unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn present_asset_with_no_holders_is_empty_array() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        let state = state_with(snap);

        let holders = list(State(state), Path(1)).await.expect("asset exists").0;

        assert!(holders.is_empty());
    }
}
