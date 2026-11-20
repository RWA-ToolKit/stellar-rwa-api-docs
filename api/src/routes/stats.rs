//! `GET /stats`.

use axum::{extract::State, Json};

use crate::indexer::AppState;
use crate::models::Stats;

/// Platform-wide statistics: asset count, TVL, holders, distributions.
pub async fn get(State(state): State<AppState>) -> Json<Stats> {
    let snap = state.snapshot();
    Json(snap.stats.clone())
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use serde_json::json;

    use super::get;
    use crate::indexer::Snapshot;
    use crate::models::Stats;
    use crate::routes::test_support::state_with;

    #[tokio::test]
    async fn stats_json_shape() {
        let snap = Snapshot {
            stats: Stats {
                total_assets: 3,
                active_assets: 2,
                tvl_cents: "123456789".to_string(),
                tvl_usd: 1_234_567.89,
                total_holders: 10,
                total_distributions: 4,
                last_indexed_ledger: 555,
                last_updated: Some("2024-01-01T00:00:00Z".to_string()),
            },
            ..Snapshot::default()
        };
        let state = state_with(snap);

        let body = get(State(state)).await.0;
        let value = serde_json::to_value(&body).expect("serialize stats");

        assert!(value["tvl_cents"].is_string());
        assert_eq!(value["tvl_cents"], json!("123456789"));
        assert!(value["tvl_usd"].is_number());
        assert_eq!(value["tvl_usd"], json!(1_234_567.89));
        assert_eq!(value["last_updated"], json!("2024-01-01T00:00:00Z"));
    }

    #[tokio::test]
    async fn stats_empty_snapshot_reports_zeroed_defaults() {
        let state = state_with(Snapshot::default());

        let body = get(State(state)).await.0;
        let value = serde_json::to_value(&body).expect("serialize stats");

        assert!(
            value.is_object(),
            "empty snapshot should still serialize as JSON object"
        );
        assert_eq!(value["total_assets"], 0);
        assert_eq!(value["active_assets"], 0);
        assert_eq!(value["tvl_cents"], "");
        assert_eq!(value["tvl_usd"], 0.0);
        assert_eq!(value["total_holders"], 0);
        assert_eq!(value["total_distributions"], 0);
        assert_eq!(value["last_indexed_ledger"], 0);
        assert!(value["last_updated"].is_null());
    }

    /// The serialized key set must exactly match the OpenAPI schema's
    /// `required` list (docs/public/openapi.json → components.schemas.Stats),
    /// so a model/spec drift fails CI instead of shipping (see #187).
    #[tokio::test]
    async fn stats_response_key_set_matches_openapi_schema() {
        let snap = Snapshot {
            stats: Stats {
                total_assets: 3,
                active_assets: 2,
                tvl_cents: "123456789".to_string(),
                tvl_usd: 1_234_567.89,
                total_holders: 10,
                total_distributions: 4,
                last_indexed_ledger: 555,
                last_updated: Some("2024-01-01T00:00:00Z".to_string()),
            },
            ..Snapshot::default()
        };
        let state = state_with(snap);

        let body = get(State(state)).await.0;
        let value = serde_json::to_value(&body).expect("serialize stats");

        let openapi: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/public/openapi.json"))
                .expect("docs/public/openapi.json is valid JSON");
        let required = openapi["components"]["schemas"]["Stats"]["required"]
            .as_array()
            .expect("Stats schema has a required list")
            .iter()
            .map(|v| v.as_str().expect("required entries are strings"))
            .collect::<Vec<_>>();

        let mut actual = value
            .as_object()
            .expect("serialized stats is a JSON object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut expected = required;
        actual.sort_unstable();
        expected.sort_unstable();

        assert_eq!(
            actual, expected,
            "serialized /stats keys must exactly match the OpenAPI schema's required list"
        );
    }

    /// Drive the stats route to the configured rate-limit boundary and confirm
    /// the 429 response uses the shared error body shape (see #555).
    #[tokio::test]
    async fn stats_rate_limit_boundary_returns_shared_error_body() {
        use crate::routes::test_support::rate_limited_router;

        let state = state_with(Snapshot::default());
        let (router, limit) = rate_limited_router(state, get);

        // Requests up to the configured limit succeed.
        for i in 0..limit {
            let resp = router
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri("/stats")
                        .body(axum::body::Body::empty())
                        .expect("build request"),
                )
                .await
                .expect("request within limit succeeds");
            assert_eq!(
                resp.status(),
                axum::http::StatusCode::OK,
                "request {} of {} should be within the rate limit",
                i + 1,
                limit
            );
        }

        // The next request crosses the boundary and must be rejected.
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/stats")
                    .body(axum::body::Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("boundary request completes");
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "request past the configured limit must be rate limited"
        );

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read 429 body");
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("429 body is valid JSON");

        // Shared error body shape: { "error": { "code": ..., "message": ... } }.
        let error = value
            .get("error")
            .and_then(|v| v.as_object())
            .expect("429 body has an `error` object");
        assert!(
            error.get("code").and_then(|v| v.as_str()).is_some(),
            "429 error body must include a string `code`"
        );
        assert!(
            error.get("message").and_then(|v| v.as_str()).is_some(),
            "429 error body must include a string `message`"
        );
    }
}
