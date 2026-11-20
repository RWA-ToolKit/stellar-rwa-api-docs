//! Indexer-specific Prometheus metrics beyond the refresh counters/histogram
//! recorded in `indexer::Indexer::run`.
//!
//! Exposed series (see `docs/app/docs/metrics`):
//! - `rwa_indexer_last_indexed_ledger` (gauge)
//! - `rwa_indexer_last_success_timestamp_seconds` (gauge, unix seconds)
//! - `rwa_indexer_last_success_age_seconds` (gauge, refreshed at scrape time)
//! - `rwa_indexer_snapshot_assets` (gauge)
//! - `rwa_indexer_snapshot_events` (gauge)

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::indexer::Snapshot;

/// Unix seconds of the last successful snapshot replacement (0 = never).
static LAST_SUCCESS_UNIX: AtomicU64 = AtomicU64::new(0);

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Record gauges for a freshly stored snapshot.
pub fn record_snapshot(snapshot: &Snapshot) {
    let now = now_unix();
    LAST_SUCCESS_UNIX.store(now, Ordering::Relaxed);
    metrics::gauge!("rwa_indexer_last_indexed_ledger").set(snapshot.stats.last_indexed_ledger as f64);
    metrics::gauge!("rwa_indexer_last_success_timestamp_seconds").set(now as f64);
    metrics::gauge!("rwa_indexer_snapshot_assets").set(snapshot.assets.len() as f64);
    metrics::gauge!("rwa_indexer_snapshot_events").set(snapshot.events.len() as f64);
}

/// Seconds since the last successful refresh, or `None` before the first one.
pub fn last_success_age_seconds() -> Option<u64> {
    match LAST_SUCCESS_UNIX.load(Ordering::Relaxed) {
        0 => None,
        t => Some(now_unix().saturating_sub(t)),
    }
}

/// Update scrape-time gauges; call immediately before rendering `/metrics`.
pub fn refresh_scrape_gauges() {
    if let Some(age) = last_success_age_seconds() {
        metrics::gauge!("rwa_indexer_last_success_age_seconds").set(age as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_is_none_or_small_after_record() {
        record_snapshot(&Snapshot::default());
        assert!(last_success_age_seconds().unwrap() < 5);
    }
}
