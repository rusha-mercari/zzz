use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneRole {
    Overseer,
    Commander,
    TaskList,
    Review,
    Editor,
}
