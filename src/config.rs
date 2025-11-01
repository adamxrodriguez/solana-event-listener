use anyhow::{Context, Result};
use clap::Parser;
use dotenvy;
use std::net::SocketAddr;

/// Solana event listener configuration
#[derive(Debug, Clone, Parser)]
#[command(name = "solana-event-listener")]
#[command(about = "Listen to Solana blockchain events via WebSocket")]
pub struct Config {
    /// Solana WebSocket endpoint
    #[arg(long, env = "WS_URL")]
    pub ws_url: String,

    /// Operation mode: logs or account
    #[arg(long, env = "MODE")]
    pub mode: Mode,

    /// Program ID for logs mode
    #[arg(long, env = "PROGRAM_ID")]
    pub program_id: Option<String>,

    /// Comma-separated account addresses for account mode
    #[arg(long, env = "ACCOUNTS")]
    pub accounts: Option<String>,

    /// Commitment level
    #[arg(long, env = "COMMITMENT", default_value = "finalized")]
    pub commitment: Commitment,

    /// Path to JSONL event log file
    #[arg(long, env = "EVENT_LOG_PATH", default_value = "./events.jsonl")]
    pub event_log_path: String,

    /// Metrics server bind address
    #[arg(long, env = "METRICS_ADDR", default_value = "0.0.0.0:9108")]
    pub metrics_addr: String,
}

impl Config {
    /// Load configuration from environment and CLI arguments
    pub fn load() -> Result<Self> {
        // Load .env file if it exists (ignore errors)
        let _ = dotenvy::dotenv();

        let config = Config::try_parse().context("Failed to parse configuration")?;

        // Validate mode-specific requirements
        match config.mode {
            Mode::Logs if config.program_id.is_none() => {
                anyhow::bail!("MODE=logs requires PROGRAM_ID to be set");
            }
            Mode::Account if config.accounts.is_none() => {
                anyhow::bail!("MODE=account requires ACCOUNTS to be set");
            }
            _ => {}
        }

        Ok(config)
    }

    /// Parse metrics address as SocketAddr
    pub fn metrics_socket_addr(&self) -> Result<SocketAddr> {
        self.metrics_addr
            .parse()
            .with_context(|| format!("Invalid METRICS_ADDR: {}", self.metrics_addr))
    }

    /// Parse comma-separated accounts into a vector
    pub fn parse_accounts(&self) -> Result<Vec<String>> {
        match &self.accounts {
            Some(accounts_str) => Ok(accounts_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()),
            None => Ok(vec![]),
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Mode {
    Logs,
    Account,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Logs => "logs",
            Mode::Account => "account",
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

impl Commitment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Commitment::Processed => "processed",
            Commitment::Confirmed => "confirmed",
            Commitment::Finalized => "finalized",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_as_str() {
        assert_eq!(Commitment::Processed.as_str(), "processed");
        assert_eq!(Commitment::Confirmed.as_str(), "confirmed");
        assert_eq!(Commitment::Finalized.as_str(), "finalized");
    }

    #[test]
    fn test_parse_accounts() {
        let config = Config {
            ws_url: "wss://test".to_string(),
            mode: Mode::Logs,
            program_id: None,
            accounts: Some("addr1,addr2,addr3".to_string()),
            commitment: Commitment::Finalized,
            event_log_path: "./test.jsonl".to_string(),
            metrics_addr: "0.0.0.0:9108".to_string(),
        };

        let parsed = config.parse_accounts().unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "addr1");
        assert_eq!(parsed[1], "addr2");
        assert_eq!(parsed[2], "addr3");
    }

    #[test]
    fn test_parse_accounts_empty() {
        let config = Config {
            ws_url: "wss://test".to_string(),
            mode: Mode::Logs,
            program_id: None,
            accounts: None,
            commitment: Commitment::Finalized,
            event_log_path: "./test.jsonl".to_string(),
            metrics_addr: "0.0.0.0:9108".to_string(),
        };

        let parsed = config.parse_accounts().unwrap();
        assert!(parsed.is_empty());
    }
}

