// Issue #429 — benchmark harness for realistic registry sizes
//
// Measures the latency of cloning/reading an in-memory snapshot at four
// realistic registry sizes (1, 10, 100, 500 assets).  This is the hot path
// that every route handler exercises on each request: `AppState::snapshot()`
// hands out an `Arc<Snapshot>` via ArcSwap, and routes then read the fields
// they need.  The clone here represents the `Arc::clone` inside `load_full()`
// plus the subsequent field traversal, which is the minimum work every
// route performs.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;

use stellar_rwa_api::indexer::{Snapshot, StaleFlags};
use stellar_rwa_api::models::{
    Asset, ComplianceSummary, Distribution, Holder, JurisdictionCount, Stats,
};

/// Build a synthetic in-memory snapshot with `n_assets` assets and
/// `holders_per_asset` holders per asset.  All values are deterministic so
/// the benchmark is reproducible across runs.
fn make_snapshot(n_assets: usize, holders_per_asset: usize) -> Snapshot {
    let mut assets = Vec::with_capacity(n_assets);
    let mut holders_map: HashMap<u64, Vec<Holder>> = HashMap::with_capacity(n_assets);
    let mut compliance_map: HashMap<u64, ComplianceSummary> = HashMap::with_capacity(n_assets);
    let mut dividends_map: HashMap<u64, Vec<Distribution>> = HashMap::with_capacity(n_assets);

    for i in 0..n_assets {
        let id = i as u64 + 1;
        let valuation = 1_000_000i128 * (id as i128);

        assets.push(Asset {
            id,
            token_contract: format!(
                "C{:055}",
                id // padded contract-like string
            ),
            issuer: format!(
                "G{:055}",
                id
            ),
            name: format!("Synthetic Asset {id}"),
            symbol: format!("SYN{id}"),
            asset_type: "real_estate".to_string(),
            description: format!("Benchmark synthetic asset number {id}"),
            valuation_cents: valuation.to_string(),
            valuation_usd: (valuation / 100) as f64,
            decimals: 7,
            total_supply: (1_000_000_000i128 * id as i128).to_string(),
            holders: holders_per_asset,
            active: true,
            paused: false,
            compliance_contract: format!("CC{:054}", id),
            created_at_ledger: 1_000_000 + i as u32,
            indexed_at_ledger: 1_000_000 + i as u32,
            dividends_indexed_at_ledger: None,
            index_error: None,
        });

        let holders: Vec<Holder> = (0..holders_per_asset)
            .map(|h| Holder {
                address: format!("G{:054}{:01}", id, h),
                balance: (100_000i128 * (h as i128 + 1)).to_string(),
                share_percent: 100.0 / holders_per_asset as f64,
            })
            .collect();
        holders_map.insert(id, holders);

        compliance_map.insert(
            id,
            ComplianceSummary {
                total_records: holders_per_asset,
                approved: holders_per_asset,
                suspended: 0,
                rejected: 0,
                pending: 0,
                with_expiry: holders_per_asset / 2,
                jurisdictions: vec![
                    JurisdictionCount {
                        jurisdiction: "US".to_string(),
                        count: holders_per_asset / 2,
                    },
                    JurisdictionCount {
                        jurisdiction: "SG".to_string(),
                        count: holders_per_asset - holders_per_asset / 2,
                    },
                ],
            },
        );

        let dists: Vec<Distribution> = (0..2)
            .map(|d| Distribution {
                id: id * 10 + d,
                asset_token: format!("C{:055}", id),
                payment_token: "USDC".to_string(),
                total_amount: "1000000".to_string(),
                distributed: "500000".to_string(),
                claimed_percent: 50.0,
                overflow_detected: false,
                completed: false,
                created_at_ledger: 1_000_000 + i as u32,
            })
            .collect();
        dividends_map.insert(id, dists);
    }

    let tvl: i128 = assets
        .iter()
        .filter(|a| a.active)
        .map(|a| a.valuation_cents.parse::<i128>().unwrap_or(0))
        .sum();

    Snapshot {
        stats: Stats {
            total_assets: n_assets,
            active_assets: n_assets,
            tvl_cents: tvl.to_string(),
            tvl_usd: (tvl / 100) as f64,
            total_holders: n_assets * holders_per_asset,
            total_distributions: n_assets * 2,
            last_indexed_ledger: 2_000_000,
            last_updated: Some("2026-01-01T00:00:00Z".to_string()),
        },
        assets,
        holders: holders_map,
        compliance: compliance_map,
        dividends: dividends_map,
        events: Vec::new(),
        stale_flags: StaleFlags::default(),
    }
}

fn bench_snapshot_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_clone");

    for n_assets in [1usize, 10, 100, 500] {
        let snapshot = make_snapshot(n_assets, 10);
        group.bench_with_input(
            BenchmarkId::new("n_assets", n_assets),
            &snapshot,
            |b, snap| {
                b.iter(|| {
                    // This is the hot path: clone the snapshot (Arc clone in
                    // the real route path) and read the fields routes need.
                    let cloned = black_box(snap.clone());
                    black_box(cloned.stats.total_assets);
                    black_box(cloned.assets.len());
                });
            },
        );
    }

    group.finish();
}

fn bench_snapshot_asset_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_asset_lookup");

    for n_assets in [1usize, 10, 100, 500] {
        let snapshot = make_snapshot(n_assets, 10);
        // Look up an asset near the middle of the list.
        let target_id = (n_assets / 2) as u64 + 1;
        group.bench_with_input(
            BenchmarkId::new("n_assets", n_assets),
            &snapshot,
            |b, snap| {
                b.iter(|| {
                    // Linear scan — mirrors AppState::snapshot().asset(id) in
                    // the real route handler.
                    black_box(snap.asset(black_box(target_id)));
                });
            },
        );
    }

    group.finish();
}

fn bench_snapshot_holders_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_holders_read");

    for n_assets in [1usize, 10, 100, 500] {
        let snapshot = make_snapshot(n_assets, 50);
        let target_id = (n_assets / 2) as u64 + 1;
        group.bench_with_input(
            BenchmarkId::new("n_assets", n_assets),
            &snapshot,
            |b, snap| {
                b.iter(|| {
                    black_box(snap.holders.get(&black_box(target_id)));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_snapshot_clone,
    bench_snapshot_asset_lookup,
    bench_snapshot_holders_read
);
criterion_main!(benches);
