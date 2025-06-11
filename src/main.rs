mod communication;
mod coordination_message;
mod file_system;
mod litellm_config;
mod notification;
mod pane_role;
mod workflow_phase;
mod zellij_service;

use communication::{
    Communication, CommunicationError, MessageEnvelope, MessageRouter, ParsedMessage,
};
use coordination_message::CoordinationMessage;
use file_system::{FileSystem, FileSystemError};
use litellm_config::LiteLLMConfig;
use notification::Notification;
use notify::Watcher;
use pane_role::PaneRole;
use std::collections::BTreeMap;
use workflow_phase::WorkflowPhase;
use zellij_service::ZellijServiceImpl;
use zellij_tile::prelude::*;

struct State {
    task_id: u32,
    task_description: String,
    current_phase: WorkflowPhase,
    file_watcher: Option<Box<dyn Watcher>>,
    pending_notifications: Vec<Notification>,
    received_messages: Vec<CoordinationMessage>,
    last_message: Option<String>,
    message_router: MessageRouter<ZellijServiceImpl>,
    communication: Communication<ZellijServiceImpl>,
    permissions_granted: bool,
    pane_manifest: Option<PaneManifest>,
    litellm_config: LiteLLMConfig,
}

impl Default for State {
    fn default() -> Self {
        Self {
            task_id: 0,
            task_description: "Default task description".to_string(),
            current_phase: WorkflowPhase::Initializing,
            file_watcher: None,
            pending_notifications: Vec::new(),
            received_messages: Vec::new(),
            last_message: None,
            message_router: MessageRouter::new(ZellijServiceImpl),
            communication: Communication::new(ZellijServiceImpl),
            permissions_granted: false,
            pane_manifest: None,
            litellm_config: LiteLLMConfig::default(),
        }
    }
}

impl State {
    /// Sets up the complete directory structure for the current task
    fn setup_task_directories(&self) -> Result<std::path::PathBuf, std::io::Error> {
        FileSystem::setup_task_directories(self.task_id)
    }

    /// Gets the path to the todo-list.md file for the current task
    fn get_todo_list_path(&self) -> std::path::PathBuf {
        FileSystem::get_todo_list_path(self.task_id)
    }

    /// Gets the path to the review.md file for the current task
    fn get_review_path(&self) -> std::path::PathBuf {
        FileSystem::get_review_path(self.task_id)
    }

    /// Gets the path to the plan.md file for the current task
    fn get_plan_path(&self) -> std::path::PathBuf {
        FileSystem::get_plan_path(self.task_id)
    }

    /// Gets the path to the logs directory for the current task
    fn get_logs_dir_path(&self) -> std::path::PathBuf {
        FileSystem::get_logs_dir_path(self.task_id)
    }

    /// Gets the path to the overseer.log file for the current task
    fn get_overseer_log_path(&self) -> std::path::PathBuf {
        FileSystem::get_overseer_log_path(self.task_id)
    }

    /// Gets the path to the commander.log file for the current task
    fn get_commander_log_path(&self) -> std::path::PathBuf {
        FileSystem::get_commander_log_path(self.task_id)
    }

    /// Gets the path to the coordinator.log file for the current task
    fn get_coordinator_log_path(&self) -> std::path::PathBuf {
        FileSystem::get_coordinator_log_path(self.task_id)
    }

    /// Atomically writes content to the todo-list.md file
    fn write_todo_list(&self, content: &str) -> Result<(), FileSystemError> {
        let path = self.get_todo_list_path();
        FileSystem::write_file_atomic(path, content)
    }

    /// Safely reads the todo-list.md file content
    fn read_todo_list(&self) -> Result<String, FileSystemError> {
        let path = self.get_todo_list_path();
        FileSystem::read_file_safe(path)
    }

    /// Atomically writes content to the review.md file
    fn write_review(&self, content: &str) -> Result<(), FileSystemError> {
        let path = self.get_review_path();
        FileSystem::write_file_atomic(path, content)
    }

