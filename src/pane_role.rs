use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneRole {
    Overseer,
    Commander,
    TaskList,
    Review,
    Editor,
}