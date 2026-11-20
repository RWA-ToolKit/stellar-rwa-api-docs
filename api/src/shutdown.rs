//! Bounds for graceful shutdown: in-flight requests and the indexer get a
//! limited time to finish once a shutdown signal has been received.

use std::{future::Future, time::Duration};

use tokio::{sync::watch, task::JoinHandle};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Grace period, from `RWA_SHUTDOWN_TIMEOUT_SECS` (default 30).
pub fn timeout() -> Duration {
    Duration::from_secs(
        std::env::var("RWA_SHUTDOWN_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS),
    )
}

/// Run `server` to completion, but once `shutdown` flips to `true` give it at
/// most `limit` to drain before abandoning the remaining connections.
pub async fn bounded<F>(
    mut shutdown: watch::Receiver<bool>,
    limit: Duration,
    server: F,
) -> std::io::Result<()>
where
    F: Future<Output = std::io::Result<()>>,
{
    let deadline = async {
        while !*shutdown.borrow() {
            if shutdown.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
        tokio::time::sleep(limit).await;
    };
    tokio::select! {
        result = server => result,
        _ = deadline => {
            tracing::warn!(?limit, "shutdown timeout reached; dropping remaining connections");
            Ok(())
        }
    }
}

/// Wait up to `limit` for the indexer task to observe shutdown and return.
pub async fn join_indexer(task: JoinHandle<()>, limit: Duration) {
    match tokio::time::timeout(limit, task).await {
        Ok(_) => tracing::info!("indexer stopped"),
        Err(_) => tracing::warn!(?limit, "indexer did not stop before the shutdown timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn abandons_a_stuck_server_after_the_timeout() {
        let (tx, rx) = watch::channel(false);
        tx.send(true).unwrap();
        let result = bounded(rx, Duration::from_millis(20), std::future::pending()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_when_the_server_finishes_first() {
        let (_tx, rx) = watch::channel(false);
        let result = bounded(rx, Duration::from_secs(60), async { Ok(()) }).await;
        assert!(result.is_ok());
    }
}
