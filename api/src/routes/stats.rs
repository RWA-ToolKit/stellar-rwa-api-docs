//! `GET /stats`.

use axum::{extract::State, Json};

use crate::indexer::AppState;
use crate::models::Stats;

/// Platform-wide statistics: asset count, TVL, holders, distributions.
pub async fn get(State(state): State<AppState>) -> Json<Stats> {
    let snap = state.snapshot();
    Json(snap.stats)
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
    async fn stats_empty_snapshot_returns_defaults() {
        let state = state_with(Snapshot::default());

        let body = get(State(state)).await.0;
        let value = serde_json::to_value(&body).expect("serialize stats");

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
}
