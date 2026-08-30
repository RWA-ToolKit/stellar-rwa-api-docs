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
/// `GET /assets/:id/distributions/:did` — a single distribution by id.
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
    let dist = snap
        .dividends
        .get(&id)
        .and_then(|dists| dists.iter().find(|d| d.id == did))
        .cloned()
        .ok_or_else(|| {
            ApiError::NotFound(format!("no distribution with id {did} for asset {id}"))
        })?;
    Ok(Json(dist))
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

    /// Distribution ids come from on-chain state, so a malicious or malformed
    /// id must 404 cleanly rather than panic or wrap around — cover both ends
    /// of the u64 range, mirroring the asset-id boundary coverage (#194/#210).
    #[tokio::test]
    async fn get_one_404s_cleanly_at_distribution_id_boundaries() {
        let app = router(crate::routes::test_support::test_state_with_asset(1));
        for did in [0u64, u64::MAX] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/assets/1/dividends/{did}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "did={did}");
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json["error"], "not_found", "did={did}");
        }
    }

    /// Same boundary values, but against an asset that doesn't exist either —
    /// the asset-not-found branch must still win cleanly, not panic.
    #[tokio::test]
    async fn get_one_404s_cleanly_at_distribution_id_boundaries_unknown_asset() {
        let app = router(test_state());
        for did in [0u64, u64::MAX] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/assets/999/dividends/{did}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "did={did}");
    use axum::extract::{Path, State};

    use super::{get_one, list};
    use crate::indexer::Snapshot;
    use crate::routes::test_support::{asset, distribution, state_with};

    #[tokio::test]
    async fn single_distribution_returned_by_id() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.dividends
            .insert(1, vec![distribution(10, 100), distribution(11, 300)]);
        let state = state_with(snap);

        let dist = get_one(State(state), Path((1, 11)))
            .await
            .expect("distribution exists")
            .0;

        assert_eq!(dist.id, 11);
    }

    #[tokio::test]
    async fn single_distribution_404_when_unknown_id() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        snap.dividends.insert(1, vec![distribution(10, 100)]);
        let state = state_with(snap);

        let result = get_one(State(state), Path((1, 999))).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn single_distribution_404_when_asset_unknown() {
        let state = state_with(Snapshot::default());

        let result = get_one(State(state), Path((1, 10))).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn distributions_sorted_by_created_at_ledger_descending() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        // Deliberately out of order, so a passing assertion proves the
        // handler sorts rather than preserving insertion order.
        snap.dividends.insert(
            1,
            vec![
                distribution(10, 100),
                distribution(11, 300),
                distribution(12, 200),
            ],
        );
        let state = state_with(snap);

        let dists = list(State(state), Path(1)).await.expect("asset exists").0;

        let ledgers: Vec<u32> = dists.iter().map(|d| d.created_at_ledger).collect();
        assert_eq!(ledgers, vec![300, 200, 100]);
    }

    // #210 – boundary ids (0 and u64::MAX) return 404 cleanly without
    // trapping or overflowing, and the body names the offending id.
    #[tokio::test]
    async fn boundary_asset_ids_return_404_cleanly() {
        use axum::{http::StatusCode, response::IntoResponse};

        for id in [0u64, u64::MAX] {
            let state = state_with(Snapshot::default());

            let response = match list(State(state), Path(id)).await {
                Ok(_) => panic!("boundary asset id {id} must not resolve to assets"),
                Err(err) => err.into_response(),
            };

            assert_eq!(
                response.status(),
                StatusCode::NOT_FOUND,
                "asset id {id} should return 404, not {}",
                response.status()
            );

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let text = String::from_utf8_lossy(&body);
            assert!(
                text.contains(&id.to_string()),
                "404 body should name the offending id; got: {text}"
            );
        }
    }
}
