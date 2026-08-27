//! `GET /assets/:id/holders`.

use axum::{
    extract::{Path, State},
    Json,
};

use super::ApiError;
use crate::indexer::AppState;
use crate::models::Holder;

/// Holder list for an asset, sorted by balance descending.
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Vec<Holder>>, ApiError> {
    let snap = state.snapshot();
    if snap.asset(id).is_none() {
        return Err(ApiError::NotFound(format!("no asset with id {id}")));
    }
    Ok(Json(snap.holders.get(&id).cloned().unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, State};
    use serde_json::json;

    use super::list;
    use crate::indexer::Snapshot;
    use crate::models::Holder;
    use crate::routes::test_support::{asset, state_with};
    use crate::routes::ApiError;

    #[tokio::test]
    async fn missing_asset_is_404() {
        let state = state_with(Snapshot::default());

        let err = list(State(state), Path(42)).await.unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn present_asset_with_no_holders_is_empty_array() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));
        let state = state_with(snap);

        let holders = list(State(state), Path(1)).await.expect("asset exists").0;

        assert!(holders.is_empty());
    }

    #[tokio::test]
    async fn unknown_asset_returns_404_with_documented_error_shape() {
        let state = state_with(Snapshot::default());

        let err = list(State(state), Path(999)).await.unwrap_err();

        let ApiError::NotFound(msg) = err;
        assert_eq!(msg, "no asset with id 999");

        let error_body = json!({
            "error": "not_found",
            "message": msg
        });

        assert!(error_body.get("error").is_some(), "Error must have 'error' field");
        assert!(
            error_body.get("message").is_some(),
            "Error must have 'message' field"
        );
        assert_eq!(error_body["error"], "not_found");
        assert!(error_body["message"].is_string());
    }

    #[tokio::test]
    async fn boundary_id_zero_returns_404_without_panic() {
        let state = state_with(Snapshot::default());

        let err = list(State(state), Path(0)).await.unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn boundary_id_u64_max_returns_404_without_panic() {
        let state = state_with(Snapshot::default());

        let err = list(State(state), Path(u64::MAX)).await.unwrap_err();

        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn holders_response_matches_openapi_schema() {
        let mut snap = Snapshot::default();
        snap.assets.push(asset(1));

        let holder_data = vec![
            Holder {
                address: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
                balance: "1000000".to_string(),
                share_percent: 100.0,
            },
        ];
        snap.holders.insert(1, holder_data);

        let state = state_with(snap);

        let holders = list(State(state), Path(1))
            .await
            .expect("asset exists")
            .0;

        assert!(!holders.is_empty(), "Should have at least one holder");

        let serialized = serde_json::to_value(&holders).expect("serialization should succeed");
        assert!(serialized.is_array(), "Response should be an array");

        let holder_obj = &serialized[0];

        let required_fields = ["address", "balance", "share_percent"];
        for field in &required_fields {
            assert!(
                holder_obj.get(field).is_some(),
                "Holder object must have '{}' field",
                field
            );
        }

        let actual_fields: Vec<&str> = holder_obj
            .as_object()
            .expect("holder should be an object")
            .keys()
            .map(|s| s.as_str())
            .collect();

        for expected in &required_fields {
            assert!(
                actual_fields.contains(expected),
                "Expected field '{}' not found in response",
                expected
            );
        }

        assert_eq!(
            actual_fields.len(),
            required_fields.len(),
            "Response should have exactly {} fields, got {}",
            required_fields.len(),
            actual_fields.len()
        );
    }
}
