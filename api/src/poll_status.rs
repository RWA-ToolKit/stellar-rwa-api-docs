//! Process-wide record of indexer poll outcomes, read by `/health`.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// Environment variable overriding the stall threshold, in seconds.
pub const MAX_POLL_AGE_ENV: &str = "RWA_HEALTH_MAX_POLL_AGE_SECS";
/// Approximate Stellar ledger close time, used to estimate ledger lag.
const LEDGER_SECS: i64 = 5;

static LAST_SUCCESS_UNIX: AtomicU64 = AtomicU64::new(0);
static LAST_LEDGER: AtomicU32 = AtomicU32::new(0);
static CONSECUTIVE_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Consecutive failed polls since the last success.
pub fn consecutive_failures() -> u32 {
    CONSECUTIVE_FAILURES.load(Ordering::Relaxed)
}

/// Record a failed poll and return the new consecutive-failure count.
pub fn record_failure() -> u32 {
    CONSECUTIVE_FAILURES.fetch_add(1, Ordering::Relaxed) + 1
}

/// Record a successful poll that indexed up to `ledger`. Logs recovery when
/// the previous polls had failed.
pub fn record_success(ledger: u32) {
    let failures = CONSECUTIVE_FAILURES.swap(0, Ordering::Relaxed);
    if failures > 0 {
        tracing::info!(
            consecutive_failures = failures,
            ledger,
            "indexer recovered after failed polls"
        );
    }
    LAST_SUCCESS_UNIX.store(chrono::Utc::now().timestamp().max(0) as u64, Ordering::Relaxed);
    LAST_LEDGER.store(ledger, Ordering::Relaxed);
}

/// Seconds since the last successful poll, or `None` if none has succeeded.
pub fn last_poll_age_seconds() -> Option<i64> {
    match LAST_SUCCESS_UNIX.load(Ordering::Relaxed) {
        0 => None,
        t => Some((chrono::Utc::now().timestamp() - t as i64).max(0)),
    }
}

/// RFC 3339 time of the last successful poll.
pub fn last_poll_at() -> Option<String> {
    match LAST_SUCCESS_UNIX.load(Ordering::Relaxed) {
        0 => None,
        t => chrono::DateTime::from_timestamp(t as i64, 0).map(|d| d.to_rfc3339()),
    }
}

/// Ledger reached by the last successful poll.
pub fn last_ledger() -> u32 {
    LAST_LEDGER.load(Ordering::Relaxed)
}

/// Estimated number of ledgers closed since the last successful poll.
pub fn estimated_ledger_lag() -> Option<i64> {
    last_poll_age_seconds().map(|s| s / LEDGER_SECS)
}

/// Maximum tolerated time since the last successful poll before the indexer
/// is reported unhealthy. Defaults to three poll intervals.
pub fn max_poll_age(poll_interval: Duration) -> i64 {
    std::env::var(MAX_POLL_AGE_ENV)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or((poll_interval * 3).as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_poll_age_defaults_to_three_intervals() {
        std::env::remove_var(MAX_POLL_AGE_ENV);
        assert_eq!(max_poll_age(Duration::from_secs(10)), 30);
    }
}
