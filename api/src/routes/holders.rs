//! `GET /assets/:id/holders`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Holder;

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 100;

#[derive(Debug, Deserialize)]
pub struct HolderQuery {
    /// Skip the first `offset` holders.
    #[serde(default)]
    pub offset: Option<usize>,
    /// Limit the number of holders returned. Defaults to 50 and is capped at 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Holder list for an asset, sorted by balance descending.
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(query): Query<HolderQuery>,
) -> Result<Json<Vec<Holder>>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);
    let holders = snap
        .holders
        .get(&id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    Ok(Json(holders))
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, Query, State};

    use super::{list, HolderQuery};
    use crate::indexer::Snapshot;
    use crate::routes::test_support::{asset, state_with};
    use crate::routes::ApiError;

    #[tokio::test]
    async fn missing_asset_is_404() {
        let state = state_with(Snapshot::default());

        let err = list(
            State(state),
            Path(42),
            Query(HolderQuery {
                offset: None,
                limit: None,
            }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn present_asset_with_no_holders_is_empty_array() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        let state = state_with(snap);

        let holders = list(
            State(state),
            Path(1),
            Query(HolderQuery {
                offset: None,
                limit: None,
            }),
        )
        .await
        .expect("asset exists")
        .0;

        assert!(holders.is_empty());
    }

    #[tokio::test]
    async fn pagination_works_for_holders() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.holders.insert(
            1,
            vec![
                crate::models::Holder {
                    address: "a".to_string(),
                    balance: "1".to_string(),
                    share_percent: 10.0,
                },
                crate::models::Holder {
                    address: "b".to_string(),
                    balance: "2".to_string(),
                    share_percent: 20.0,
                },
                crate::models::Holder {
                    address: "c".to_string(),
                    balance: "3".to_string(),
                    share_percent: 30.0,
                },
            ],
        );
        let state = state_with(snap);

        let holders = list(
            State(state),
            Path(1),
            Query(HolderQuery {
                offset: Some(1),
                limit: Some(2),
            }),
        )
        .await
        .expect("asset exists")
        .0;

        assert_eq!(holders.len(), 2);
        assert_eq!(holders[0].address, "b");
        assert_eq!(holders[1].address, "c");
    }

    // Iterator::skip handles an offset larger than the holder list gracefully
    // today (empty array, still 200 OK), but nothing pinned that behavior —
    // a future refactor of the pagination logic could silently regress it.
    #[tokio::test]
    async fn offset_beyond_holder_count_returns_empty_array() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.holders.insert(
            1,
            vec![crate::models::Holder {
                address: "a".to_string(),
                balance: "1".to_string(),
                share_percent: 100.0,
            }],
        );
        let state = state_with(snap);

        let holders = list(
            State(state),
            Path(1),
            Query(HolderQuery {
                offset: Some(1000),
                limit: None,
            }),
        )
        .await
        .expect("asset exists")
        .0;

        assert!(
            holders.is_empty(),
            "offset past the end of the holder list should return an empty array, not an error"
        );
    }
}
