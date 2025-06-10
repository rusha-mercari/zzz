use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowPhase {
    Initializing,
    PlanningInProgress,
    PlanReady,
    ImplementationInProgress,
    ImplementationComplete,
    ReviewInProgress,
    ReviewComplete,
    Finished,
}
