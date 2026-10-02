//! Stellar RWA API — a read-only REST index of tokenized real-world asset
//! activity on Stellar.
//!
//! The server starts the background indexer (which polls Soroban RPC every 10s)
//! and serves the current in-memory snapshot over HTTP. It holds no secrets,
//! signs nothing, and never mutates on-chain state.

use stellar_rwa_api::config_env;
use stellar_rwa_api::indexer;
use stellar_rwa_api::indexer::{AppState, Config, Indexer};
use stellar_rwa_api::routes;
use stellar_rwa_api::request_id;
use stellar_rwa_api::shutdown;

use std::net::SocketAddr;

use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    init_tracing();

    // Validate every env var up front, naming each offending variable.
    config_env::validate();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "config validation failed; exiting");
            std::process::exit(1);
        }
    };
    tracing::info!(
        rpc = %config.rpc_url,
        registry = %config.registry_id,
        "starting stellar-rwa-api"
    );

    // Issue #432 — probe configured contract ids at startup and warn loudly
    // if any do not resolve. Startup continues regardless so a transient RPC
    // hiccup or an incorrect env var does not prevent the process from
    // starting; the warnings are actionable without being fatal.
    for warning in indexer::probe_contract_ids(&config).await {
        tracing::warn!(warning, "contract id probe failed at startup");
    }

    let metrics_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    let state = AppState::new(config, metrics_handle);

    // Shared shutdown flag: flipped once by `shutdown_signal` and observed
    // by the indexer's poll loop so it stops issuing new refresh cycles
    // once the process is terminating, rather than racing shutdown.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn the indexer; it owns its own clone of the shared state.
    let indexer = Indexer::new(state.clone());
    let drain_rx = shutdown_rx.clone();
    let indexer_task = tokio::spawn(async move { indexer.run(shutdown_rx).await });

    let app = routes::router(state.clone())
        .layer(axum::middleware::from_fn_with_state(state, request_id::layer))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, %addr, "failed to bind");
            std::process::exit(1);
        }
    };
    tracing::info!(%addr, "listening");

    let limit = shutdown::timeout();
    if let Err(e) = shutdown::bounded(
        drain_rx,
        limit,
        async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal(shutdown_tx))
            .await
        },
    )
    .await
    {
        tracing::error!(error = %e, "server error");
        std::process::exit(1);
    }
    shutdown::join_indexer(indexer_task, limit).await;
    tracing::info!("shut down cleanly");
}

/// Resolve when the process receives Ctrl-C (SIGINT) or SIGTERM, for
/// graceful shutdown. Axum stops accepting new connections and lets
/// in-flight requests finish once this future resolves; we also flip
/// `shutdown_tx` so the indexer's poll loop halts rather than starting
/// another refresh cycle mid-shutdown.
async fn shutdown_signal(shutdown_tx: watch::Sender<bool>) {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to install Ctrl-C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => tracing::error!(error = %e, "failed to install SIGTERM handler"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received; finishing in-flight requests");
    let _ = shutdown_tx.send(true);
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("stellar_rwa_api=info,tower_http=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();
}
