use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zellij_tile::prelude::*;

use crate::coordination_message::CoordinationMessage;
use crate::pane_role::PaneRole;

/// Errors that can occur during inter-pane communication
#[derive(Debug)]
pub enum CommunicationError {
    /// Failed to serialize message to JSON
    SerializationError(serde_json::Error),
    /// Failed to deliver message to target
    MessageDeliveryFailed(String),
    /// Invalid target pane specified
    InvalidTarget(String),
    /// Target pane not found in routing table
    PaneNotFound(PaneRole),
    /// Failed to discover panes
    PaneDiscoveryFailed(String),
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
            CommunicationError::PaneNotFound(role) => {
                write!(f, "Pane not found for role: {:?}", role)
            }
            CommunicationError::PaneDiscoveryFailed(msg) => {
                write!(f, "Pane discovery failed: {}", msg)
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

/// Message router for dispatching coordination messages by pane role
pub struct MessageRouter {
    /// Mapping from pane roles to their pane IDs
    pane_registry: HashMap<PaneRole, PaneId>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            pane_registry: HashMap::new(),
        }
    }

    /// Discover panes and map them to roles based on their names
    pub fn discover_panes(&mut self) -> Result<(), CommunicationError> {
        // Get the current layout info which includes pane information
        let _layout_info = get_plugin_ids();
        
        // For now, we'll build the registry based on expected pane names
        // In a real implementation, we would iterate through actual panes
        // This is a simplified approach that logs what we're looking for
        
        // Clear existing registry
        self.pane_registry.clear();
        
        // Note: Since we don't have direct access to pane information in the current API,
        // we'll implement a discovery mechanism that can be populated externally
        // or through configuration
        
        Ok(())
    }

    /// Manually register a pane with a specific role
    pub fn register_pane(&mut self, role: PaneRole, pane_id: PaneId) {
        self.pane_registry.insert(role, pane_id);
    }

    /// Get the pane ID for a specific role
    pub fn get_pane_id(&self, role: &PaneRole) -> Option<PaneId> {
        self.pane_registry.get(role).copied()
    }

    /// Route a message to a specific pane role
    pub fn route_message_to_role(
        &self,
        message: &CoordinationMessage,
        target_role: PaneRole,
    ) -> Result<(), CommunicationError> {
        // Look up the pane ID for the target role
        let pane_id = self.get_pane_id(&target_role)
            .ok_or(CommunicationError::PaneNotFound(target_role))?;
        
        // Serialize the message
        let message_json = serde_json::to_string(message)
            .map_err(CommunicationError::SerializationError)?;
        
        // Write the message to the target pane
        write_chars_to_pane_id(&message_json, pane_id);
        
        Ok(())
    }

    /// Route a message to multiple pane roles
    pub fn route_message_to_roles(
        &self,
        message: &CoordinationMessage,
        target_roles: &[PaneRole],
    ) -> Vec<(PaneRole, Result<(), CommunicationError>)> {
        target_roles
            .iter()
            .map(|role| {
                let result = self.route_message_to_role(message, *role);
                (*role, result)
            })
            .collect()
    }

    /// Broadcast a message to all registered panes
    pub fn broadcast_to_all(&self, message: &CoordinationMessage) -> Vec<(PaneRole, Result<(), CommunicationError>)> {
        let all_roles: Vec<PaneRole> = self.pane_registry.keys().copied().collect();
        self.route_message_to_roles(message, &all_roles)
    }

    /// Get a list of all registered pane roles
    pub fn get_registered_roles(&self) -> Vec<PaneRole> {
        self.pane_registry.keys().copied().collect()
    }

    /// Check if a specific role is registered
    pub fn is_role_registered(&self, role: &PaneRole) -> bool {
        self.pane_registry.contains_key(role)
    }

    /// Match pane name to role using pattern matching
    pub fn match_pane_name_to_role(pane_name: &str) -> Option<PaneRole> {
        match pane_name.to_lowercase().as_str() {
            name if name.contains("overseer") => Some(PaneRole::Overseer),
            name if name.contains("commander") => Some(PaneRole::Commander),
            name if name.contains("tasklist") || name.contains("task-list") || name.contains("task_list") => Some(PaneRole::TaskList),
            name if name.contains("review") => Some(PaneRole::Review),
            name if name.contains("editor") => Some(PaneRole::Editor),
            _ => None,
        }
    }
}