    /// Safely reads the review.md file content
    fn read_review(&self) -> Result<String, FileSystemError> {
        let path = self.get_review_path();
        FileSystem::read_file_safe(path)
    }

    /// Atomically writes content to the plan.md file
    fn write_plan(&self, content: &str) -> Result<(), FileSystemError> {
        let path = self.get_plan_path();
        FileSystem::write_file_atomic(path, content)
    }

    /// Safely reads the plan.md file content
    fn read_plan(&self) -> Result<String, FileSystemError> {
        let path = self.get_plan_path();
        FileSystem::read_file_safe(path)
    }

    /// Appends a log entry to the coordinator log
    fn log_coordinator(&self, message: &str) -> Result<(), FileSystemError> {
        let path = self.get_coordinator_log_path();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let log_entry = format!("[{}] {}\n", timestamp, message);
        FileSystem::append_to_file(path, &log_entry)
    }

    /// Checks if the todo-list.md file exists
    fn todo_list_exists(&self) -> bool {
        let path = self.get_todo_list_path();
        FileSystem::file_exists(path)
    }

    /// Checks if the review.md file exists
    fn review_exists(&self) -> bool {
        let path = self.get_review_path();
        FileSystem::file_exists(path)
    }

    /// Ensures all required files exist for the current task
    fn ensure_task_files_exist(&self) -> Result<(), FileSystemError> {
        // Create the directory structure first
        self.setup_task_directories()
            .map_err(FileSystemError::from)?;

        // Ensure log files exist
        FileSystem::ensure_file_exists(self.get_coordinator_log_path())?;
        FileSystem::ensure_file_exists(self.get_overseer_log_path())?;
        FileSystem::ensure_file_exists(self.get_commander_log_path())?;

        Ok(())
    }

    // === Communication Methods ===

    /// Send a coordination message to a specific pane by title
    ///
    /// # Arguments
    /// * `message` - The coordination message to send
    /// * `target_pane_title` - Title of the target pane (e.g., "Overseer", "Commander")
    ///
    /// # Returns
    /// * `Ok(())` if message was sent successfully
    /// * `Err(CommunicationError)` if sending failed
    fn send_coordination_message(
        &self,
        message: CoordinationMessage,
        target_pane_title: &str,
    ) -> Result<(), CommunicationError> {
        // Create envelope with target pane
        let envelope =
            MessageEnvelope::new_targeted(message.clone(), target_pane_title, "zzz-coordinator");

        // Log the outgoing message
        let log_msg = format!("Sending message to '{}': {:?}", target_pane_title, message);
        let _ = self.log_coordinator(&log_msg);

        // Send the message
        match self.communication.send_pipe_message(&envelope) {
            Ok(()) => {
                let success_msg = format!(
                    "Successfully sent message to '{}': {:?}",
                    target_pane_title, message
                );
                let _ = self.log_coordinator(&success_msg);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to send message to '{}': {}", target_pane_title, e);
                let _ = self.log_coordinator(&error_msg);
                Err(e)
            }
        }
    }

    /// Broadcast a coordination message to all listening panes
    ///
    /// # Arguments
    /// * `message` - The coordination message to broadcast
    ///
    /// # Returns
    /// * `Ok(())` if message was sent successfully
    /// * `Err(CommunicationError)` if sending failed
    fn broadcast_coordination_message(
        &self,
        message: CoordinationMessage,
    ) -> Result<(), CommunicationError> {
        // Create envelope for broadcasting
        let envelope = MessageEnvelope::new_broadcast(message.clone(), "zzz-coordinator");

        // Log the outgoing message
        let log_msg = format!("Broadcasting message: {:?}", message);
        let _ = self.log_coordinator(&log_msg);

        // Send the message
        match self.communication.send_pipe_message(&envelope) {
            Ok(()) => {
                let success_msg = format!("Successfully broadcast message: {:?}", message);
                let _ = self.log_coordinator(&success_msg);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to broadcast message: {}", e);
                let _ = self.log_coordinator(&error_msg);
                Err(e)
            }
        }
    }

