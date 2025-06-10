use std::collections::HashMap;
use zellij_tile::prelude::*;

use super::error::CommunicationError;
use crate::coordination_message::CoordinationMessage;
use crate::pane_role::PaneRole;

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
