//! Solana WebSocket RPC client

use anyhow::{Context, Result};
use crate::config::Config;
use crate::event::{AccountEvent, LogEvent};
use crate::metrics::MetricsRegistry;
use crate::storage::JsonlWriter;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, trace, warn};

/// JSON-RPC request wrapper
#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// JSON-RPC response wrapper
#[derive(Debug, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// RPC error structure
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Logs notification payload
#[derive(Debug, Deserialize)]
struct LogsNotification {
    result: LogsNotificationResult,
}

/// Logs notification result
#[derive(Debug, Deserialize)]
struct LogsNotificationResult {
    context: NotificationContext,
    value: LogsNotificationValue,
}

/// Notification context
#[derive(Debug, Deserialize)]
struct NotificationContext {
    slot: u64,
}

/// Logs notification value
#[derive(Debug, Deserialize)]
struct LogsNotificationValue {
    err: Option<serde_json::Value>,
    logs: Vec<String>,
    signature: String,
}

/// Calculate exponential backoff delay with max cap
fn calculate_backoff(attempt: u32, max_seconds: u64) -> Duration {
    let delay_secs = (1u64 << attempt.min(5)).min(max_seconds);
    Duration::from_secs(delay_secs)
}

/// Run logs subscription with automatic reconnection
pub async fn run_logs_subscribe(
    config: &Config,
    writer: JsonlWriter,
    metrics: MetricsRegistry,
) -> Result<()> {
    let ws_url = &config.ws_url;
    let program_id = config.program_id.as_ref().unwrap();
    let commitment = config.commitment.as_str();

    let mut attempt = 0u32;
    loop {
        match try_logs_subscribe(ws_url, program_id, commitment, &writer, &metrics).await {
            Ok(()) => {
                info!("Logs subscription loop exited normally");
                break Ok(());
            }
            Err(e) => {
                error!("Logs subscription error: {}", e);
                metrics.errors_total.inc();
                metrics.ws_connected.set(0.0);
                
                // Calculate backoff
                let backoff = calculate_backoff(attempt, 30);
                attempt += 1;
                
                warn!("Reconnecting in {:?} (attempt {})", backoff, attempt);
                tokio::time::sleep(backoff).await;
                
                // Reset backoff counter on every 10 attempts to prevent overflow
                if attempt >= 10 {
                    attempt = 0;
                }
            }
        }
    }
}

/// Try to run logs subscription (single attempt)
async fn try_logs_subscribe(
    ws_url: &str,
    program_id: &str,
    commitment: &str,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    info!("Connecting to Solana WebSocket: {}", ws_url);
    
    // Set connected gauge to 0 initially
    metrics.ws_connected.set(0.0);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .context("Failed to connect to WebSocket")?;

    info!("Connected to WebSocket");
    metrics.ws_connected.set(1.0);

    // Split the stream for read/write
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to logs
    let subscription_id = 1u64;
    let subscribe_request = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: subscription_id,
        method: "logsSubscribe".to_string(),
        params: json!({
            "mentions": [program_id],
            "commitment": commitment
        }),
    };

    let subscribe_msg = serde_json::to_string(&subscribe_request)?;
    info!("Sending subscription request for program: {}", program_id);

    // Send subscription request
    write
        .send(Message::Text(subscribe_msg))
        .await
        .context("Failed to send subscription request")?;

    info!("Subscribed to logs for program: {}", program_id);

    // Process incoming messages
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                trace!("Received message: {}", text);
                
                if let Err(e) = handle_message(&text, &writer, &metrics).await {
                    error!("Error handling message: {}", e);
                    metrics.errors_total.inc();
                }
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    anyhow::bail!("Failed to send pong");
                }
            }
            Ok(Message::Pong(_)) => {
                trace!("Received pong");
            }
            Ok(Message::Close(_)) => {
                warn!("WebSocket closed by server");
                metrics.ws_connected.set(0.0);
                anyhow::bail!("WebSocket closed by server");
            }
            Ok(Message::Binary(_)) => {
                warn!("Received unexpected binary message");
            }
            Ok(Message::Frame(_)) => {
                // Low-level frame, skip
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                metrics.errors_total.inc();
                metrics.ws_connected.set(0.0);
                anyhow::bail!("WebSocket error: {}", e);
            }
        }
    }

    // Stream ended
    anyhow::bail!("WebSocket stream ended");
}