    /// Handle incoming message payload with enhanced parsing
    fn handle_incoming_message(
        &mut self,
        payload: &str,
        source: &str,
        _input_id: Option<String>,
    ) -> bool {
        // Try to parse the payload using the new parsing logic
        match Communication::<ZellijServiceImpl>::parse_incoming_message(payload) {
            Ok(ParsedMessage::Envelope(envelope)) => {
                // Handle modern envelope format
                self.handle_envelope_message(envelope, source)
            }
            Ok(ParsedMessage::Legacy(message)) => {
                // Handle legacy direct CoordinationMessage format
                self.handle_legacy_message(message, source)
            }
            Err(_) => {
                // Handle as raw text message
                self.handle_raw_message(payload, source)
            }
        }
    }

    /// Handle a message in the modern envelope format
    fn handle_envelope_message(&mut self, envelope: MessageEnvelope, source: &str) -> bool {
        let message = &envelope.coordination_message;

        // Store the coordination message
        self.received_messages.push(message.clone());

        // Create display message with envelope info
        let display = if let Some(ref target) = envelope.target_pane {
            format!(
                "Envelope from {} → {}: {:?} (sent by {} at {})",
                source, target, message, envelope.sender, envelope.timestamp
            )
        } else {
            format!(
                "Broadcast from {}: {:?} (sent by {} at {})",
                source, message, envelope.sender, envelope.timestamp
            )
        };

        self.last_message = Some(display.clone());

        // Log the received envelope
        let log_msg = format!(
            "Received envelope from {}: target={:?}, sender={}, message={:?}",
            source, envelope.target_pane, envelope.sender, message
        );
        let _ = self.log_coordinator(&log_msg);

        true // trigger re-render
    }

    /// Handle a message in the legacy direct CoordinationMessage format
    fn handle_legacy_message(&mut self, message: CoordinationMessage, source: &str) -> bool {
        self.received_messages.push(message.clone());
        self.last_message = Some(format!("Legacy from {}: {:?}", source, message));

        // Log the legacy message
        let log_msg = format!("Received legacy message from {}: {:?}", source, message);
        let _ = self.log_coordinator(&log_msg);

        true // trigger re-render
    }

    /// Handle a raw text message that couldn't be parsed as JSON
    fn handle_raw_message(&mut self, payload: &str, source: &str) -> bool {
        self.last_message = Some(format!("Raw from {}: {}", source, payload));

        // Log the raw message
        let log_msg = format!("Received raw message from {}: {}", source, payload);
        let _ = self.log_coordinator(&log_msg);

        true // trigger re-render
    }

    // === Message Routing Methods ===

    /// Send a coordination message to a specific pane role using the router
    fn route_message_to_role(
        &self,
        message: CoordinationMessage,
        target_role: PaneRole,
    ) -> Result<(), CommunicationError> {
        match self
            .message_router
            .route_message_to_role(&message, target_role)
        {
            Ok(()) => {
                let log_msg = format!(
                    "Successfully routed message to {:?}: {:?}",
                    target_role, message
                );
                let _ = self.log_coordinator(&log_msg);
                Ok(())
            }
            Err(CommunicationError::PaneNotFound(role)) => {
                let error_msg = format!(
                    "Pane not found for role {:?} when trying to send message: {:?}",
                    role, message
                );
                let _ = self.log_coordinator(&error_msg);
                Err(CommunicationError::PaneNotFound(role))
            }
            Err(e) => {
                let error_msg = format!("Failed to route message to {:?}: {}", target_role, e);
                let _ = self.log_coordinator(&error_msg);
                Err(e)
            }
        }
    }

