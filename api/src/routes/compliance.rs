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
pub async fn for_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<AddressCompliance>>, ApiError> {
    let snap = state.snapshot();
    let latest_ledger = snap.stats.last_indexed_ledger;
    // Blocked jurisdictions are not yet fetched from the chain; default to
    // empty so we don't incorrectly gate anyone, and document this clearly.
    let blocked: HashSet<String> = HashSet::new();

    let mut records = Vec::new();

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
                records.push(AddressCompliance {
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

    records.sort_by(|a, b| {
        b.balance
            .parse::<i128>()
            .unwrap_or_default()
            .cmp(&a.balance.parse::<i128>().unwrap_or_default())
    });

    Ok(Json(records))
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
        } else {
            snap.holders.insert(asset_id, vec![]);
        }
        let mut crecs = HashMap::new();
        crecs.insert(address.to_string(), cr);
        snap.compliance_records.insert(asset_id, crecs);
        snap
    }

    /// An approved, non-expired address with a positive balance returns
    /// allowed: true.
    #[tokio::test]
    async fn approved_address_is_allowed_true() {
        let snap = snap_with_records(1, "GADDR", rec("Approved", "US", 0), 1_000, 100);
        let state = state_with(snap);

        let result = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "Approved");
        assert!(result[0].allowed, "Approved with no expiry must be allowed");
        assert_eq!(result[0].balance, "1000");
    }

    /// A suspended address must have allowed: false regardless of balance.
    #[tokio::test]
    async fn suspended_address_is_not_allowed() {
        let snap = snap_with_records(1, "GADDR", rec("Suspended", "US", 0), 500, 100);
        let state = state_with(snap);

        let result = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "Suspended");
        assert!(!result[0].allowed, "Suspended address must not be allowed");
    }

    /// An Approved address whose `expires_at` is in the past (≤ latest_ledger)
    /// must not be allowed.
    #[tokio::test]
    async fn expired_approval_is_not_allowed() {
        // expires_at=50, latest_ledger=100 → expired
        let snap = snap_with_records(1, "GADDR", rec("Approved", "US", 50), 100, 100);
        let state = state_with(snap);

        let result = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "Approved");
        assert!(!result[0].allowed, "Expired approval must not be allowed");
        assert_eq!(
            result[0].expires_at,
            Some(50),
            "expires_at must be surfaced in the response"
        );
    }

    /// An Approved address whose `expires_at` is in the future must be allowed.
    #[tokio::test]
    async fn approval_expiring_in_future_is_allowed() {
        // expires_at=200, latest_ledger=100 → still valid
        let snap = snap_with_records(1, "GADDR", rec("Approved", "US", 200), 100, 100);
        let state = state_with(snap);

        let result = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1);
        assert!(result[0].allowed, "Approval expiring at ledger 200 must still be allowed at ledger 100");
    }

    /// An allowlisted address with a zero balance must still appear in the
    /// response (balance "0") — distinguishable from an unknown address.
    #[tokio::test]
    async fn zero_balance_address_appears_with_balance_zero() {
        // balance=0 → not in holders, but is in compliance_records
        let snap = snap_with_records(1, "GADDR", rec("Approved", "SG", 0), 0, 100);
        let state = state_with(snap);

        let result = for_address(State(state), Path("GADDR".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert_eq!(result.len(), 1, "zero-balance allowlisted address must appear");
        assert_eq!(result[0].balance, "0", "balance must be '0'");
        assert!(result[0].allowed);
    }

    /// An address that is completely unknown returns an empty array, not 404.
    #[tokio::test]
    async fn unknown_address_returns_empty_array() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.holders.insert(1, vec![]);
        // no compliance_records entry for "GUNKNOWN"
        let state = state_with(snap);

        let result = for_address(State(state), Path("GUNKNOWN".to_string()))
            .await
            .expect("should succeed")
            .0;

        assert!(
            result.is_empty(),
            "unknown address must return empty array, not 404"
        );
    }
}
