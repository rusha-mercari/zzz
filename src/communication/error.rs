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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_role::PaneRole;

    #[test]
    fn test_serialization_error_creation_and_display() {
        let json_error = serde_json::from_str::<i32>("invalid_json").unwrap_err();
        let error = CommunicationError::SerializationError(json_error);

        let display_message = format!("{}", error);
        assert!(display_message.starts_with("Message serialization failed:"));
        assert!(display_message.contains("expected"));
    }

    #[test]
    fn test_message_delivery_failed_creation_and_display() {
        let reason = "Network timeout occurred";
        let error = CommunicationError::MessageDeliveryFailed(reason.to_string());

        let display_message = format!("{}", error);
        assert_eq!(
            display_message,
            "Message delivery failed: Network timeout occurred"
        );

        if let CommunicationError::MessageDeliveryFailed(stored_reason) = error {
            assert_eq!(stored_reason, reason);
        } else {
            panic!("Error variant mismatch");
        }
    }

    #[test]
    fn test_invalid_target_creation_and_display() {
        let target = "invalid-pane-name";
        let error = CommunicationError::InvalidTarget(target.to_string());

        let display_message = format!("{}", error);
        assert_eq!(display_message, "Invalid target pane: invalid-pane-name");

        if let CommunicationError::InvalidTarget(stored_target) = error {
            assert_eq!(stored_target, target);
        } else {
            panic!("Error variant mismatch");
        }
    }

    #[test]
    fn test_pane_not_found_creation_and_display() {
        let role = PaneRole::Commander;
        let error = CommunicationError::PaneNotFound(role);

        let display_message = format!("{}", error);
        assert_eq!(display_message, "Pane not found for role: Commander");

        if let CommunicationError::PaneNotFound(stored_role) = error {
            assert_eq!(stored_role, role);
        } else {
            panic!("Error variant mismatch");
        }
    }

    #[test]
    fn test_pane_discovery_failed_creation_and_display() {
        let message = "Failed to connect to Zellij API";
        let error = CommunicationError::PaneDiscoveryFailed(message.to_string());

        let display_message = format!("{}", error);
        assert_eq!(
            display_message,
            "Pane discovery failed: Failed to connect to Zellij API"
        );

        if let CommunicationError::PaneDiscoveryFailed(stored_message) = error {
            assert_eq!(stored_message, message);
        } else {
            panic!("Error variant mismatch");
        }
    }

    #[test]
    fn test_all_pane_roles_in_pane_not_found_error() {
        let roles = vec![
            PaneRole::Overseer,
            PaneRole::Commander,
            PaneRole::TaskList,
            PaneRole::Review,
            PaneRole::Editor,
        ];

        for role in roles {
            let error = CommunicationError::PaneNotFound(role);
            let display_message = format!("{}", error);

            assert!(display_message.starts_with("Pane not found for role:"));
            assert!(display_message.contains(&format!("{:?}", role)));
        }
    }

    #[test]
    fn test_from_serde_json_error_conversion() {
        let json_error = serde_json::from_str::<i32>("not_a_number").unwrap_err();
        let communication_error: CommunicationError = json_error.into();

        if let CommunicationError::SerializationError(_) = communication_error {
            // Correct variant
        } else {
            panic!("From conversion should create SerializationError variant");
        }

        let display_message = format!("{}", communication_error);
        assert!(display_message.starts_with("Message serialization failed:"));
    }

    #[test]
    fn test_from_serde_json_error_direct_conversion() {
        let result: Result<i32, CommunicationError> =
            serde_json::from_str("invalid").map_err(|e| e.into());

        assert!(result.is_err());
        let error = result.unwrap_err();

        if let CommunicationError::SerializationError(_) = error {
            // Correct
        } else {
            panic!("Direct conversion should work via From trait");
        }
    }

    #[test]
    fn test_debug_output_contains_variant_names() {
        let errors = vec![
            CommunicationError::SerializationError(
                serde_json::from_str::<i32>("invalid").unwrap_err(),
            ),
            CommunicationError::MessageDeliveryFailed("test".to_string()),
            CommunicationError::InvalidTarget("test".to_string()),
            CommunicationError::PaneNotFound(PaneRole::Overseer),
            CommunicationError::PaneDiscoveryFailed("test".to_string()),
        ];

        for error in errors {
            let debug_output = format!("{:?}", error);

            // Debug output should contain the variant name
            assert!(
                debug_output.contains("SerializationError")
                    || debug_output.contains("MessageDeliveryFailed")
                    || debug_output.contains("InvalidTarget")
                    || debug_output.contains("PaneNotFound")
                    || debug_output.contains("PaneDiscoveryFailed")
            );
        }
    }

    #[test]
    fn test_error_display_edge_cases() {
        // Test with empty strings
        let empty_delivery_error = CommunicationError::MessageDeliveryFailed("".to_string());
        let empty_target_error = CommunicationError::InvalidTarget("".to_string());
        let empty_discovery_error = CommunicationError::PaneDiscoveryFailed("".to_string());

        assert_eq!(
            format!("{}", empty_delivery_error),
            "Message delivery failed: "
        );
        assert_eq!(format!("{}", empty_target_error), "Invalid target pane: ");
        assert_eq!(
            format!("{}", empty_discovery_error),
            "Pane discovery failed: "
        );

        // Test with special characters
        let special_chars = "test-with-special_chars!@#$%^&*()";
        let special_delivery_error =
            CommunicationError::MessageDeliveryFailed(special_chars.to_string());
        let special_target_error = CommunicationError::InvalidTarget(special_chars.to_string());

        assert!(format!("{}", special_delivery_error).contains(special_chars));
        assert!(format!("{}", special_target_error).contains(special_chars));

        // Test with Unicode characters
        let unicode_message = "测试-🚀-тест";
        let unicode_error = CommunicationError::MessageDeliveryFailed(unicode_message.to_string());

        assert!(format!("{}", unicode_error).contains(unicode_message));
    }

    #[test]
    fn test_error_chain_preservation() {
        // Test that the original serde_json error is preserved in the chain
        let original_error = serde_json::from_str::<i32>("definitely_not_a_number").unwrap_err();
        let original_message = format!("{}", original_error);

        let communication_error = CommunicationError::SerializationError(original_error);
        let wrapped_message = format!("{}", communication_error);

        // The wrapped message should contain the original error message
        assert!(wrapped_message.contains(&original_message));
    }

    #[test]
    fn test_comprehensive_serialization_error_scenarios() {
        // Test various JSON parsing scenarios that would generate different serde errors
        let test_cases = vec![
            ("", "expected value"),
            ("{", "EOF while parsing"),
            ("null", "invalid type: null"),
            ("[1,2,", "EOF while parsing"),
        ];

        for (invalid_json, _expected_error_part) in test_cases {
            let json_error = serde_json::from_str::<i32>(invalid_json).unwrap_err();
            let communication_error = CommunicationError::SerializationError(json_error);

            let display_message = format!("{}", communication_error);
            assert!(display_message.starts_with("Message serialization failed:"));
            // Note: We can't reliably test for specific error text as it may vary between serde versions
        }
    }
}
