mod config;
mod event;
mod metrics;
mod notifier;
mod rpc;
mod storage;

use anyhow::Result;
use config::Config;
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

    tracing::info!("Starting Solana Event Listener v0.1.0");

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded: mode={:?}", config.mode.as_str());

    // For now, just print a placeholder message
    tracing::info!("Event listener ready (implementation in progress)");

    Ok(())
}

