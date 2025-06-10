use zellij_tile::prelude::*;

/// Trait for abstracting Zellij API calls to enable testing
pub trait ZellijService {
    /// Write characters to a specific pane
    fn write_chars_to_pane_id(&self, message: &str, pane_id: PaneId);

    /// Send a pipe message to a plugin by name
    fn pipe_message_to_plugin(&self, message: &str, target: &str);
}

/// Production implementation that calls real Zellij APIs  
pub struct ZellijServiceImpl;

impl ZellijService for ZellijServiceImpl {
    fn write_chars_to_pane_id(&self, message: &str, pane_id: PaneId) {
        zellij_tile::prelude::write_chars_to_pane_id(message, pane_id);
    }

    fn pipe_message_to_plugin(&self, message: &str, target: &str) {
        let message_to_plugin =
            zellij_tile::prelude::MessageToPlugin::new(target).with_payload(message);
        zellij_tile::prelude::pipe_message_to_plugin(message_to_plugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock implementation for testing that captures all API calls
    pub struct MockZellijService {
        pub sent_messages: RefCell<Vec<(String, PaneId)>>,
        pub piped_messages: RefCell<Vec<(String, String)>>,
    }

    impl MockZellijService {
        pub fn new() -> Self {
            Self {
                sent_messages: RefCell::new(Vec::new()),
                piped_messages: RefCell::new(Vec::new()),
            }
        }

        /// Get all messages sent to panes
        pub fn get_sent_messages(&self) -> Vec<(String, PaneId)> {
            self.sent_messages.borrow().clone()
        }

        /// Get all messages piped to plugins
        pub fn get_piped_messages(&self) -> Vec<(String, String)> {
            self.piped_messages.borrow().clone()
        }

        /// Clear all recorded messages
        pub fn clear(&self) {
            self.sent_messages.borrow_mut().clear();
            self.piped_messages.borrow_mut().clear();
        }
    }

    impl super::ZellijService for MockZellijService {
        fn write_chars_to_pane_id(&self, message: &str, pane_id: PaneId) {
            self.sent_messages
                .borrow_mut()
                .push((message.to_string(), pane_id));
        }

        fn pipe_message_to_plugin(&self, message: &str, target: &str) {
            self.piped_messages
                .borrow_mut()
                .push((message.to_string(), target.to_string()));
        }
    }
}

// Re-export MockZellijService for use in other test modules
#[cfg(test)]
pub use tests::MockZellijService;