/// Handle incoming WebSocket message
async fn handle_message(
    text: &str,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    // Try to parse as RPC response first
    if let Ok(response) = serde_json::from_str::<RpcResponse>(text) {
        // This is likely a subscription confirmation
        if let Some(ref error) = response.error {
            anyhow::bail!("RPC error: {} (code: {})", error.message, error.code);
        }
        if let Some(ref result) = response.result {
            info!("Subscription confirmed: {}", result);
        }
        return Ok(());
    }

    // Try to parse as notification
    if let Ok(notification) = serde_json::from_str::<LogsNotification>(text) {
        handle_logs_notification(notification, writer, metrics).await?;
        return Ok(());
    }

    // Unknown message format
    warn!("Unknown message format: {}", text);
    Ok(())
}

/// Handle logs notification
async fn handle_logs_notification(
    notification: LogsNotification,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    let slot = notification.result.context.slot;
    let signature = &notification.result.value.signature;
    let logs = &notification.result.value.logs;

    // Get program ID from logs (first log line usually)
    let program_id = logs
        .first()
        .and_then(|log| {
            // Parse "Program log: {program_id}" format
            if log.starts_with("Program ") {
                log.split_whitespace().nth(2).map(String::from)
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Create timestamp
    let timestamp = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .context("Failed to format timestamp")?;

    // Create log event
    let event = LogEvent::new(
        timestamp,
        signature.clone(),
        slot,
        program_id.clone(),
        logs.clone(),
    );

    // Write to storage
    writer.write(&event).await.context("Failed to write event")?;

    // Increment metrics
    metrics.events_total.inc();

    // Log event
    info!(
        "Event: signature={}, slot={}, program={}, log_lines={}",
        signature,
        slot,
        program_id,
        logs.len()
    );

    Ok(())
}

/// Account notification payload
#[derive(Debug, Deserialize)]
struct AccountNotification {
    result: AccountNotificationResult,
}

/// Account notification result
#[derive(Debug, Deserialize)]
struct AccountNotificationResult {
    context: NotificationContext,
    value: AccountNotificationValue,
}

/// Account notification value
#[derive(Debug, Deserialize)]
struct AccountNotificationValue {
    account: AccountData,
}

/// Account data
#[derive(Debug, Deserialize)]
struct AccountData {
    #[serde(rename = "lamports")]
    lamports: u64,
    data: Vec<String>,  // base64 encoded
}

/// Run account subscription with automatic reconnection
pub async fn run_account_subscribe(
    config: &Config,
    writer: JsonlWriter,
    metrics: MetricsRegistry,
) -> Result<()> {
    let ws_url = &config.ws_url;
    let accounts = config.parse_accounts()?;
    let commitment = config.commitment.as_str();

    if accounts.is_empty() {
        anyhow::bail!("No accounts provided for account subscription");
    }

    let mut attempt = 0u32;
    loop {
        match try_account_subscribe(ws_url, &accounts, commitment, &writer, &metrics).await {
            Ok(()) => {
                info!("Account subscription loop exited normally");
                break Ok(());
            }
            Err(e) => {
                error!("Account subscription error: {}", e);
                metrics.errors_total.inc();
                metrics.ws_connected.set(0.0);
                
                // Calculate backoff
                let backoff = calculate_backoff(attempt, 30);
                attempt += 1;
                
                warn!("Reconnecting in {:?} (attempt {})", backoff, attempt);
                tokio::time::sleep(backoff).await;
                
                // Reset backoff counter on every 10 attempts
                if attempt >= 10 {
                    attempt = 0;
                }
            }
        }
    }
}

/// Try to run account subscription (single attempt)
async fn try_account_subscribe(
    ws_url: &str,
    accounts: &[String],
    commitment: &str,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    info!("Connecting to Solana WebSocket: {}", ws_url);
    
    metrics.ws_connected.set(0.0);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .context("Failed to connect to WebSocket")?;

    info!("Connected to WebSocket");
    metrics.ws_connected.set(1.0);

    // Split the stream for read/write
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to all accounts
    let mut subscription_id = 1u64;
    for account in accounts {
        let subscribe_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: subscription_id,
            method: "accountSubscribe".to_string(),
            params: json!({
                "account": account,
                "commitment": commitment,
                "encoding": "base64"
            }),
        };

        let subscribe_msg = serde_json::to_string(&subscribe_request)?;
        info!("Subscribing to account: {}", account);

        write
            .send(Message::Text(subscribe_msg))
            .await
            .context("Failed to send subscription request")?;

        subscription_id += 1;
    }

    info!("Subscribed to {} accounts", accounts.len());

    // Process incoming messages
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                trace!("Received message: {}", text);
                
                if let Err(e) = handle_account_message(&text, &writer, &metrics).await {
                    error!("Error handling message: {}", e);
                    metrics.errors_total.inc();
                }
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    anyhow::bail!("Failed to send pong");
                }
            }
            Ok(Message::Pong(_)) => {
                trace!("Received pong");
            }
            Ok(Message::Close(_)) => {
                warn!("WebSocket closed by server");
                metrics.ws_connected.set(0.0);
                anyhow::bail!("WebSocket closed by server");
            }
            Ok(Message::Binary(_)) => {
                warn!("Received unexpected binary message");
            }
            Ok(Message::Frame(_)) => {
                // Low-level frame, skip
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                metrics.errors_total.inc();
                metrics.ws_connected.set(0.0);
                anyhow::bail!("WebSocket error: {}", e);
            }
        }
    }

    // Stream ended
    anyhow::bail!("WebSocket stream ended");
}

