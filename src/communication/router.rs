use std::collections::HashMap;
use zellij_tile::prelude::*;

use super::error::CommunicationError;
use crate::coordination_message::CoordinationMessage;
use crate::pane_role::PaneRole;
use crate::zellij_service::ZellijService;

/// Message router for dispatching coordination messages by pane role
pub struct MessageRouter<T: ZellijService> {
    /// Mapping from pane roles to their pane IDs
    pane_registry: HashMap<PaneRole, PaneId>,
    /// Service for interacting with Zellij APIs
    zellij_service: T,
}

impl<T: ZellijService> MessageRouter<T> {
    /// Create a new message router
    pub fn new(zellij_service: T) -> Self {
        Self {
            pane_registry: HashMap::new(),
            zellij_service,
        }
    }

    /// Discover panes and map them to roles based on their names using real Zellij API
    pub fn discover_panes_with_manifest(
        &mut self,
        pane_manifest: &PaneManifest,
    ) -> Result<(), CommunicationError> {
        // Clear existing registry
        self.pane_registry.clear();

        let mut discovered_panes = 0;

        // Iterate through all tabs and their panes
        for panes in pane_manifest.panes.values() {
            for pane_info in panes {
                // Try to match the pane title to a role
                if let Some(role) = Self::match_pane_name_to_role(&pane_info.title) {
                    // Create the correct PaneId based on the pane type
                    let pane_id = if pane_info.is_plugin {
                        PaneId::Plugin(pane_info.id)
                    } else {
                        PaneId::Terminal(pane_info.id)
                    };

                    // Register the pane with its role
                    self.pane_registry.insert(role, pane_id);
                    discovered_panes += 1;
                }
            }
        }

        if discovered_panes == 0 {
            return Err(CommunicationError::PaneDiscoveryFailed(
                "No matching panes found in current layout".to_string(),
            ));
        }

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
        let pane_id = self
            .get_pane_id(&target_role)
            .ok_or(CommunicationError::PaneNotFound(target_role))?;

        // Serialize the message
        let message_json =
            serde_json::to_string(message).map_err(CommunicationError::SerializationError)?;

        // Write the message to the target pane
        self.zellij_service
            .write_chars_to_pane_id(&message_json, pane_id);

        Ok(())
    }

