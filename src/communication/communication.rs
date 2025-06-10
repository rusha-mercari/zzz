use zellij_tile::prelude::*;

use crate::coordination_message::CoordinationMessage;
use super::envelope::MessageEnvelope;
use super::error::CommunicationError;

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