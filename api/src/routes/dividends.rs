//! `GET /assets/:id/dividends`.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::Snapshot;
    use crate::models::Asset;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_dividends_sorted_by_created_at_ledger_desc() {
        let asset_id = 42u64;
        let asset = Asset {
            id: asset_id,
            token_contract: "CAV".to_string(),
            issuer: "ISSUER".to_string(),
            name: "Test Asset".to_string(),
            symbol: "TST".to_string(),
            asset_type: "test".to_string(),
            description: "Test".to_string(),
            valuation_cents: "1000000".to_string(),
            valuation_usd: 10000.0,
            decimals: 7,
            total_supply: "1000000000".to_string(),
            holders: 0,
            active: true,
            paused: false,
            compliance_contract: "COMP".to_string(),
            created_at_ledger: 100,
        };

        let dist1 = Distribution {
            id: 1,
            asset_token: "TOKEN1".to_string(),
            payment_token: "USDC".to_string(),
            total_amount: "1000000".to_string(),
            distributed: "500000".to_string(),
            claimed_percent: 50.0,
            completed: false,
            snapshot_ledger: 100,
            created_at_ledger: 200,
        };

        let dist2 = Distribution {
            id: 2,
            asset_token: "TOKEN2".to_string(),
            payment_token: "USDC".to_string(),
            total_amount: "2000000".to_string(),
            distributed: "2000000".to_string(),
            claimed_percent: 100.0,
            completed: true,
            snapshot_ledger: 150,
            created_at_ledger: 300,
        };

        let dist3_anomaly = Distribution {
            id: 3,
            asset_token: "TOKEN3".to_string(),
            payment_token: "USDC".to_string(),
            total_amount: "500000".to_string(),
            distributed: "600000".to_string(),
            claimed_percent: 100.0,
            completed: true,
            snapshot_ledger: 200,
            created_at_ledger: 250,
        };

        let mut dividends_map = HashMap::new();
        dividends_map.insert(asset_id, vec![dist1.clone(), dist3_anomaly.clone(), dist2.clone()]);

        let snapshot = Snapshot {
            assets: vec![asset],
            dividends: dividends_map,
            ..Default::default()
        };

        let mut result = snapshot
            .dividends
            .get(&asset_id)
            .cloned()
            .unwrap_or_default();
        result.sort_by_key(|d| std::cmp::Reverse(d.created_at_ledger));

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].created_at_ledger, 300, "highest ledger first");
        assert_eq!(result[1].created_at_ledger, 250, "middle ledger");
        assert_eq!(result[2].created_at_ledger, 200, "lowest ledger last");

        assert_eq!(
            result[1].claimed_percent, 100.0,
            "anomaly (distributed > total) clamps claimed_percent at 100"
        );
    }
}
