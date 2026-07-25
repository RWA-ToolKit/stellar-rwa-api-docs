//! Stellar RWA API — a read-only REST index of tokenized real-world asset
//! activity on Stellar.
//!
//! The server starts the background indexer (which polls Soroban RPC every 10s)
//! and serves the current in-memory snapshot over HTTP. It holds no secrets,
//! signs nothing, and never mutates on-chain state.

mod indexer;
mod models;
mod routes;

use std::net::SocketAddr;

use metrics_exporter_prometheus::PrometheusBuilder;

use indexer::{AppState, Config, ConfigError, Indexer};

#[tokio::main]
async fn main() {
    init_tracing();

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

    let metrics_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    let state = AppState::new(config, metrics_handle);

    // Spawn the indexer; it owns its own clone of the shared state.
    let indexer = Indexer::new(state.clone());
    tokio::spawn(async move { indexer.run().await });

    let app = routes::router(state).layer(tower_http::trace::TraceLayer::new_for_http());

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

    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        tracing::error!(error = %e, "server error");
        std::process::exit(1);
    }
    tracing::info!("shut down cleanly");
}

/// Resolve on Ctrl-C or SIGTERM (the signal container runtimes send on
/// `docker stop`/pod termination), for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to install Ctrl-C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("shutdown signal received (ctrl-c)"),
        _ = terminate => tracing::info!("shutdown signal received (SIGTERM)"),
    }
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
