mod config;
mod event;
mod metrics;
mod notifier;
mod rpc;
mod storage;

use anyhow::Result;
use config::Config;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber with environment filter
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Solana Event Listener v0.1.0");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: mode={:?}", config.mode.as_str());

    // Initialize metrics registry
    let metrics = metrics::MetricsRegistry::default();
    info!("Metrics registry initialized");

    // Spawn metrics server
    let metrics_addr = config.metrics_socket_addr()?;
    let _metrics_handle = metrics.spawn_server(metrics_addr);
    info!("Metrics server spawned on {}", metrics_addr);

    // For now, just keep the process running
    info!("Event listener ready (implementation in progress)");

    // Wait indefinitely (we'll add proper shutdown in PR 6)
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
