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