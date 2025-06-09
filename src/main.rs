mod coordination_message;
mod notification;
mod pane_role;
mod workflow_phase;

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
