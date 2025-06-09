use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zellij_tile::prelude::*;

use crate::coordination_message::CoordinationMessage;

/// Errors that can occur during inter-pane communication
#[derive(Debug)]
pub enum CommunicationError {
    /// Failed to serialize message to JSON
    SerializationError(serde_json::Error),
    /// Failed to deliver message to target
    MessageDeliveryFailed(String),
    /// Invalid target pane specified
    InvalidTarget(String),
}

impl std::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationError::SerializationError(e) => {
                write!(f, "Message serialization failed: {}", e)
            }
            CommunicationError::MessageDeliveryFailed(msg) => {
                write!(f, "Message delivery failed: {}", msg)
            }
            CommunicationError::InvalidTarget(target) => {
                write!(f, "Invalid target pane: {}", target)
            }
        }
    }
}

impl From<serde_json::Error> for CommunicationError {
    fn from(error: serde_json::Error) -> Self {
        CommunicationError::SerializationError(error)
    }
}

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
    pub fn new_targeted(
        message: CoordinationMessage,
        target_pane: &str,
        sender: &str,
    ) -> Self {
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

/// Communication utilities for the ZZZ plugin
pub struct Communication;

impl Communication {
    /// Send a coordination message using Zellij's pipe system
    /// 
    /// This is a low-level function that handles the actual pipe message sending.
    /// Use the State wrapper methods for most use cases.
    pub fn send_pipe_message(envelope: &MessageEnvelope) -> Result<(), CommunicationError> {
        // Serialize the envelope to JSON
        let payload = serde_json::to_string(envelope)?;
        
        // Create the pipe message
        let message = MessageToPlugin::new("coordination").with_payload(payload);
        
        // Send via Zellij's pipe system
        // Note: This sends to all plugins listening on the "coordination" pipe
        pipe_message_to_plugin(message);
        
        Ok(())
    }

    /// Parse an incoming payload as either MessageEnvelope or legacy CoordinationMessage
    pub fn parse_incoming_message(payload: &str) -> Result<ParsedMessage, serde_json::Error> {
        // Try parsing as MessageEnvelope first
        if let Ok(envelope) = serde_json::from_str::<MessageEnvelope>(payload) {
            return Ok(ParsedMessage::Envelope(envelope));
        }
        
        // Fall back to legacy CoordinationMessage format
        if let Ok(message) = serde_json::from_str::<CoordinationMessage>(payload) {
            return Ok(ParsedMessage::Legacy(message));
        }
        
        // If neither works, return the JSON error from the envelope parsing
        Err(serde_json::from_str::<MessageEnvelope>(payload).unwrap_err())
    }
}

/// Result of parsing an incoming message
#[derive(Debug)]
pub enum ParsedMessage {
    /// Modern envelope format
    Envelope(MessageEnvelope),
    /// Legacy direct CoordinationMessage format
    Legacy(CoordinationMessage),
}