    /// Send a coordination message to multiple pane roles
    fn route_message_to_roles(
        &self,
        message: CoordinationMessage,
        target_roles: &[PaneRole],
    ) -> Vec<(PaneRole, Result<(), CommunicationError>)> {
        let results = self
            .message_router
            .route_message_to_roles(&message, target_roles);

        // Log results
        for (role, result) in &results {
            match result {
                Ok(()) => {
                    let log_msg =
                        format!("Successfully routed message to {:?}: {:?}", role, message);
                    let _ = self.log_coordinator(&log_msg);
                }
                Err(e) => {
                    let error_msg = format!("Failed to route message to {:?}: {}", role, e);
                    let _ = self.log_coordinator(&error_msg);
                }
            }
        }

        results
    }

    /// Broadcast a coordination message to all registered panes
    fn broadcast_to_all_roles(
        &self,
        message: CoordinationMessage,
    ) -> Vec<(PaneRole, Result<(), CommunicationError>)> {
        let results = self.message_router.broadcast_to_all(&message);

        let log_msg = format!("Broadcasting message to all roles: {:?}", message);
        let _ = self.log_coordinator(&log_msg);

        // Log individual results
        for (role, result) in &results {
            match result {
                Ok(()) => {
                    let log_msg = format!("Broadcast successful to {:?}", role);
                    let _ = self.log_coordinator(&log_msg);
                }
                Err(e) => {
                    let error_msg = format!("Broadcast failed to {:?}: {}", role, e);
                    let _ = self.log_coordinator(&error_msg);
                }
            }
        }

        results
    }

    /// Discover and register panes based on their names/titles using current manifest
    fn discover_and_register_panes(&mut self) {
        let log_msg = "Attempting to discover panes...".to_string();
        let _ = self.log_coordinator(&log_msg);

        if let Some(ref manifest) = self.pane_manifest {
            match self.message_router.discover_panes_with_manifest(manifest) {
                Ok(()) => {
                    let discovered_roles = self.message_router.get_registered_roles();
                    let log_msg = format!(
                        "Pane discovery completed successfully. Found {} panes: {:?}",
                        discovered_roles.len(),
                        discovered_roles
                    );
                    let _ = self.log_coordinator(&log_msg);
                }
                Err(e) => {
                    let error_msg = format!("Pane discovery failed: {}", e);
                    let _ = self.log_coordinator(&error_msg);
                }
            }
        } else {
            let error_msg = "No pane manifest available for discovery".to_string();
            let _ = self.log_coordinator(&error_msg);
        }
    }

    /// Get the list of registered pane roles
    fn get_registered_roles(&self) -> Vec<PaneRole> {
        self.message_router.get_registered_roles()
    }

    /// Check if a specific role is registered
    fn is_role_registered(&self, role: &PaneRole) -> bool {
        self.message_router.is_role_registered(role)
    }

    /// Send the initial StartPlanning message to the Overseer pane
    fn send_start_planning_message(&self) {
        // Create a StartPlanning message with configured task info
        let start_planning_msg = CoordinationMessage::StartPlanning {
            task_id: self.task_id,
            task_description: self.task_description.clone(),
        };

        // Try to send to Overseer pane using role-based routing
        match self.route_message_to_role(start_planning_msg.clone(), PaneRole::Overseer) {
            Ok(()) => {
                let success_msg = "Successfully sent StartPlanning message to Overseer".to_string();
                let _ = self.log_coordinator(&success_msg);

                // Update workflow phase to PlanningInProgress
                // Note: This would need mutable self, so we'll log it for now
                let phase_msg =
                    "Workflow phase should transition to PlanningInProgress".to_string();
                let _ = self.log_coordinator(&phase_msg);
            }
            Err(e) => {
                let error_msg = format!("Failed to send StartPlanning message to Overseer: {}", e);
                let _ = self.log_coordinator(&error_msg);

                // Fall back to direct pane targeting by name
                match self.send_coordination_message(start_planning_msg, "Overseer") {
                    Ok(()) => {
                        let fallback_msg =
                            "Successfully sent StartPlanning via direct pane targeting".to_string();
                        let _ = self.log_coordinator(&fallback_msg);
                    }
                    Err(fallback_err) => {
                        let fallback_error =
                            format!("Both routing methods failed: {}", fallback_err);
                        let _ = self.log_coordinator(&fallback_error);
                    }
                }
            }
        }
    }
}

