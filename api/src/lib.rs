//! Library surface for the `stellar-rwa-api` crate.
//!
//! Exposes the public indexer types and domain models so that benchmarks and
//! integration tests can import them without going through the binary entry
//! point.

pub mod config_env;
pub mod indexer;
pub mod indexer_metrics;
pub mod models;
pub mod poll_status;
pub mod request_id;
pub mod snapshot_bounds;
pub mod routes;
pub mod shutdown;
pub mod stale_guard;
