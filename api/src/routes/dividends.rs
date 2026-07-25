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

#[cfg(test)]
mod tests {
    use axum::extract::{Path, State};

    use super::list;
    use crate::indexer::Snapshot;
    use crate::routes::test_support::{asset, distribution, state_with};

    #[tokio::test]
    async fn distributions_sorted_by_created_at_ledger_descending() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        // Deliberately out of order, so a passing assertion proves the
        // handler sorts rather than preserving insertion order.
        snap.dividends.insert(
            1,
            vec![
                distribution(10, 100),
                distribution(11, 300),
                distribution(12, 200),
            ],
        );
        let state = state_with(snap);

        let dists = list(State(state), Path(1)).await.expect("asset exists").0;

        let ledgers: Vec<u32> = dists.iter().map(|d| d.created_at_ledger).collect();
        assert_eq!(ledgers, vec![300, 200, 100]);
    }
}