/// Handle incoming WebSocket message for account subscriptions
async fn handle_account_message(
    text: &str,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    // Try to parse as RPC response first
    if let Ok(response) = serde_json::from_str::<RpcResponse>(text) {
        if let Some(ref error) = response.error {
            anyhow::bail!("RPC error: {} (code: {})", error.message, error.code);
        }
        if let Some(ref result) = response.result {
            info!("Subscription confirmed: {}", result);
        }
        return Ok(());
    }

    // Try to parse as account notification
    if let Ok(notification) = serde_json::from_str::<AccountNotification>(text) {
        handle_account_notification(notification, writer, metrics).await?;
        return Ok(());
    }

    // Unknown message format
    warn!("Unknown message format: {}", text);
    Ok(())
}

/// Handle account notification
async fn handle_account_notification(
    notification: AccountNotification,
    writer: &JsonlWriter,
    metrics: &MetricsRegistry,
) -> Result<()> {
    let slot = notification.result.context.slot;
    let lamports = notification.result.value.account.lamports;
    let data = notification.result.value.account.data.join("");

    // Note: We don't have the pubkey directly in the notification
    // This is a limitation of the current implementation
    // In a real implementation, we'd need to track subscription IDs to pubkeys
    let pubkey = "unknown".to_string();

    // Create timestamp
    let timestamp = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .context("Failed to format timestamp")?;

    // Create account event
    let event = AccountEvent::new(
        timestamp,
        pubkey,
        slot,
        lamports,
        data,
    );

    // Write to storage
    writer.write(&event).await.context("Failed to write event")?;

    // Increment metrics
    metrics.events_total.inc();

    // Log event
    info!(
        "Account event: pubkey={}, slot={}, lamports={}",
        pubkey, slot, lamports
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff() {
        // Test exponential backoff
        assert_eq!(calculate_backoff(0, 30), Duration::from_secs(1));
        assert_eq!(calculate_backoff(1, 30), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2, 30), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3, 30), Duration::from_secs(8));
        assert_eq!(calculate_backoff(4, 30), Duration::from_secs(16));
        assert_eq!(calculate_backoff(5, 30), Duration::from_secs(30));

        // Test max cap
        assert_eq!(calculate_backoff(10, 30), Duration::from_secs(30));
        assert_eq!(calculate_backoff(100, 30), Duration::from_secs(30));

        // Test different max_seconds
        assert_eq!(calculate_backoff(3, 10), Duration::from_secs(8));
        assert_eq!(calculate_backoff(4, 10), Duration::from_secs(10));
    }
}