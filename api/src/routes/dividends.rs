//! `GET /assets/:id/dividends` and `GET /assets/:id/dividends/:did`.

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

/// A single distribution by id within an asset's dividend history.
pub async fn get_one(
    State(state): State<AppState>,
    Path((id, did)): Path<(u64, u64)>,
) -> Result<Json<Distribution>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    snap.dividends
        .get(&id)
        .and_then(|dists| dists.iter().find(|d| d.id == did))
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("no distribution {did} for asset {id}")))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::routes::{router, test_support::test_state};

    /// Mirrors the existing coverage for a non-numeric `:id` on `GET /assets/:id`
    /// (400, rejected by axum's path extractor before the handler runs) but for
    /// the distribution id segment of `GET /assets/:id/dividends/:did`.
    #[tokio::test]
    async fn non_numeric_distribution_id_is_rejected_with_400() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/1/dividends/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_one_404s_for_unknown_asset() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/999/dividends/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_one_404s_for_unknown_distribution() {
        let app = router(crate::routes::test_support::test_state_with_asset(1));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/1/dividends/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "not_found");
    }
}
