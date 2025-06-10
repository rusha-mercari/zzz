use zellij_tile::prelude::*;

use super::envelope::MessageEnvelope;
use super::error::CommunicationError;
use crate::coordination_message::CoordinationMessage;
use crate::zellij_service::ZellijService;

/// Communication utilities for the ZZZ plugin
pub struct Communication<T: ZellijService> {
    zellij_service: T,
}

impl<T: ZellijService> Communication<T> {
    /// Create a new Communication instance with the given ZellijService
    pub fn new(zellij_service: T) -> Self {
        Self { zellij_service }
    }

    /// Send a coordination message using Zellij's pipe system
    ///
    /// This is a low-level function that handles the actual pipe message sending.
    /// Use the State wrapper methods for most use cases.
    pub fn send_pipe_message(&self, envelope: &MessageEnvelope) -> Result<(), CommunicationError> {
        // Serialize the envelope to JSON
        let payload = serde_json::to_string(envelope)?;

        // Send via Zellij's pipe system using the injected service
        // Note: This sends to all plugins listening on the "coordination" pipe
        self.zellij_service.pipe_message_to_plugin(&payload, "coordination");

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

/// Type alias for test Communication with mock Zellij service
#[cfg(test)]
pub type MockCommunication = Communication<MockZellijService>;

#[cfg(test)]
use crate::zellij_service::MockZellijService;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination_message::CoordinationMessage;
    use crate::workflow_phase::WorkflowPhase;
    use crate::zellij_service::MockZellijService;

    fn create_test_communication() -> MockCommunication {
        Communication::new(MockZellijService::new())
    }

    fn create_test_envelope() -> MessageEnvelope {
        let message = CoordinationMessage::StartPlanning {
            task_id: 123,
            task_description: "Test task".to_string(),
        };
        MessageEnvelope::new_targeted(message, "test-pane", "test-sender")
    }

    fn create_test_coordination_message() -> CoordinationMessage {
        CoordinationMessage::StartImplementation
    }

    #[test]
    fn test_communication_new_creates_instance() {
        let mock_service = MockZellijService::new();
        let communication = Communication::new(mock_service);
        
        // Verify the struct was created (no direct way to inspect the service field)
        // We'll verify functionality in subsequent tests
        let _envelope = create_test_envelope();
        let _result = communication.send_pipe_message(&_envelope);
        // If it compiles and doesn't panic, the constructor worked
    }

    #[test]
    fn test_send_pipe_message_success_with_mock() {
        let communication = create_test_communication();
        let envelope = create_test_envelope();
        
        // Send the message
        let result = communication.send_pipe_message(&envelope);
        assert!(result.is_ok());
        
        // Verify the mock captured the message
        let piped_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(piped_messages.len(), 1);
        
        let (payload, target) = &piped_messages[0];
        assert_eq!(target, "coordination");
        
        // Verify the payload is valid JSON that can be deserialized back to MessageEnvelope
        let deserialized: MessageEnvelope = serde_json::from_str(payload).expect("Should deserialize");
        assert_eq!(deserialized.target_pane, envelope.target_pane);
        assert_eq!(deserialized.sender, envelope.sender);
    }

    #[test]
    fn test_send_pipe_message_with_different_envelope_types() {
        let communication = create_test_communication();
        
        // Test targeted envelope
        let targeted_envelope = MessageEnvelope::new_targeted(
            create_test_coordination_message(),
            "specific-pane",
            "sender1"
        );
        
        // Test broadcast envelope
        let broadcast_envelope = MessageEnvelope::new_broadcast(
            create_test_coordination_message(),
            "sender2"
        );
        
        // Send both messages
        assert!(communication.send_pipe_message(&targeted_envelope).is_ok());
        assert!(communication.send_pipe_message(&broadcast_envelope).is_ok());
        
        // Verify both were captured
        let piped_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(piped_messages.len(), 2);
        
        // Both should target "coordination"
        assert_eq!(piped_messages[0].1, "coordination");
        assert_eq!(piped_messages[1].1, "coordination");
        
        // Verify payloads are different (different envelope types)
        assert_ne!(piped_messages[0].0, piped_messages[1].0);
    }

    #[test]
    fn test_parse_incoming_message_envelope_format() {
        let original_envelope = create_test_envelope();
        let json_payload = serde_json::to_string(&original_envelope).unwrap();
        
        let result = Communication::<MockZellijService>::parse_incoming_message(&json_payload);
        assert!(result.is_ok());
        
        if let Ok(ParsedMessage::Envelope(parsed_envelope)) = result {
            assert_eq!(parsed_envelope.target_pane, original_envelope.target_pane);
            assert_eq!(parsed_envelope.sender, original_envelope.sender);
            assert_eq!(parsed_envelope.timestamp, original_envelope.timestamp);
        } else {
            panic!("Should parse as Envelope format");
        }
    }

    #[test]
    fn test_parse_incoming_message_legacy_format() {
        let original_message = create_test_coordination_message();
        let json_payload = serde_json::to_string(&original_message).unwrap();
        
        let result = Communication::<MockZellijService>::parse_incoming_message(&json_payload);
        assert!(result.is_ok());
        
        if let Ok(ParsedMessage::Legacy(parsed_message)) = result {
            assert!(matches!(parsed_message, CoordinationMessage::StartImplementation));
        } else {
            panic!("Should parse as Legacy format");
        }
    }

    #[test]
    fn test_parse_incoming_message_invalid_json() {
        let invalid_json = "{ invalid json structure";
        
        let result = Communication::<MockZellijService>::parse_incoming_message(invalid_json);
        assert!(result.is_err());
        
        // The error should be a serde_json::Error
        let error = result.unwrap_err();
        let error_message = format!("{}", error);
        // Just verify it's a JSON parsing error, don't be too specific about the message
        assert!(error_message.len() > 0);
    }

    #[test]
    fn test_parse_incoming_message_all_coordination_variants() {
        let coordination_messages = vec![
            CoordinationMessage::StartPlanning {
                task_id: 1,
                task_description: "Plan task".to_string(),
            },
            CoordinationMessage::PlanReady {
                todo_file_path: "/path/to/todo.md".to_string(),
            },
            CoordinationMessage::StartImplementation,
            CoordinationMessage::TaskCompleted {
                task_id: "task-123".to_string(),
            },
            CoordinationMessage::AllTasksComplete,
            CoordinationMessage::StartReview,
            CoordinationMessage::ReviewComplete {
                review_file_path: "/path/to/review.md".to_string(),
            },
            CoordinationMessage::PhaseTransition {
                from: WorkflowPhase::PlanningInProgress,
                to: WorkflowPhase::ImplementationInProgress,
            },
            CoordinationMessage::FileChanged {
                file_path: "/some/file.rs".to_string(),
                event_type: "modified".to_string(),
            },
        ];

        for message in coordination_messages {
            // Test as legacy format
            let legacy_json = serde_json::to_string(&message).unwrap();
            let legacy_result = Communication::<MockZellijService>::parse_incoming_message(&legacy_json);
            assert!(legacy_result.is_ok());
            
            if let Ok(ParsedMessage::Legacy(_)) = legacy_result {
                // Correct
            } else {
                panic!("Should parse as Legacy format for {:?}", message);
            }
            
            // Test as envelope format
            let envelope = MessageEnvelope::new_broadcast(message, "test-sender");
            let envelope_json = serde_json::to_string(&envelope).unwrap();
            let envelope_result = Communication::<MockZellijService>::parse_incoming_message(&envelope_json);
            assert!(envelope_result.is_ok());
            
            if let Ok(ParsedMessage::Envelope(_)) = envelope_result {
                // Correct
            } else {
                panic!("Should parse as Envelope format");
            }
        }
    }

    #[test]
    fn test_parsed_message_debug_output() {
        let envelope = create_test_envelope();
        let message = create_test_coordination_message();
        
        let envelope_parsed = ParsedMessage::Envelope(envelope);
        let legacy_parsed = ParsedMessage::Legacy(message);
        
        let envelope_debug = format!("{:?}", envelope_parsed);
        let legacy_debug = format!("{:?}", legacy_parsed);
        
        assert!(envelope_debug.contains("Envelope"));
        assert!(legacy_debug.contains("Legacy"));
        assert!(legacy_debug.contains("StartImplementation"));
    }

    #[test]
    fn test_roundtrip_send_and_parse_integration() {
        let communication = create_test_communication();
        let original_envelope = create_test_envelope();
        
        // Send the message
        let send_result = communication.send_pipe_message(&original_envelope);
        assert!(send_result.is_ok());
        
        // Get the payload that was sent
        let piped_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(piped_messages.len(), 1);
        let sent_payload = &piped_messages[0].0;
        
        // Parse the sent payload
        let parse_result = Communication::<MockZellijService>::parse_incoming_message(sent_payload);
        assert!(parse_result.is_ok());
        
        // Verify we get back the same envelope
        if let Ok(ParsedMessage::Envelope(parsed_envelope)) = parse_result {
            assert_eq!(parsed_envelope.target_pane, original_envelope.target_pane);
            assert_eq!(parsed_envelope.sender, original_envelope.sender);
            assert_eq!(parsed_envelope.timestamp, original_envelope.timestamp);
        } else {
            panic!("Roundtrip should preserve envelope format");
        }
    }

    #[test]
    fn test_edge_cases_with_special_characters() {
        let communication = create_test_communication();
        
        // Create envelope with special characters
        let special_message = CoordinationMessage::StartPlanning {
            task_id: 999,
            task_description: "Task with special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?`~".to_string(),
        };
        let special_envelope = MessageEnvelope::new_targeted(
            special_message,
            "pane-with-special_chars!@#",
            "sender/with\\slashes"
        );
        
        // Send message with special characters
        let result = communication.send_pipe_message(&special_envelope);
        assert!(result.is_ok());
        
        // Verify it was captured correctly
        let piped_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(piped_messages.len(), 1);
        
        // Parse the sent payload to verify integrity
        let parse_result = Communication::<MockZellijService>::parse_incoming_message(&piped_messages[0].0);
        assert!(parse_result.is_ok());
        
        // Test Unicode characters
        let unicode_message = CoordinationMessage::FileChanged {
            file_path: "/path/to/测试文件-🚀.rs".to_string(),
            event_type: "modified-тест".to_string(),
        };
        let unicode_envelope = MessageEnvelope::new_broadcast(unicode_message, "sender-测试-🎯");
        
        let unicode_result = communication.send_pipe_message(&unicode_envelope);
        assert!(unicode_result.is_ok());
        
        // Verify Unicode handling
        let updated_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(updated_messages.len(), 2);
        
        let unicode_parse = Communication::<MockZellijService>::parse_incoming_message(&updated_messages[1].0);
        assert!(unicode_parse.is_ok());
    }

    #[test]
    fn test_send_pipe_message_with_multiple_calls() {
        let communication = create_test_communication();
        
        // Send multiple messages
        for i in 0..5 {
            let message = CoordinationMessage::TaskCompleted {
                task_id: format!("task-{}", i),
            };
            let envelope = MessageEnvelope::new_broadcast(message, &format!("sender-{}", i));
            
            let result = communication.send_pipe_message(&envelope);
            assert!(result.is_ok());
        }
        
        // Verify all were captured
        let piped_messages = communication.zellij_service.get_piped_messages();
        assert_eq!(piped_messages.len(), 5);
        
        // Verify all target "coordination"
        for (_, target) in &piped_messages {
            assert_eq!(target, "coordination");
        }
        
        // Verify payloads are different
        let payloads: std::collections::HashSet<_> = piped_messages.iter().map(|(p, _)| p).collect();
        assert_eq!(payloads.len(), 5); // All unique
    }

    #[test]
    fn test_send_pipe_message_serialization_error() {
        // Note: This test is tricky because MessageEnvelope has proper Serialize implementation
        // We can't easily force a serialization error with normal data
        // This test exists to document the expected behavior
        let communication = create_test_communication();
        let envelope = create_test_envelope();
        
        // Normal envelope should serialize successfully
        let result = communication.send_pipe_message(&envelope);
        assert!(result.is_ok());
        
        // If serialization ever fails, it would propagate as CommunicationError::SerializationError
        // due to the From<serde_json::Error> implementation in CommunicationError
    }

    #[test]
    fn test_parse_incoming_message_empty_and_whitespace() {
        // Test empty string
        let empty_result = Communication::<MockZellijService>::parse_incoming_message("");
        assert!(empty_result.is_err());
        
        // Test whitespace only
        let whitespace_result = Communication::<MockZellijService>::parse_incoming_message("   \t\n  ");
        assert!(whitespace_result.is_err());
        
        // Test null
        let null_result = Communication::<MockZellijService>::parse_incoming_message("null");
        assert!(null_result.is_err());
        
        // Test array instead of object
        let array_result = Communication::<MockZellijService>::parse_incoming_message("[1,2,3]");
        assert!(array_result.is_err());
    }
}
