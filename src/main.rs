use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use zellij_tile::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum WorkflowPhase {
    Initializing,
    PlanningInProgress,
    PlanReady,
    ImplementationInProgress,
    ImplementationComplete,
    ReviewInProgress,
    ReviewComplete,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum PaneRole {
    Overseer,
    Commander,
    TaskList,
    Review,
    Editor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Notification {
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CoordinationMessage {
    // CLI initiates workflow with task details
    StartPlanning {
        task_id: u32,
        task_description: String,
    },

    // Planning phase completion
    PlanReady {
        todo_file_path: String,
    },

    // Implementation phase
    StartImplementation,
    TaskCompleted {
        task_id: String,
    },
    AllTasksComplete,

    // Review phase
    StartReview,
    ReviewComplete {
        review_file_path: String,
    },

    // State management
    PhaseTransition {
        from: WorkflowPhase,
        to: WorkflowPhase,
    },

    // File system events
    FileChanged {
        file_path: String,
        event_type: String,
    },
}

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

register_plugin!(State);

// More info on plugins: https://zellij.dev/documentation/plugins

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // runs once on plugin load, provides the configuration with which this plugin was loaded
        // (if any)
        //
        // this is a good place to `subscribe` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.subscribe.html)
        // to `Event`s (https://docs.rs/zellij-tile/latest/zellij_tile/prelude/enum.Event.html)
        // and `request_permissions` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.request_permission.html)
    }
    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        // react to `Event`s that have been subscribed to (and the plugin has permissions for)
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
    }
    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let mut should_render = false;
        // react to data piped to this plugin from the CLI, a keybinding or another plugin
        // read more about pipes: https://zellij.dev/documentation/plugin-pipes
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
    }
    fn render(&mut self, rows: usize, cols: usize) {}
}
