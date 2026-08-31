//! `GET /assets/:id/holders`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ApiError;
use crate::indexer::AppState;
use crate::models::{AddressHolding, Holder};

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

/// Portfolio of assets held by a single address, sorted by balance descending.
pub async fn by_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<AddressHolding>>, ApiError> {
    let snap = state.snapshot();
    let mut holdings = Vec::new();

    for (asset_id, holders) in &snap.holders {
        if let Some(holder) = holders.iter().find(|h| h.address == address) {
            if let Some(asset) = snap.asset(*asset_id) {
                holdings.push(AddressHolding {
                    address: holder.address.clone(),
                    asset_id: *asset_id,
                    asset_name: asset.name.clone(),
                    symbol: asset.symbol.clone(),
                    balance: holder.balance.clone(),
                    share_percent: holder.share_percent,
                });
            }
        }
    }

    holdings.sort_by(|a, b| {
        b.balance
            .parse::<i128>()
            .unwrap_or_default()
            .cmp(&a.balance.parse::<i128>().unwrap_or_default())
    });

    Ok(Json(holdings))
}

/// Compliance view for a single address, showing which assets it currently holds.
pub async fn by_address_compliance(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<crate::models::AddressCompliance>>, ApiError> {
    let snap = state.snapshot();
    let mut entries = Vec::new();

    for (asset_id, holders) in &snap.holders {
        if let Some(holder) = holders.iter().find(|h| h.address == address) {
            if let Some(asset) = snap.asset(*asset_id) {
                entries.push(crate::models::AddressCompliance {
                    address: holder.address.clone(),
                    asset_id: *asset_id,
                    asset_name: asset.name.clone(),
                    symbol: asset.symbol.clone(),
                    balance: holder.balance.clone(),
                    status: "approved".to_string(),
                    allowed: true,
                });
            }
        }
    }

    entries.sort_by(|a, b| {
        b.balance
            .parse::<i128>()
            .unwrap_or_default()
            .cmp(&a.balance.parse::<i128>().unwrap_or_default())
    });

    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, Query, State};

    use super::{by_address, list, HolderQuery};
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

    #[tokio::test]
    async fn address_holding_lookup_returns_matching_assets() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.assets.push(asset(2));
        snap.holders.insert(
            1,
            vec![crate::models::Holder {
                address: "GADDRESS".to_string(),
                balance: "250".to_string(),
                share_percent: 25.0,
            }],
        );
        snap.holders.insert(
            2,
            vec![crate::models::Holder {
                address: "GADDRESS".to_string(),
                balance: "100".to_string(),
                share_percent: 10.0,
            }],
        );
        let state = state_with(snap);

        let holdings = by_address(State(state), Path("GADDRESS".to_string()))
            .await
            .expect("address lookup should succeed")
            .0;

        assert_eq!(holdings.len(), 2);
        assert_eq!(holdings[0].asset_id, 1);
        assert_eq!(holdings[1].asset_id, 2);
    }
}
