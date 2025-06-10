# ZZZ Plugin Implementation Tasks

Based on the plan.md file, here is a comprehensive todo list broken down into discrete, testable tasks.

## Phase 1: Core Infrastructure

### 1. Core Data Structures

- [x] **1.1** Define ZzzState struct with task_id, current_phase, pane_ids, file_watcher, and pending_notifications fields

- [x] **1.2** Implement WorkflowPhase enum with all 8 phases (Initializing → Finished)

- [x] **1.3** Implement PaneRole enum for Overseer, Commander, TaskList, Review, Editor

- [x] **1.4** Create Notification struct for pending notifications queue

- [x] **1.5** Implement CoordinationMessage enum with all message types from the protocol

### 2. File System Operations

- [x] **2.1** Implement directory creation for `.zzz/task-{task_id}/` structure

- [x] **2.2** Create file path utilities for todo-list.md, review.md, plan.md

- [x] **2.3** Implement atomic file read/write operations with error handling

- [x] **2.4** Create log directory structure and logging utilities

- [x] **2.5** Comprehensive testing suite for file system operations (50 tests covering error handling, atomic operations, retry mechanisms, path utilities, logging functions, integration workflows, and edge cases)

### 3. Basic Plugin Communication

- [x] **3.1** Implement pipe_message_to_plugin wrapper for inter-pane communication
  - ✅ Created `src/communication.rs` module with `CommunicationError` and `MessageEnvelope`
  - ✅ Implemented `send_coordination_message()` for targeted messages
  - ✅ Implemented `broadcast_coordination_message()` for broadcast messages
  - ✅ Enhanced `pipe()` method with backward compatibility for envelope and legacy formats
  - ✅ Added comprehensive error handling and logging
  - ✅ Supports raw text, legacy JSON, and modern envelope message formats

- [x] **3.2** Create message routing system to dispatch messages by pane role
  - ✅ Implemented `MessageRouter` struct with pane role registry
  - ✅ Added permission requests for `ReadApplicationState` and `WriteToStdin`
  - ✅ Created role-based message routing with `route_message_to_role()`
  - ✅ Added multi-role messaging with `route_message_to_roles()`
  - ✅ Implemented broadcasting to all registered panes
  - ✅ Added pane name to role pattern matching
  - ✅ Enhanced error handling with simple logging for missing panes
  - ✅ Integrated routing system into main State struct with wrapper methods

- [ ] **3.3** Implement message queue for handling async communication

- [x] **3.4** Add message serialization/deserialization utilities
  - ✅ **Complete**: Message serialization/deserialization implemented as part of 3.1
  - ✅ **Complete**: Real pane discovery implemented using Zellij PaneManifest API

## Phase 2: Workflow Coordination

### 4. Workflow State Machine

- [ ] **4.1** Implement state transition logic for Initialize → Planning

- [ ] **4.2** Implement Planning → Implementation transition with todo-list.md detection

- [ ] **4.3** Implement Implementation → Review transition with completion detection

- [ ] **4.4** Implement Review → Complete transition with review.md processing

- [ ] **4.5** Add state persistence to survive plugin restarts

- [ ] **4.6** Implement state validation and error recovery mechanisms

### 5. Planning Phase Coordination

- [ ] **5.1** Implement StartPlanning message generation and sending to Overseer

- [ ] **5.2** Add todo-list.md file monitoring for creation detection

- [ ] **5.3** Implement todo list parsing to understand task structure

### 6. Implementation Phase Coordination

- [ ] **6.1** Monitor todo-list.md for task completion markers (checkboxes, status updates)

- [ ] **6.2** Track implementation progress and calculate completion percentage

- [ ] **6.3** Detect when all tasks are completed and trigger AllTasksComplete message

### 7. Review Phase Coordination

- [ ] **7.1** Send StartReview message to Overseer when implementation complete

- [ ] **7.2** Monitor review.md creation and parse review content

- [ ] **7.3** Coordinate review implementation cycle with Commander

## Phase 3: Advanced Features

### 8. File System Monitoring

- [ ] **8.1** Implement file watcher using notify crate for .zzz directory

- [ ] **8.2** Add debouncing for rapid file changes to prevent spam

- [ ] **8.3** Handle file system events (create, modify, delete) with proper error handling


### 9. Error Handling

- [ ] **9.1** Implement retry mechanisms for failed pane communications

- [ ] **9.2** Add fallback procedures for unresponsive AI assistants

- [ ] **9.3** Create error reporting and recovery mechanisms

### 10. Configuration Management

- [ ] **10.1** Implement plugin configuration parsing from zellij.kdl

- [ ] **10.2** Add runtime configuration validation and defaults

## Phase 4: Testing Infrastructure

### 11. Test Framework Setup

- [ ] **11.1** Create mock AI assistant responses for testing

- [ ] **11.2** Implement integration test harness for end-to-end workflow

- [ ] **11.3** Add performance and load testing utilities

## Implementation Priority

### High Priority (MVP)

- Tasks 1.1-1.5 (Core Data Structures)
- Tasks 2.1-2.3 (Basic File Operations)
- Tasks 3.1-3.2 (Basic Communication)
- Tasks 4.1-4.4 (Core State Machine)
- Tasks 5.1-5.2 (Basic Planning)
- Tasks 6.1-6.3 (Basic Implementation)

### Medium Priority

- Tasks 7.1-7.3 (Review Coordination)
- Tasks 8.1-8.3 (File Monitoring)
- Tasks 9.1-9.2 (Error Handling)

### Low Priority (Nice to Have)

- Tasks 10.1-10.2 (Advanced Configuration)
- Tasks 11.1-11.3 (Testing Infrastructure)
- Task 9.3 (Advanced Error Recovery)
- Progress tracking features (removed from MVP)

## Dependencies Between Tasks

- Tasks 1.x must be completed before any other tasks
- Tasks 2.1-2.2 required for tasks 5.2, 6.1, 7.2
- Tasks 3.1-3.2 required for tasks 5.1, 6.3, 7.1
- Tasks 4.1-4.2 required for task 5.2
- Tasks 4.2-4.3 required for task 6.3
- Tasks 4.3-4.4 required for task 7.1
- Task 8.1 required for tasks 5.2, 6.1, 7.2

## Testing Strategy

Each task includes both:

1. **Unit Tests**: Test individual functions and components in isolation
2. **Integration Tests**: Test component interactions and file system operations
3. **Manual Tests**: For UI components and end-to-end workflows

Total estimated tasks: **33 discrete implementable features** (progress tracking removed from MVP)