register_plugin!(State);

// More info on plugins: https://zellij.dev/documentation/plugins

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Read configuration parameters
        if let Some(task_id_str) = configuration.get("task_id") {
            if let Ok(parsed_id) = task_id_str.parse::<u32>() {
                self.task_id = parsed_id;
                let _ = self.log_coordinator(&format!(
                    "Loaded task_id from configuration: {}",
                    self.task_id
                ));
            } else {
                let _ = self.log_coordinator(&format!(
                    "Warning: Invalid task_id in configuration: {}",
                    task_id_str
                ));
            }
        }

        if let Some(task_desc) = configuration.get("task_description") {
            self.task_description = task_desc.clone();
            let _ = self.log_coordinator(&format!(
                "Loaded task_description from configuration: {}",
                self.task_description
            ));
        }

        // Load LiteLLM configuration
        if let Some(api_key) = configuration.get("api_key") {
            self.litellm_config.api_key = api_key.clone();
            let _ = self.log_coordinator("Loaded API key from configuration");
        }

        if let Some(url) = configuration.get("litellm_url") {
            self.litellm_config.url = url.clone();
            let _ = self.log_coordinator(&format!(
                "Loaded LiteLLM URL from configuration: {}",
                self.litellm_config.url
            ));
        }

        // Request permissions needed for pane discovery and writing to panes
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::WriteToStdin,
        ]);

        // Subscribe to permission results and layout events
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::PaneUpdate,
            EventType::TabUpdate,
        ]);

        // Initialize task directories
        match self.ensure_task_files_exist() {
            Ok(()) => {
                let success_msg = format!(
                    "Successfully created task directories for task {}",
                    self.task_id
                );
                let _ = self.log_coordinator(&success_msg);
            }
            Err(e) => {
                let error_msg = format!(
                    "CRITICAL: Failed to create task directories for task {}: {:?}",
                    self.task_id, e
                );
                let _ = self.log_coordinator(&error_msg);
                // Note: Plugin continues to run even if directory creation fails
                // This allows the UI to show the error state
            }
        }
    }
    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(_permission_status) => {
                // Log that we received a permission result
                let log_msg = format!("Permission result received: {:?}", _permission_status);
                let _ = self.log_coordinator(&log_msg);

                // Check if all required permissions are granted
                // For now, we'll assume they are if we get here
                if !self.permissions_granted {
                    self.permissions_granted = true;
                    let _ = self
                        .log_coordinator("All permissions granted, waiting for pane manifest...");
                }

                true // trigger re-render to show permission status
            }
            Event::PaneUpdate(pane_manifest) => {
                // Store the updated pane manifest
                self.pane_manifest = Some(pane_manifest);

                let log_msg =
                    "Received pane manifest update, attempting pane discovery...".to_string();
                let _ = self.log_coordinator(&log_msg);

                // Rediscover panes with the new manifest
                self.discover_and_register_panes();

                // If we have permissions and found panes, send initial message
                if self.permissions_granted && !self.get_registered_roles().is_empty() {
                    self.send_start_planning_message();
                }

                true // trigger re-render to show updated pane information
            }
            Event::TabUpdate(_tab_info) => {
                // Tab structure changed, request updated pane information
                let log_msg = "Tab update received, pane manifest may be outdated".to_string();
                let _ = self.log_coordinator(&log_msg);

                // Note: Zellij will send a PaneUpdate event after TabUpdate,
                // so we don't need to do anything special here
                true // trigger re-render
            }
            _ => false,
        }
    }
    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        match pipe_message.source {
            PipeSource::Cli(input_id) => {
                if let Some(payload) = pipe_message.payload {
                    return self.handle_incoming_message(&payload, "CLI", Some(input_id));
                }
            }
            PipeSource::Plugin(plugin_id) => {
                if let Some(payload) = pipe_message.payload {
                    return self.handle_incoming_message(
                        &payload,
                        &format!("Plugin-{}", plugin_id),
                        None,
                    );
                } else {
                    self.last_message = Some("Received empty message from plugin".to_string());
                    return true;
                }
            }
            PipeSource::Keybind => {
                if let Some(payload) = pipe_message.payload {
                    return self.handle_incoming_message(&payload, "Keybind", None);
                } else {
                    self.last_message = Some("Received keybind trigger".to_string());
                    return true;
                }
            }
        }
        false
    }
    fn render(&mut self, _rows: usize, _cols: usize) {
        // Create condensed status bar format
        // ZZZ | Phase: Init | Perms: ✓ | Panes: O,C,T,R,E (5/5) | Last: StartPlanning→Overseer | Msgs: 3

        // Format phase
        let phase = match self.current_phase {
            WorkflowPhase::Initializing => "Init",
            WorkflowPhase::PlanningInProgress => "Plan",
            WorkflowPhase::PlanReady => "Plan",
            WorkflowPhase::ImplementationInProgress => "Impl",
            WorkflowPhase::ImplementationComplete => "Impl",
            WorkflowPhase::ReviewInProgress => "Rev",
            WorkflowPhase::ReviewComplete => "Rev",
            WorkflowPhase::Finished => "Done",
        };

        // Format permissions
        let perms = if self.permissions_granted {
            "✓"
        } else {
            "✗"
        };

        // Format pane roles
        let registered_roles = self.get_registered_roles();
        let pane_icons: Vec<String> = registered_roles
            .iter()
            .map(|role| match role {
                PaneRole::Overseer => "O".to_string(),
                PaneRole::Commander => "C".to_string(),
                PaneRole::TaskList => "T".to_string(),
                PaneRole::Review => "R".to_string(),
                PaneRole::Editor => "E".to_string(),
            })
            .collect();
        let panes_display = if pane_icons.is_empty() {
            "None (0/5)".to_string()
        } else {
            format!("{} ({}/5)", pane_icons.join(","), pane_icons.len())
        };

        // Format last message
        let last_msg = if let Some(ref msg) = self.last_message {
            // Extract key info from complex message strings
            if msg.contains("StartPlanning") && msg.contains("→") {
                "StartPlanning→Overseer".to_string()
            } else if msg.contains("Envelope from") && msg.contains("→") {
                // Extract "from source → target: MessageType"
                if let Some(arrow_pos) = msg.find(" → ") {
                    if let Some(colon_pos) = msg[arrow_pos..].find(": ") {
                        let start = arrow_pos + 3;
                        let end = arrow_pos + colon_pos;
                        let target = &msg[start..end];
                        if let Some(msg_start) = msg.find(": ") {
                            if let Some(msg_type) = msg[msg_start + 2..].split('(').next() {
                                format!("{}→{}", msg_type, target)
                            } else {
                                "Message→Target".to_string()
                            }
                        } else {
                            format!("Msg→{}", target)
                        }
                    } else {
                        "Message→Unknown".to_string()
                    }
                } else {
                    "Recent".to_string()
                }
            } else if msg.contains("Broadcast") {
                "Broadcast*All".to_string()
            } else if msg.contains("Raw from") {
                "Raw→Plugin".to_string()
            } else {
                "Recent".to_string()
            }
        } else {
            "None".to_string()
        };

        // Format message count
        let msg_count = self.received_messages.len();

        // Render single-line status bar
        print!(
            "ZZZ | Phase: {} | Perms: {} | Panes: {} | Last: {} | Msgs: {}",
            phase, perms, panes_display, last_msg, msg_count
        );
    }
}
