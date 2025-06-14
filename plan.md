# ZZZ Plugin Implementation Plan

## Overview

The ZZZ plugin will coordinate communication between multiple AI assistants in a Zellij layout designed for collaborative development. The plugin acts as a central coordinator that manages the workflow between different panes:

- **Overseer**: OpenAI Codex for high-level planning and code review
- **Commander**: Claude Code for implementation
- **Task List**: View current todo list
- **Review**: View review comments and suggestions

## Architecture

### Core State Management

```rust
struct State {
    task_id: u32,
    current_phase: WorkflowPhase,
    pane_ids: HashMap<PaneRole, PaneId>,
    file_watcher: Option<Box<dyn Watcher>>,
    pending_notifications: Vec<Notification>,
}

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

enum PaneRole {
    Overseer,
    Commander,
    TaskList,
    Review,
    Editor,
}
```

### Key Components

#### 1. File System Monitoring

- Monitor `.zzz/task-{task_id}/todo-list.md` for changes
- Monitor `.zzz/task-{task_id}/review.md` for creation/changes
- Use filesystem events to trigger workflow transitions

#### 2. Inter-Pane Communication

- Use `pipe_message_to_plugin` API to send messages between panes
- Implement message types for coordination:
  - `StartPlanning` → Overseer
  - `PlanReady` → Commander
  - `StartImplementation` → Commander
  - `ImplementationComplete` → Overseer
  - `StartReview` → Overseer
  - `ReviewReady` → Commander

#### 3. Workflow State Machine

```
Initialize → Planning → Implementation → Review → Complete
     ↓         ↓            ↓           ↓        ↓
  Setup    Generate     Implement    Review   Apply
   dirs    todo-list     features    code     fixes
```

## Implementation Steps

### Phase 1: Core Infrastructure

1. **State Management**

   - Define core state structures
   - Implement workflow phase transitions
   - Add task_id configuration handling

2. **File System Integration**

   - Implement directory creation (`.zzz/task-{task_id}/`)
   - Add file monitoring for todo-list.md and review.md
   - Handle file creation, modification, and completion detection

3. **Basic Plugin Communication**
   - Implement pipe message handling
   - Define message protocol for inter-pane communication
   - Add basic message routing

### Phase 2: Workflow Coordination

1. **Planning Phase**

   - Detect when Overseer creates todo-list.md
   - Parse todo list to understand task structure
   - Signal Commander to start implementation planning

2. **Implementation Phase**

   - Monitor todo-list.md for completion markers
   - Track implementation progress
   - Detect when all items are completed

3. **Review Phase**
   - Trigger Overseer to start code review
   - Monitor review.md creation
   - Coordinate review implementation cycle

### Phase 3: Advanced Features

1. **Error Handling**

   - Retry mechanisms for failed communications
   - Fallback procedures
   - Error reporting and recovery

2. **Configuration**
   - Customizable workflow steps
   - Configurable file paths
   - AI assistant preferences

## File Structure

```
.zzz/
└── task-{task_id}/
    ├── todo-list.md      # Generated by Overseer
    ├── review.md         # Generated by Overseer
    ├── plan.md           # Generated by Commander
    └── logs/
        ├── overseer.log
        ├── commander.log
        └── coordinator.log
```

## Message Protocol

### Message Types

```rust
#[derive(Serialize, Deserialize)]
enum CoordinationMessage {
    StartPlanning {
        task_id: u32,
        feature_description: String,
    },
    PlanReady {
        task_id: u32,
        todo_file_path: String,
    },
    StartImplementation {
        task_id: u32,
        plan_file_path: String,
    },
    TaskCompleted {
        task_id: u32,
        task_index: usize,
    },
    AllTasksComplete {
        task_id: u32,
    },
    StartReview {
        task_id: u32,
    },
    ReviewReady {
        task_id: u32,
        review_file_path: String,
    },
    ApplyReviewSuggestions {
        task_id: u32,
    },
}
```

### Communication Flow

1. **Initialization**: Plugin receives task_id and feature description via configuration
2. **Planning**: Send `StartPlanning` to Overseer pane
3. **Implementation**: Monitor todo-list.md, send `PlanReady` to Commander
4. **Progress Tracking**: Monitor todo-list.md for completion markers
5. **Review**: Send `AllTasksComplete` to Overseer when done
6. **Review Implementation**: Send `ReviewReady` to Commander
7. **Completion**: Monitor review completion and finalize

## UI Components

### Main Display

- Current workflow phase
- Progress indicators
- Active task information
- Status messages

### Debug Panel (Optional)

- Message log
- File system events
- Pane communication status

## Configuration

### Plugin Configuration

```kdl
pane {
    plugin location="file:target/wasm32-wasi/debug/zzz.wasm" {
        task_id "123"
        feature_description "Implement user authentication system"
        base_directory ".zzz"
        overseer_pane "Overseer"
        commander_pane "Commander"
        enable_logging "true"
    }
}
```

### Integration with zzz() Function

The existing `zzz()` function will be enhanced to:

1. Create the `.zzz/task-{task_id}/` directory structure
2. Pass task_id and feature_description to the plugin
3. Ensure proper pane naming for communication

## Dependencies

### Additional Crates

```toml
[dependencies]
zellij-tile = "0.41.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
notify = "6.0"  # File system watching
tokio = { version = "1.0", features = ["fs", "time"] }
uuid = "1.0"    # For unique message IDs
```

## Risk Mitigation

### Potential Issues

1. **File System Race Conditions**: Use proper file locking and retry mechanisms
2. **Pane Communication Failures**: Implement timeout and retry logic
3. **AI Assistant Unresponsiveness**: Add watchdog timers and manual override
4. **State Synchronization**: Use atomic file operations and state validation

### Fallback Mechanisms

1. Manual workflow progression via plugin UI
2. File-based state recovery on restart
3. Emergency reset functionality
4. Detailed logging for debugging

## Future Enhancements

### Phase 4: Advanced Features

1. **Multi-Language Support**: Support for different programming languages
2. **Custom Workflows**: User-defined workflow configurations
3. **Performance Metrics**: Detailed analytics and reporting
4. **AI Assistant Plugins**: Pluggable AI assistant interfaces
5. **Collaboration Features**: Multi-user coordination
6. **Version Control Integration**: Git workflow automation

This plan provides a comprehensive roadmap for implementing the ZZZ plugin that will effectively coordinate multiple AI assistants in a collaborative development environment.
