//! Measurement of in-memory snapshot growth and per-request clone cost.
//!
//! Size is approximated by the JSON-serialized length of the snapshot's
//! collections, which tracks heap payload (strings dominate) and is stable
//! across allocator/platform. Recorded on every refresh as:
//! - `rwa_indexer_snapshot_estimated_bytes` (gauge)
//! - `rwa_indexer_snapshot_clone_duration_seconds` (histogram): cost of one
//!   full `Snapshot::clone()`, i.e. what each request pays today.
//!
//! Strategy and measurements: `docs/app/docs/snapshot-memory`.

use std::time::Instant;

use crate::indexer::Snapshot;

/// Approximate payload size of `snapshot` in bytes.
pub fn estimate_bytes(snapshot: &Snapshot) -> usize {
    fn len<T: serde::Serialize>(v: &T) -> usize {
        serde_json::to_vec(v).map(|b| b.len()).unwrap_or(0)
    }
    len(&snapshot.assets)
        + len(&snapshot.holders)
        + len(&snapshot.compliance)
        + len(&snapshot.compliance_records)
        + len(&snapshot.dividends)
        + len(&snapshot.events)
        + len(&snapshot.stats)
}

/// Record size and clone-cost metrics for a freshly built snapshot.
pub fn record(snapshot: &Snapshot) {
    metrics::gauge!("rwa_indexer_snapshot_estimated_bytes").set(estimate_bytes(snapshot) as f64);
    let started = Instant::now();
    let cloned = snapshot.clone();
    metrics::histogram!("rwa_indexer_snapshot_clone_duration_seconds")
        .record(started.elapsed().as_secs_f64());
    drop(cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Holder;

    #[test]
    fn estimate_grows_with_registry_size() {
        let empty = Snapshot::default();
        let mut bigger = Snapshot::default();
        for id in 0..50u64 {
            bigger.holders.insert(id, Vec::<Holder>::new());
        }
        assert!(estimate_bytes(&bigger) > estimate_bytes(&empty));
        record(&bigger);
    }
}