    /// Execute a raw command in a specific pane role
    pub fn execute_command_in_role(
        &self,
        command: &str,
        target_role: PaneRole,
    ) -> Result<(), CommunicationError> {
        // Look up the pane ID for the target role
        let pane_id = self
            .get_pane_id(&target_role)
            .ok_or(CommunicationError::PaneNotFound(target_role))?;

        // Write the command directly to the target pane
        self.zellij_service.write_chars_to_pane_id(command, pane_id);

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
    pub fn broadcast_to_all(
        &self,
        message: &CoordinationMessage,
    ) -> Vec<(PaneRole, Result<(), CommunicationError>)> {
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

    /// Get access to the zellij service (for testing)
    #[cfg(test)]
    pub fn get_zellij_service(&self) -> &T {
        &self.zellij_service
    }

    /// Match pane name to role using pattern matching
    pub fn match_pane_name_to_role(pane_name: &str) -> Option<PaneRole> {
        match pane_name.to_lowercase().as_str() {
            name if name.contains("overseer") => Some(PaneRole::Overseer),
            name if name.contains("commander") => Some(PaneRole::Commander),
            name if name.contains("task list") => Some(PaneRole::TaskList),
            name if name.contains("review") => Some(PaneRole::Review),
            name if name.contains("editor") => Some(PaneRole::Editor),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_phase::WorkflowPhase;
    use crate::zellij_service::MockZellijService;
    use std::collections::HashMap;

    /// Mock PaneInfo for testing
    fn create_mock_pane_info(id: u32, title: &str, is_plugin: bool) -> PaneInfo {
        PaneInfo {
            id,
            is_plugin,
            is_focused: false,
            is_fullscreen: false,
            is_floating: false,
            is_suppressed: false,
            title: title.to_string(),
            exited: false,
            exit_status: None,
            is_held: false,
            pane_x: 0,
            pane_content_x: 0,
            pane_y: 0,
            pane_content_y: 0,
            pane_rows: 10,
            pane_content_rows: 8,
            pane_columns: 80,
            pane_content_columns: 78,
            cursor_coordinates_in_pane: None,
            terminal_command: None,
            plugin_url: if is_plugin {
                Some("test://plugin".to_string())
            } else {
                None
            },
            is_selectable: true,
        }
    }

    /// Create a mock PaneManifest for testing
    fn create_mock_pane_manifest() -> PaneManifest {
        let mut panes = HashMap::new();

        // Tab 0 with various panes
        let tab_0_panes = vec![
            create_mock_pane_info(1, "Overseer AI Assistant", true),
            create_mock_pane_info(2, "Commander Terminal", false),
            create_mock_pane_info(3, "Task List Manager", true),
            create_mock_pane_info(4, "Code Review Panel", true),
            create_mock_pane_info(5, "Main Editor", false),
        ];

        panes.insert(0, tab_0_panes);

        PaneManifest { panes }
    }

    /// Create empty PaneManifest for testing
    fn create_empty_pane_manifest() -> PaneManifest {
        PaneManifest {
            panes: HashMap::new(),
        }
    }

    #[test]
    fn test_new_router() {
        let mock_service = MockZellijService::new();
        let router = MessageRouter::new(mock_service);

        assert_eq!(router.get_registered_roles().len(), 0);
    }

    #[test]
    fn test_register_pane() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);

        let pane_id = PaneId::Terminal(42);
        router.register_pane(PaneRole::Overseer, pane_id);

        assert_eq!(router.get_pane_id(&PaneRole::Overseer), Some(pane_id));
        assert!(router.is_role_registered(&PaneRole::Overseer));
        assert!(!router.is_role_registered(&PaneRole::Commander));
    }

    #[test]
    fn test_get_registered_roles() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);

        router.register_pane(PaneRole::Overseer, PaneId::Terminal(1));
        router.register_pane(PaneRole::Commander, PaneId::Plugin(2));

        let roles = router.get_registered_roles();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&PaneRole::Overseer));
        assert!(roles.contains(&PaneRole::Commander));
    }

    #[test]
    fn test_match_pane_name_to_role() {
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Overseer AI Assistant"),
            Some(PaneRole::Overseer)
        );
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Commander Terminal"),
            Some(PaneRole::Commander)
        );
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Task List Manager"),
            Some(PaneRole::TaskList)
        );
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Code Review Panel"),
            Some(PaneRole::Review)
        );
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Main Editor"),
            Some(PaneRole::Editor)
        );
        assert_eq!(
            MessageRouter::<MockZellijService>::match_pane_name_to_role("Random Pane"),
            None
        );
    }

    #[test]
    fn test_discover_panes_with_manifest() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);
        let manifest = create_mock_pane_manifest();

        let result = router.discover_panes_with_manifest(&manifest);
        assert!(result.is_ok());

        // Check that all roles were discovered
        assert!(router.is_role_registered(&PaneRole::Overseer));
        assert!(router.is_role_registered(&PaneRole::Commander));
        assert!(router.is_role_registered(&PaneRole::TaskList));
        assert!(router.is_role_registered(&PaneRole::Review));
        assert!(router.is_role_registered(&PaneRole::Editor));

        // Check correct pane IDs
        assert_eq!(
            router.get_pane_id(&PaneRole::Overseer),
            Some(PaneId::Plugin(1))
        );
        assert_eq!(
            router.get_pane_id(&PaneRole::Commander),
            Some(PaneId::Terminal(2))
        );
        assert_eq!(
            router.get_pane_id(&PaneRole::TaskList),
            Some(PaneId::Plugin(3))
        );
        assert_eq!(
            router.get_pane_id(&PaneRole::Review),
            Some(PaneId::Plugin(4))
        );
        assert_eq!(
            router.get_pane_id(&PaneRole::Editor),
            Some(PaneId::Terminal(5))
        );
    }

    #[test]
    fn test_discover_panes_empty_manifest() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);
        let manifest = create_empty_pane_manifest();

        let result = router.discover_panes_with_manifest(&manifest);
        assert!(result.is_err());

        if let Err(CommunicationError::PaneDiscoveryFailed(msg)) = result {
            assert!(msg.contains("No matching panes found"));
        } else {
            panic!("Expected PaneDiscoveryFailed error");
        }
    }

    #[test]
    fn test_route_message_to_role() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);

        // Register a pane
        let pane_id = PaneId::Terminal(42);
        router.register_pane(PaneRole::Overseer, pane_id);

        // Create a test message
        let message = CoordinationMessage::StartPlanning {
            task_id: 123,
            task_description: "Test task".to_string(),
        };

        // Route the message
        let result = router.route_message_to_role(&message, PaneRole::Overseer);
        assert!(result.is_ok());

        // Check that the message was sent
        let sent_messages = router.get_zellij_service().get_sent_messages();
        assert_eq!(sent_messages.len(), 1);

        let (sent_message, sent_pane_id) = &sent_messages[0];
        assert_eq!(*sent_pane_id, pane_id);

        // Verify the message was serialized correctly
        let parsed_message: CoordinationMessage = serde_json::from_str(sent_message).unwrap();
        if let CoordinationMessage::StartPlanning {
            task_id,
            task_description,
        } = parsed_message
        {
            assert_eq!(task_id, 123);
            assert_eq!(task_description, "Test task");
        } else {
            panic!("Message was not serialized correctly");
        }
    }

    #[test]
    fn test_route_message_to_unregistered_role() {
        let mock_service = MockZellijService::new();
        let router = MessageRouter::new(mock_service);

        let message = CoordinationMessage::StartImplementation;
        let result = router.route_message_to_role(&message, PaneRole::Commander);

        assert!(result.is_err());
        if let Err(CommunicationError::PaneNotFound(role)) = result {
            assert_eq!(role, PaneRole::Commander);
        } else {
            panic!("Expected PaneNotFound error");
        }
    }

    #[test]
    fn test_route_message_to_roles() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);

        // Register multiple panes
        router.register_pane(PaneRole::Overseer, PaneId::Plugin(1));
        router.register_pane(PaneRole::Commander, PaneId::Terminal(2));

        let message = CoordinationMessage::AllTasksComplete;
        let target_roles = [PaneRole::Overseer, PaneRole::Commander, PaneRole::TaskList]; // TaskList not registered

        let results = router.route_message_to_roles(&message, &target_roles);
        assert_eq!(results.len(), 3);

        // Check results
        let overseer_result = results
            .iter()
            .find(|(role, _)| *role == PaneRole::Overseer)
            .unwrap();
        assert!(overseer_result.1.is_ok());

        let commander_result = results
            .iter()
            .find(|(role, _)| *role == PaneRole::Commander)
            .unwrap();
        assert!(commander_result.1.is_ok());

        let tasklist_result = results
            .iter()
            .find(|(role, _)| *role == PaneRole::TaskList)
            .unwrap();
        assert!(tasklist_result.1.is_err());

        // Check that messages were sent to registered panes
        let sent_messages = router.get_zellij_service().get_sent_messages();
        assert_eq!(sent_messages.len(), 2);
    }

    #[test]
    fn test_broadcast_to_all() {
        let mock_service = MockZellijService::new();
        let mut router = MessageRouter::new(mock_service);

        // Register multiple panes
        router.register_pane(PaneRole::Overseer, PaneId::Plugin(1));
        router.register_pane(PaneRole::Commander, PaneId::Terminal(2));
        router.register_pane(PaneRole::Editor, PaneId::Terminal(3));

        let message = CoordinationMessage::PhaseTransition {
            from: WorkflowPhase::Initializing,
            to: WorkflowPhase::PlanningInProgress,
        };

        let results = router.broadcast_to_all(&message);
        assert_eq!(results.len(), 3);

        // All should succeed
        for (_, result) in &results {
            assert!(result.is_ok());
        }

        // Check that messages were sent to all panes
        let sent_messages = router.get_zellij_service().get_sent_messages();
        assert_eq!(sent_messages.len(), 3);

        // Verify all messages are the same
        for (sent_message, _) in &sent_messages {
            let parsed: CoordinationMessage = serde_json::from_str(sent_message).unwrap();
            if let CoordinationMessage::PhaseTransition { from, to } = parsed {
                assert_eq!(from, WorkflowPhase::Initializing);
                assert_eq!(to, WorkflowPhase::PlanningInProgress);
            } else {
                panic!("Unexpected message type");
            }
        }
    }
}
