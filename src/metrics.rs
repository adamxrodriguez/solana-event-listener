//! Prometheus metrics for monitoring

use anyhow::{Context, Result};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Counter, Gauge, Opts, Registry, TextEncoder};
use std::net::SocketAddr;
use tracing::{error, info};

/// Prometheus metrics registry wrapper
#[derive(Clone)]
pub struct MetricsRegistry {
    /// Total number of events processed
    pub events_total: Counter,
    /// Total number of errors encountered
    pub errors_total: Counter,
    /// WebSocket connection status (1=connected, 0=disconnected)
    pub ws_connected: Gauge,
    /// Inner Prometheus registry
    registry: Registry,
}

impl MetricsRegistry {
    /// Create and register all metrics
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        // Register events_total counter
        let events_total_opts = Opts::new("sol_events_total", "Total number of events processed")
            .namespace("sol");
        let events_total = Counter::with_opts(events_total_opts)?;
        registry.register(Box::new(events_total.clone()))?;

        // Register errors_total counter
        let errors_total_opts = Opts::new("sol_errors_total", "Total number of errors encountered")
            .namespace("sol");
        let errors_total = Counter::with_opts(errors_total_opts)?;
        registry.register(Box::new(errors_total.clone()))?;

        // Register ws_connected gauge
        let ws_connected_opts = Opts::new(
            "sol_ws_connected",
            "WebSocket connection status (1=connected, 0=disconnected)",
        )
        .namespace("sol");
        let ws_connected = Gauge::with_opts(ws_connected_opts)?;
        registry.register(Box::new(ws_connected.clone()))?;

        Ok(Self {
            events_total,
            errors_total,
            ws_connected,
            registry,
        })
    }

    /// Start metrics HTTP server
    #[allow(dead_code)]
    pub async fn start_server(&self, addr: SocketAddr) -> Result<()> {
        let app = Router::new().route("/metrics", get(metrics_handler));

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind metrics server to {}", addr))?;

        info!("Metrics server listening on http://{}", addr);

        let app_state = AppState {
            registry: self.registry.clone(),
        };

        let server = axum::serve(
            listener,
            app.with_state(app_state)
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;

        if let Err(e) = server {
            error!("Metrics server error: {}", e);
        }

        Ok(())
    }

    /// Spawn metrics server in background task
    pub fn spawn_server(&self, addr: SocketAddr) -> tokio::task::JoinHandle<()> {
        let registry = self.registry.clone();
        tokio::spawn(async move {
            let app = Router::new().route("/metrics", get(metrics_handler));

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind metrics server to {}: {}", addr, e);
                    return;
                }
            };

            info!("Metrics server listening on http://{}", addr);

            let app_state = AppState { registry };

            let server = axum::serve(
                listener,
                app.with_state(app_state)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;

            if let Err(e) = server {
                error!("Metrics server error: {}", e);
            }
        })
    }
}

/// Application state for metrics handler
#[derive(Clone)]
struct AppState {
    registry: Registry,
}

/// Handler for /metrics endpoint
async fn metrics_handler(state: axum::extract::State<AppState>) -> Response {
    let encoder = TextEncoder::new();
    let metric_families = state.registry.gather();

    match encoder.encode_to_string(&metric_families) {
        Ok(body) => {
            let mut response = Response::new(body.into());
            *response.headers_mut() = axum::http::HeaderMap::new();
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4".parse().unwrap(),
            );
            response
        }
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response()
        }
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new();
        assert!(registry.is_ok());
    }

    #[test]
    fn test_metrics_increment() {
        let registry = MetricsRegistry::default();
        
        registry.events_total.inc();
        assert_eq!(registry.events_total.get(), 1.0);

        registry.errors_total.inc_by(5.0);
        assert_eq!(registry.errors_total.get(), 5.0);

        registry.ws_connected.set(1.0);
        assert_eq!(registry.ws_connected.get(), 1.0);
    }

    #[tokio::test]
    async fn test_metrics_server_startup() {
        let registry = MetricsRegistry::default();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        
        // Spawn the server in a background task
        let handle = registry.spawn_server(addr);
        
        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Abort the server task to clean up
        handle.abort();
        
        // Wait for the task to finish (it will be cancelled)
        let _ = handle.await;
        
        // Test passes if we got here without hanging
    }
}
