//! Event type definitions for Solana blockchain events

use serde::{Deserialize, Serialize};

/// Log event from a Solana program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// RFC3339 timestamp
    pub timestamp: String,
    /// Transaction signature
    pub signature: String,
    /// Slot number
    pub slot: u64,
    /// Program ID
    pub program_id: String,
    /// Array of log messages
    pub logs: Vec<String>,
}

impl LogEvent {
    /// Create a new log event
    pub fn new(
        timestamp: String,
        signature: String,
        slot: u64,
        program_id: String,
        logs: Vec<String>,
    ) -> Self {
        Self {
            timestamp,
            signature,
            slot,
            program_id,
            logs,
        }
    }
}

/// Account event (state change)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEvent {
    /// RFC3339 timestamp
    pub timestamp: String,
    /// Public key of the account
    pub pubkey: String,
    /// Slot number
    pub slot: u64,
    /// Account lamports balance
    pub lamports: u64,
    /// Account data (base64 encoded)
    pub data: String,
}

impl AccountEvent {
    /// Create a new account event
    pub fn new(
        timestamp: String,
        pubkey: String,
        slot: u64,
        lamports: u64,
        data: String,
    ) -> Self {
        Self {
            timestamp,
            pubkey,
            slot,
            lamports,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event_serialization() {
        let event = LogEvent::new(
            "2024-01-15T10:30:45Z".to_string(),
            "signature123".to_string(),
            12345,
            "program123".to_string(),
            vec!["Log message 1".to_string(), "Log message 2".to_string()],
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: LogEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.signature, "signature123");
        assert_eq!(deserialized.slot, 12345);
        assert_eq!(deserialized.logs.len(), 2);
    }

    #[test]
    fn test_account_event_serialization() {
        let event = AccountEvent::new(
            "2024-01-15T10:30:45Z".to_string(),
            "pubkey123".to_string(),
            12345,
            1000000,
            "base64data".to_string(),
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AccountEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.pubkey, "pubkey123");
        assert_eq!(deserialized.lamports, 1000000);
        assert_eq!(deserialized.data, "base64data");
    }
}
