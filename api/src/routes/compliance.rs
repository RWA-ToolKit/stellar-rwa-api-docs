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
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.compliance.get(&id).cloned().unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, State};

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

    #[tokio::test]
    async fn compliance_counts_sum_to_total_records() {
        use crate::models::{ComplianceSummary, JurisdictionCount};

        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.compliance.insert(
            1,
            ComplianceSummary {
                total_records: 15,
                approved: 7,
                suspended: 3,
                rejected: 2,
                pending: 3,
                with_expiry: 4,
                jurisdictions: vec![
                    JurisdictionCount {
                        jurisdiction: "US".to_string(),
                        count: 10,
                    },
                    JurisdictionCount {
                        jurisdiction: "EU".to_string(),
                        count: 5,
                    },
                ],
            },
        );
        let state = state_with(snap);

        let body = summary(State(state), Path(1)).await.expect("asset exists").0;
        let value = serde_json::to_value(&body).expect("serialize compliance summary");

        let approved = value["approved"].as_u64().unwrap();
        let suspended = value["suspended"].as_u64().unwrap();
        let rejected = value["rejected"].as_u64().unwrap();
        let pending = value["pending"].as_u64().unwrap();
        let total_records = value["total_records"].as_u64().unwrap();

        assert_eq!(
            approved + suspended + rejected + pending,
            total_records,
            "approved + suspended + rejected + pending must sum to total_records"
        );
    }
}
