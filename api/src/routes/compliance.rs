//! `GET /assets/:id/compliance`.

use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    Json,
};

use super::ApiError;
use crate::indexer::{derive_allowed, AppState};
use crate::models::{AddressCompliance, ComplianceSummary};

/// Aggregate compliance summary for an asset (counts only — no addresses/PII).
pub async fn summary(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<ComplianceSummary>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.compliance.get(&id).cloned().unwrap_or_default()))
}

/// Compliance status for a single address across every asset whose allowlist
/// contains the address.
///
/// Returns every asset the address is allowlisted on, even when the balance is
/// zero (the caller can distinguish zero-balance from unknown address).  An
/// unknown address — one that appears on no allowlist at all — returns an
/// empty array `[]`.
///
/// `allowed` is derived from the on-chain record: `status == "Approved"`,
/// record not expired at the latest indexed ledger, and jurisdiction not in
/// the blocked set.  The blocked-jurisdiction set is not currently read from
/// the chain, so the API defaults to an empty set; future work can expose the
/// `is_jurisdiction_blocked` contract query here.
///
/// Entries are sorted by `share_percent` descending then `asset_id` ascending.
/// Raw base-unit balances are not comparable across assets with different
/// `decimals`; `share_percent` is the only normalized cross-asset ordering key.
/// The secondary `asset_id` sort makes the response fully deterministic even
/// when two entries share the same `share_percent`.
pub async fn for_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<AddressCompliance>>, ApiError> {
    let snap = state.snapshot();
    let latest_ledger = snap.stats.last_indexed_ledger;
    // Blocked jurisdictions are not yet fetched from the chain; default to
    // empty so we don't incorrectly gate anyone, and document this clearly.
    let blocked: HashSet<String> = HashSet::new();

    // Iterate in asset_id order so the secondary sort key is stable from the start.
    let mut asset_ids: Vec<u64> = snap.compliance_records.keys().copied().collect();
    asset_ids.sort_unstable();

    let mut records: Vec<(f64, u64, AddressCompliance)> = Vec::new();

    for asset_id in asset_ids {
        let asset_records = &snap.compliance_records[&asset_id];
        if let Some(rec) = asset_records.get(&address) {
            if let Some(asset) = snap.asset(asset_id) {
                let balance = snap
                    .holders
                    .get(&asset_id)
                    .and_then(|holders| holders.iter().find(|h| h.address == address))
                    .map(|h| h.balance.clone())
                    .unwrap_or_else(|| "0".to_string());

                let share_percent = snap
                    .holders
                    .get(&asset_id)
                    .and_then(|holders| holders.iter().find(|h| h.address == address))
                    .map(|h| h.share_percent)
                    .unwrap_or(0.0);

                let allowed = derive_allowed(rec, latest_ledger, &blocked);
                records.push((
                    share_percent,
                    asset_id,
                    AddressCompliance {
                        address: address.clone(),
                        asset_id,
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
                    },
                ));
            }
        }
    }

    // Primary: share_percent descending. Secondary: asset_id ascending (deterministic).
    records.sort_by(|(sp_a, id_a, _), (sp_b, id_b, _)| {
        sp_b.partial_cmp(sp_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| id_a.cmp(id_b))
    });

    Ok(Json(records.into_iter().map(|(_, _, r)| r).collect()))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        extract::{Path, State},
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt as _;

    use super::summary;
    use crate::indexer::Snapshot;
    use crate::routes::test_support::{asset, state_with};
    use crate::routes::ApiError;

    #[tokio::test]
    async fn missing_asset_is_404() {
        let state = state_with(Snapshot::default());

        let err = summary(State(state), Path(7)).await.unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn present_asset_with_no_compliance_entry_is_default_summary() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(7));
        let state = state_with(snap);

        let body = summary(State(state), Path(7))
            .await
            .expect("asset exists")
            .0;

        assert_eq!(body.total_records, 0);
        assert_eq!(body.approved, 0);
        assert_eq!(body.suspended, 0);
        assert_eq!(body.rejected, 0);
        assert_eq!(body.pending, 0);
        assert_eq!(body.with_expiry, 0);
        assert!(body.jurisdictions.is_empty());
    }

    // #204 – non-numeric asset id returns 400, not 404
    #[tokio::test]
    async fn non_numeric_asset_id_returns_400_with_message() {
        let state = state_with(Snapshot::default());
        let app = Router::new()
            .route("/assets/:id/compliance", get(summary))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/abc/compliance")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "non-numeric id 'abc' should return 400, not {}",
            response.status()
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("id") || text.contains("abc"),
            "400 body should name the offending parameter; got: {text}"
        );
    }

    // -------------------------------------------------------------------------
    // #457 – real compliance status in for_address
    // -------------------------------------------------------------------------

    use super::for_address;
    use crate::models::{ComplianceRecord, Holder, Stats};
    use std::collections::HashMap;

    fn rec(status: &str, jurisdiction: &str, expires_at: u32) -> ComplianceRecord {
        ComplianceRecord {
            status: status.to_string(),
            jurisdiction: jurisdiction.to_string(),
            expires_at,
        }
    }

    fn snap_with_records(
        asset_id: u64,
        address: &str,
        cr: ComplianceRecord,
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
                vec![Holder {
                    address: address.to_string(),
                    balance: balance.to_string(),
                    share_percent: 100.0,
                }],
            );
        }
        let mut asset_records: HashMap<String, ComplianceRecord> = HashMap::new();
        asset_records.insert(address.to_string(), cr);
        snap.compliance_records.insert(asset_id, asset_records);
        snap
    }

    // #736 – worked example: a blocked jurisdiction flips `allowed` to false
    // even when the record is Approved and unexpired, mirroring the
    // compliance-guide worked example.
    #[tokio::test]
    async fn blocked_jurisdiction_is_not_allowed() {
        let mut snap = snap_with_records(1, "GADDR", rec("Approved", "KP", 0), 100, 10);
        // Simulate the chain exposing "KP" as a blocked jurisdiction.
        snap.blocked_jurisdictions.insert("KP".to_string());
        let state = state_with(snap);

        let body = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("address known")
            .0;

        assert_eq!(body.len(), 1);
        assert_eq!(body[0].status, "Approved");
        assert_eq!(body[0].jurisdiction.as_deref(), Some("KP"));
        assert!(
            !body[0].allowed,
            "blocked jurisdiction KP must yield allowed=false"
        );
    }
}
