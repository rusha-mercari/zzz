mod coordination_message;
mod file_system;
mod notification;
mod pane_role;
mod workflow_phase;

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
}

impl Default for State {
    fn default() -> Self {
        Self {
            task_id: 0,
            current_phase: WorkflowPhase::Initializing,
            pane_ids: HashMap::new(),
            file_watcher: None,
            pending_notifications: Vec::new(),
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
    fn pipe(&mut self, _pipe_message: PipeMessage) -> bool {
        let should_render = false;
        // react to data piped to this plugin from the CLI, a keybinding or another plugin
        // read more about pipes: https://zellij.dev/documentation/plugin-pipes
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
    }
    fn render(&mut self, _rows: usize, _cols: usize) {}
}
