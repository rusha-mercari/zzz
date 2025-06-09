use serde::{Deserialize, Serialize};
use crate::workflow_phase::WorkflowPhase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationMessage {
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