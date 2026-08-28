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

    #[tokio::test]
    async fn holder_share_percentages_sum_to_100() {
        use crate::models::Holder;

        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.holders.insert(
            1,
            vec![
                Holder {
                    address: "ADDR1".to_string(),
                    balance: "500000".to_string(),
                    share_percent: 50.0,
                },
                Holder {
                    address: "ADDR2".to_string(),
                    balance: "333333".to_string(),
                    share_percent: 33.3333,
                },
                Holder {
                    address: "ADDR3".to_string(),
                    balance: "166667".to_string(),
                    share_percent: 16.6667,
                },
            ],
        );
        let state = state_with(snap);

        let body = list(State(state), Path(1)).await.expect("asset exists").0;
        let value = serde_json::to_value(&body).expect("serialize holder list");

        let sum_shares: f64 = value
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["share_percent"].as_f64().unwrap())
            .sum();

        assert!(
            (sum_shares - 100.0).abs() < 0.01,
            "holder share percentages sum ({sum_shares}) must equal 100 within rounding tolerance"
        );
    }
}
