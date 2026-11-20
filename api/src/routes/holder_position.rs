//! `GET /holders/:address/position` — a holder's full position across assets.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::indexer::AppState;

/// One asset position with the holder's estimated unclaimed dividends.
#[derive(Debug, Serialize)]
pub struct PositionEntry {
    pub asset_id: u64,
    pub asset_name: String,
    pub symbol: String,
    pub balance: String,
    pub share_percent: f64,
    /// Estimated value of the position in USD (`share_percent` of valuation).
    pub value_usd: f64,
    /// Pro-rata share of the unclaimed pool of each open distribution.
    pub claimable: Vec<Claimable>,
}

#[derive(Debug, Serialize)]
pub struct Claimable {
    pub distribution_id: u64,
    pub payment_token: String,
    /// Unclaimed base units of the payment token attributable to this holder.
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct HolderPosition {
    pub address: String,
    pub asset_count: usize,
    pub total_value_usd: f64,
    pub positions: Vec<PositionEntry>,
}

fn parse(value: &str) -> i128 {
    value.parse::<i128>().unwrap_or_default()
}

/// Aggregate position for an address; an address holding nothing gets an
/// empty position (matching `GET /holders/:address`).
pub async fn get(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Json<HolderPosition> {
    let snap = state.snapshot();
    let mut positions = Vec::new();

    for (asset_id, holders) in &snap.holders {
        let (Some(holder), Some(asset)) = (
            holders.iter().find(|h| h.address == address),
            snap.asset(*asset_id),
        ) else {
            continue;
        };
        let supply = parse(&asset.total_supply);
        let balance = parse(&holder.balance);
        let mut claimable = Vec::new();
        if supply > 0 {
            for d in snap.dividends.get(asset_id).into_iter().flatten() {
                let remaining = parse(&d.total_amount) - parse(&d.distributed);
                if d.completed || remaining <= 0 {
                    continue;
                }
                claimable.push(Claimable {
                    distribution_id: d.id,
                    payment_token: d.payment_token.clone(),
                    amount: (remaining * balance / supply).to_string(),
                });
            }
        }
        positions.push(PositionEntry {
            asset_id: *asset_id,
            asset_name: asset.name.clone(),
            symbol: asset.symbol.clone(),
            balance: holder.balance.clone(),
            share_percent: holder.share_percent,
            value_usd: asset.valuation_usd * holder.share_percent / 100.0,
            claimable,
        });
    }

    positions.sort_by(|a, b| {
        parse(&b.balance)
            .cmp(&parse(&a.balance))
            .then(a.asset_id.cmp(&b.asset_id))
    });
    Json(HolderPosition {
        address,
        asset_count: positions.len(),
        total_value_usd: positions.iter().map(|p| p.value_usd).sum(),
        positions,
    })
}
