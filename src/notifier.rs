//! Event notification system (future implementation)
//!
//! TODO: Implement webhook notifications for event thresholds
//! TODO: Add Slack integration
//! TODO: Support configurable notification rules

use anyhow::Result;

/// Trait for notifying about events
#[allow(dead_code)]
pub trait Notifier: Send + Sync {
    /// Send a notification message
    async fn notify(&self, message: &str) -> Result<()>;
}

/// Stub notifier that does nothing
#[allow(dead_code)]
pub struct StubNotifier;

impl Notifier for StubNotifier {
    async fn notify(&self, _message: &str) -> Result<()> {
        // No-op for now
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_notifier() {
        let notifier = StubNotifier;
        // Should not panic or error
        notifier.notify("test message").await.unwrap();
    }
}

