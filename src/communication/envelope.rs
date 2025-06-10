use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::coordination_message::CoordinationMessage;

/// Message envelope for inter-pane communication
/// Wraps CoordinationMessage with metadata for routing and debugging
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageEnvelope {
    /// Target pane title (None = broadcast to all panes)
    pub target_pane: Option<String>,
    /// The actual coordination message
    pub coordination_message: CoordinationMessage,
    /// Sender identifier for debugging and routing
    pub sender: String,
    /// Unix timestamp when message was created
    pub timestamp: u64,
}

impl MessageEnvelope {
    /// Create a new message envelope for a specific target pane
    pub fn new_targeted(message: CoordinationMessage, target_pane: &str, sender: &str) -> Self {
        Self {
            target_pane: Some(target_pane.to_string()),
            coordination_message: message,
            sender: sender.to_string(),
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a new message envelope for broadcasting to all panes
    pub fn new_broadcast(message: CoordinationMessage, sender: &str) -> Self {
        Self {
            target_pane: None,
            coordination_message: message,
            sender: sender.to_string(),
            timestamp: Self::current_timestamp(),
        }
    }

    /// Check if this message is targeted to a specific pane
    pub fn is_targeted_to(&self, pane_title: &str) -> bool {
        match &self.target_pane {
            Some(target) => target == pane_title,
            None => true, // Broadcast messages are for everyone
        }
    }

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.target_pane.is_none()
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
