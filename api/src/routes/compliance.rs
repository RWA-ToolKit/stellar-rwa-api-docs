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
        assert_eq!(body.unread, 0);
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
}
