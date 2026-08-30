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
    use crate::models::{ComplianceSummary, JurisdictionCount};
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

    // Route-level coverage for jurisdiction aggregation: the indexer's
    // aggregation logic is covered in `indexer::mod::compliance_summary_counts_by_status`,
    // but that never exercises the HTTP handler's JSON serialization. This
    // seeds `snap.compliance` directly and asserts the populated
    // `jurisdictions` entries round-trip through the route unchanged.
    #[tokio::test]
    async fn jurisdictions_round_trip_through_the_http_response() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(9));
        snap.compliance.insert(
            9,
            ComplianceSummary {
                total_records: 100,
                approved: 60,
                suspended: 15,
                rejected: 10,
                pending: 15,
                with_expiry: 25,
                jurisdictions: vec![
                    JurisdictionCount {
                        jurisdiction: "US".to_string(),
                        count: 50,
                    },
                    JurisdictionCount {
                        jurisdiction: "SG".to_string(),
                        count: 30,
                    },
                    JurisdictionCount {
                        jurisdiction: "UK".to_string(),
                        count: 20,
                    },
                ],
            },
        );
        let state = state_with(snap);
        let app = Router::new()
            .route("/assets/:id/compliance", get(summary))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/9/compliance")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response must be valid JSON");

        assert_eq!(body["total_records"], 100);
        assert_eq!(body["approved"], 60);
        assert_eq!(body["suspended"], 15);
        assert_eq!(body["rejected"], 10);
        assert_eq!(body["pending"], 15);
        assert_eq!(body["with_expiry"], 25);

        let jurisdictions = body["jurisdictions"]
            .as_array()
            .expect("jurisdictions must be an array");
        assert_eq!(jurisdictions.len(), 3);
        assert_eq!(jurisdictions[0]["jurisdiction"], "US");
        assert_eq!(jurisdictions[0]["count"], 50);
        assert_eq!(jurisdictions[1]["jurisdiction"], "SG");
        assert_eq!(jurisdictions[1]["count"], 30);
        assert_eq!(jurisdictions[2]["jurisdiction"], "UK");
        assert_eq!(jurisdictions[2]["count"], 20);
    }
}
