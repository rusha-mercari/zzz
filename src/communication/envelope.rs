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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination_message::CoordinationMessage;
    use crate::workflow_phase::WorkflowPhase;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_message() -> CoordinationMessage {
        CoordinationMessage::StartPlanning {
            task_id: 123,
            task_description: "Test task".to_string(),
        }
    }

    #[test]
    fn test_new_targeted_creates_correct_envelope() {
        let message = create_test_message();
        let target_pane = "test-pane";
        let sender = "test-sender";

        let envelope = MessageEnvelope::new_targeted(message.clone(), target_pane, sender);

        assert_eq!(envelope.target_pane, Some(target_pane.to_string()));
        assert_eq!(envelope.sender, sender);
        assert!(matches!(
            envelope.coordination_message,
            CoordinationMessage::StartPlanning { .. }
        ));
        assert!(envelope.timestamp > 0);

        // Verify timestamp is recent (within last few seconds)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(envelope.timestamp <= now);
        assert!(envelope.timestamp > now - 5);
    }

    #[test]
    fn test_new_broadcast_creates_correct_envelope() {
        let message = create_test_message();
        let sender = "broadcast-sender";

        let envelope = MessageEnvelope::new_broadcast(message.clone(), sender);

        assert_eq!(envelope.target_pane, None);
        assert_eq!(envelope.sender, sender);
        assert!(matches!(
            envelope.coordination_message,
            CoordinationMessage::StartPlanning { .. }
        ));
        assert!(envelope.timestamp > 0);

        // Verify timestamp is recent
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(envelope.timestamp <= now);
        assert!(envelope.timestamp > now - 5);
    }

    #[test]
    fn test_is_targeted_to_with_targeted_message() {
        let message = create_test_message();
        let target_pane = "specific-pane";
        let envelope = MessageEnvelope::new_targeted(message, target_pane, "sender");

        assert!(envelope.is_targeted_to(target_pane));
        assert!(!envelope.is_targeted_to("different-pane"));
        assert!(!envelope.is_targeted_to(""));
    }

    #[test]
    fn test_is_targeted_to_with_broadcast_message() {
        let message = create_test_message();
        let envelope = MessageEnvelope::new_broadcast(message, "sender");

        // Broadcast messages should be targeted to everyone
        assert!(envelope.is_targeted_to("any-pane"));
        assert!(envelope.is_targeted_to("another-pane"));
        assert!(envelope.is_targeted_to(""));
    }

    #[test]
    fn test_is_broadcast_identification() {
        let message = create_test_message();

        let targeted_envelope = MessageEnvelope::new_targeted(message.clone(), "pane", "sender");
        let broadcast_envelope = MessageEnvelope::new_broadcast(message, "sender");

        assert!(!targeted_envelope.is_broadcast());
        assert!(broadcast_envelope.is_broadcast());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let message = CoordinationMessage::PhaseTransition {
            from: WorkflowPhase::PlanningInProgress,
            to: WorkflowPhase::ImplementationInProgress,
        };
        let envelope = MessageEnvelope::new_targeted(message, "test-pane", "test-sender");

        // Serialize to JSON
        let serialized = serde_json::to_string(&envelope).expect("Serialization failed");

        // Deserialize back
        let deserialized: MessageEnvelope =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        // Verify all fields match
        assert_eq!(envelope.target_pane, deserialized.target_pane);
        assert_eq!(envelope.sender, deserialized.sender);
        assert_eq!(envelope.timestamp, deserialized.timestamp);

        // Verify coordination message content
        match (
            &envelope.coordination_message,
            &deserialized.coordination_message,
        ) {
            (
                CoordinationMessage::PhaseTransition { from: f1, to: t1 },
                CoordinationMessage::PhaseTransition { from: f2, to: t2 },
            ) => {
                assert_eq!(f1, f2);
                assert_eq!(t1, t2);
            }
            _ => panic!("Coordination message type mismatch after deserialization"),
        }
    }

    #[test]
    fn test_timestamp_generation_validity() {
        let message = create_test_message();
        let envelope1 = MessageEnvelope::new_broadcast(message.clone(), "sender1");

        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(1));

        let envelope2 = MessageEnvelope::new_broadcast(message, "sender2");

        // Timestamps should be valid Unix timestamps
        assert!(envelope1.timestamp > 1_000_000_000); // After year 2001
        assert!(envelope2.timestamp > 1_000_000_000);

        // Second envelope should have later or equal timestamp
        assert!(envelope2.timestamp >= envelope1.timestamp);
    }

    #[test]
    fn test_edge_cases_with_empty_and_special_strings() {
        let message = create_test_message();

        // Test with empty strings
        let envelope_empty_target = MessageEnvelope::new_targeted(message.clone(), "", "sender");
        let envelope_empty_sender = MessageEnvelope::new_targeted(message.clone(), "pane", "");

        assert_eq!(envelope_empty_target.target_pane, Some("".to_string()));
        assert_eq!(envelope_empty_target.sender, "sender");
        assert_eq!(envelope_empty_sender.target_pane, Some("pane".to_string()));
        assert_eq!(envelope_empty_sender.sender, "");

        // Test with special characters
        let special_target = "pane-with-special-chars_123!@#";
        let special_sender = "sender.with.dots/and\\slashes";
        let envelope_special =
            MessageEnvelope::new_targeted(message.clone(), special_target, special_sender);

        assert_eq!(
            envelope_special.target_pane,
            Some(special_target.to_string())
        );
        assert_eq!(envelope_special.sender, special_sender);
        assert!(envelope_special.is_targeted_to(special_target));
        assert!(!envelope_special.is_targeted_to("different"));

        // Test Unicode characters
        let unicode_target = "pane-测试-🚀";
        let unicode_sender = "sender-тест-🎯";
        let envelope_unicode =
            MessageEnvelope::new_targeted(message, unicode_target, unicode_sender);

        assert_eq!(
            envelope_unicode.target_pane,
            Some(unicode_target.to_string())
        );
        assert_eq!(envelope_unicode.sender, unicode_sender);
        assert!(envelope_unicode.is_targeted_to(unicode_target));
    }

    #[test]
    fn test_different_coordination_message_types() {
        let messages = vec![
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

        for (i, message) in messages.into_iter().enumerate() {
            let envelope = MessageEnvelope::new_broadcast(message, &format!("sender-{}", i));

            // Each envelope should be valid
            assert!(envelope.is_broadcast());
            assert_eq!(envelope.sender, format!("sender-{}", i));
            assert!(envelope.timestamp > 0);

            // Should serialize/deserialize without issues
            let serialized = serde_json::to_string(&envelope).expect("Serialization failed");
            let _: MessageEnvelope =
                serde_json::from_str(&serialized).expect("Deserialization failed");
        }
    }
}
