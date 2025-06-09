mod communication;
mod coordination_message;
mod file_system;
mod notification;
mod pane_role;
mod workflow_phase;

use communication::{Communication, CommunicationError, MessageEnvelope, ParsedMessage};
use coordination_message::CoordinationMessage;
use file_system::{FileSystem, FileSystemError};
use notification::Notification;
use notify::Watcher;
use pane_role::PaneRole;
use std::collections::{BTreeMap, HashMap};
use workflow_phase::WorkflowPhase;
use zellij_tile::prelude::*;

struct State {
    task_id: u32,
    current_phase: WorkflowPhase,
    pane_ids: HashMap<PaneRole, PaneId>,
    file_watcher: Option<Box<dyn Watcher>>,
    pending_notifications: Vec<Notification>,
    received_messages: Vec<CoordinationMessage>,
    last_message: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            task_id: 0,
            current_phase: WorkflowPhase::Initializing,
            pane_ids: HashMap::new(),
            file_watcher: None,
            pending_notifications: Vec::new(),
            received_messages: Vec::new(),
            last_message: None,
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
        let envelope = MessageEnvelope::new_targeted(
            message.clone(),
            target_pane_title,
            "zzz-coordinator",
        );

        // Log the outgoing message
        let log_msg = format!(
            "Sending message to '{}': {:?}",
            target_pane_title, message
        );
        let _ = self.log_coordinator(&log_msg);

        // Send the message
        match Communication::send_pipe_message(&envelope) {
            Ok(()) => {
                let success_msg = format!(
                    "Successfully sent message to '{}': {:?}",
                    target_pane_title, message
                );
                let _ = self.log_coordinator(&success_msg);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to send message to '{}': {}",
                    target_pane_title, e
                );
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
        match Communication::send_pipe_message(&envelope) {
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
    fn handle_incoming_message(&mut self, payload: &str, source: &str, _input_id: Option<String>) -> bool {
        // Try to parse the payload using the new parsing logic
        match Communication::parse_incoming_message(payload) {
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
}


register_plugin!(State);

// More info on plugins: https://zellij.dev/documentation/plugins

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        // runs once on plugin load, provides the configuration with which this plugin was loaded
        // (if any)
        //
        // this is a good place to `subscribe` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.subscribe.html)
        // to `Event`s (https://docs.rs/zellij-tile/latest/zellij_tile/prelude/enum.Event.html)
        // and `request_permissions` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.request_permission.html)
    }
    fn update(&mut self, _event: Event) -> bool {
        let should_render = false;
        // react to `Event`s that have been subscribed to (and the plugin has permissions for)
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
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
                    return self.handle_incoming_message(&payload, &format!("Plugin-{}", plugin_id), None);
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
        // Display plugin header
        println!("┌─ ZZZ Plugin ─┐");
        println!("│ Phase: {:?}", self.current_phase);
        println!("│ Task ID: {}", self.task_id);
        
        // Display last received message
        if let Some(ref message) = self.last_message {
            println!("│");
            println!("│ Last Message:");
            println!("│ {}", message);
        } else {
            println!("│ Waiting for messages...");
        }
        
        // Display message count
        println!("│ Total messages: {}", self.received_messages.len());
        
        // Display recent coordination messages (last 3)
        if !self.received_messages.is_empty() {
            println!("│");
            println!("│ Recent Messages:");
            for (i, msg) in self.received_messages.iter().rev().take(3).enumerate() {
                match msg {
                    CoordinationMessage::StartPlanning { task_id, task_description } => {
                        println!("│ {}: StartPlanning({}): {}", 
                                self.received_messages.len() - i, task_id, task_description);
                    }
                    _ => {
                        println!("│ {}: {:?}", self.received_messages.len() - i, msg);
                    }
                }
            }
        }
        
        println!("└─────────────┘");
        
        // Display instructions
        println!();
        println!("=== Communication Examples ===");
        println!();
        println!("1. Raw text message:");
        println!("zellij pipe --plugin file:target/wasm32-wasip1/debug/zzz.wasm --name test -- 'Hello'");
        println!();
        println!("2. Legacy format:");
        println!(r#"zellij pipe --plugin file:target/wasm32-wasip1/debug/zzz.wasm --name coordination -- '{{"StartPlanning":{{"task_id":123,"task_description":"Legacy test"}}}}'"#);
        println!();
        println!("3. Modern envelope (targeted):");
        println!(r#"zellij pipe --plugin file:target/wasm32-wasip1/debug/zzz.wasm --name coordination -- '{{"target_pane":"Overseer","coordination_message":{{"StartPlanning":{{"task_id":123,"task_description":"Targeted test"}}}},"sender":"cli","timestamp":1234567890}}'"#);
        println!();
        println!("4. Modern envelope (broadcast):");
        println!(r#"zellij pipe --plugin file:target/wasm32-wasip1/debug/zzz.wasm --name coordination -- '{{"target_pane":null,"coordination_message":{{"PhaseTransition":{{"from":"Initializing","to":"PlanningInProgress"}}}},"sender":"cli","timestamp":1234567890}}'"#);
    }
}
