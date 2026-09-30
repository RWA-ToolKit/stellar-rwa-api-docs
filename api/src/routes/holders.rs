//! `GET /assets/:id/holders`.

use std::collections::HashSet;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ApiError;
use crate::indexer::{derive_allowed, AppState};
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

/// Compliance view for a single address, showing every asset whose allowlist
/// contains the address (including zero-balance entries).
///
/// `status` and `allowed` reflect the real on-chain KYC record persisted in
/// the snapshot — not a hardcoded constant.  An unknown address returns `[]`.
pub async fn by_address_compliance(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<crate::models::AddressCompliance>>, ApiError> {
    let snap = state.snapshot();
    let latest_ledger = snap.stats.last_indexed_ledger;
    // Blocked jurisdictions are not yet fetched from the chain; default to
    // empty so we don't incorrectly gate anyone.
    let blocked: HashSet<String> = HashSet::new();

    let mut entries = Vec::new();

    for (asset_id, asset_records) in &snap.compliance_records {
        if let Some(rec) = asset_records.get(&address) {
            if let Some(asset) = snap.asset(*asset_id) {
                let balance = snap
                    .holders
                    .get(asset_id)
                    .and_then(|holders| holders.iter().find(|h| h.address == address))
                    .map(|h| h.balance.clone())
                    .unwrap_or_else(|| "0".to_string());

                let allowed = derive_allowed(rec, latest_ledger, &blocked);
                entries.push(crate::models::AddressCompliance {
                    address: address.clone(),
                    asset_id: *asset_id,
                    asset_name: asset.name.clone(),
                    symbol: asset.symbol.clone(),
                    balance,
                    status: rec.status.clone(),
                    allowed,
                    jurisdiction: Some(rec.jurisdiction.clone()),
                    expires_at: if rec.expires_at == 0 {
                        None
                    } else {
                        Some(rec.expires_at)
                    },
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

    // -------------------------------------------------------------------------
    // #457 – by_address_compliance uses real on-chain records
    // -------------------------------------------------------------------------

    use super::by_address_compliance;
    use crate::models::{ComplianceRecord, Stats};
    use std::collections::HashMap;

    fn crec(status: &str, jurisdiction: &str, expires_at: u32) -> ComplianceRecord {
        ComplianceRecord {
            status: status.to_string(),
            jurisdiction: jurisdiction.to_string(),
            expires_at,
        }
    }

    fn snap_with_crec(
        asset_id: u64,
        address: &str,
        rec: ComplianceRecord,
        balance: i128,
        latest_ledger: u32,
    ) -> Snapshot {
        let mut snap = Snapshot {
            stats: Stats {
                last_indexed_ledger: latest_ledger,
                ..Stats::default()
            },
            ..Snapshot::default()
        };
        snap.assets.push(asset(asset_id));
        if balance > 0 {
            snap.holders.insert(
                asset_id,
                vec![crate::models::Holder {
                    address: address.to_string(),
                    balance: balance.to_string(),
                    share_percent: 100.0,
                }],
            );
        } else {
            snap.holders.insert(asset_id, vec![]);
        }
        let mut crecs = HashMap::new();
        crecs.insert(address.to_string(), rec);
        snap.compliance_records.insert(asset_id, crecs);
        snap
    }

    /// Suspended address: status="Suspended", allowed=false.
    #[tokio::test]
    async fn holders_compliance_suspended_is_not_allowed() {
        let snap = snap_with_crec(1, "GADDR", crec("Suspended", "US", 0), 1_000, 100);
        let state = state_with(snap);

        let result = by_address_compliance(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "Suspended");
        assert!(!result[0].allowed, "Suspended must not be allowed");
    }

    /// Expired approval: status="Approved", expires_at in the past → allowed=false.
    #[tokio::test]
    async fn holders_compliance_expired_approval_is_not_allowed() {
        // expires_at=50, latest_ledger=100 → expired
        let snap = snap_with_crec(1, "GADDR", crec("Approved", "US", 50), 500, 100);
        let state = state_with(snap);

        let result = by_address_compliance(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "Approved");
        assert!(!result[0].allowed, "Expired approval must not be allowed");
        assert_eq!(result[0].expires_at, Some(50));
    }

    /// Zero-balance allowlisted address appears with balance="0" and correct status.
    #[tokio::test]
    async fn holders_compliance_zero_balance_shows_allowlisted_entry() {
        // balance=0 → not in holders map
        let snap = snap_with_crec(1, "GADDR", crec("Approved", "SG", 0), 0, 100);
        let state = state_with(snap);

        let result = by_address_compliance(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1, "allowlisted zero-balance address must appear");
        assert_eq!(result[0].balance, "0");
        assert!(result[0].allowed);
    }

    /// Unknown address returns empty array (not 404).
    #[tokio::test]
    async fn holders_compliance_unknown_address_returns_empty() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.holders.insert(1, vec![]);
        let state = state_with(snap);

        let result = by_address_compliance(State(state), Path("GUNKNOWN".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert!(result.is_empty(), "unknown address must return []");
    }
}
