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
}
