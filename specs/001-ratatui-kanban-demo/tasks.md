---
description: "Task list template for feature implementation"
---

# Tasks: Ratatui Kanban Demo

**Input**: Design documents from `/specs/001-ratatui-kanban-demo/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create `examples/kanban_board.rs` file structure

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T002 Implement data models (`Ticket`, `Column`, `KanbanState`) in `examples/kanban_board.rs`
- [ ] T003 Configure `PubSub` instance mappings for specific event types (`ui_input`, `action_move_ticket`, `state_updated`)

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Event-Driven State Manager (Priority: P1) 🎯 MVP

**Goal**: Implement an isolated state manager that subscribes to action events and publishes state updates.

**Independent Test**: Can instantiate the state manager, publish an action event, and observe a `state_updated` broadcast.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T004 [US1] Write failing test for KanbanState manager (publish move event -> assert updated state)

### Implementation for User Story 1

- [ ] T005 [US1] Implement `KanbanState` manager async loop holding internal state
- [ ] T006 [US1] Subscribe state manager to `action_move_ticket` via `PubSub`
- [ ] T007 [US1] Publish `state_updated` via `PubSub` on state changes

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Terminal UI Rendering (Priority: P2)

**Goal**: Draw a 3-column Kanban board using `ratatui` that updates automatically upon receiving state updates.

**Independent Test**: Can trigger a render loop by manually broadcasting `state_updated` via `PubSub`.

### Tests for User Story 2 ⚠️

- [ ] T008 [US2] Write failing test to assert the UI struct correctly updates its internal state when `state_updated` is broadcasted.

### Implementation for User Story 2

- [ ] T009 [US2] Implement Ratatui boilerplate (terminal setup/teardown) in `examples/kanban_board.rs`
- [ ] T010 [US2] Implement UI rendering of 3 columns (`Todo`, `InProgress`, `Done`) from `KanbanState`
- [ ] T011 [US2] Subscribe UI task to `state_updated` event to trigger continuous redraws

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Keyboard Input Dispatcher (Priority: P3)

**Goal**: Read physical keyboard inputs and translate them into PubSub events without blocking the async runtime.

**Independent Test**: Pressing keyboard keys results in `action_move_ticket` or `ui_input` events being published to the bus.

### Tests for User Story 3 ⚠️

- [ ] T012 [US3] Write failing test (mocked terminal input) validating that keys result in proper PubSub broadcasts.

### Implementation for User Story 3

- [ ] T013 [US3] Implement `crossterm::event::read()` loop inside a dedicated blocking tokio task
- [ ] T014 [US3] Map keys (Left/Right, 'a', 'q') to `PubSub` broadcasts (`action_move_ticket`, `ui_input`)

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T015 Run `quickstart.md` validation by verifying `cargo run --example kanban_board` works flawlessly
- [ ] T016 Verify concurrent execution works correctly per recent `ez_pubsub` API updates

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed sequentially in priority order (P1 → P2 → P3) or partially in parallel.
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2)
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) 
- **User Story 3 (P3)**: Can start after Foundational (Phase 2)

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Logic/State before UI
- Event broadcasting before handling

### Implementation Strategy

#### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently
3. Add User Story 2 → Test independently 
4. Add User Story 3 → Test independently
5. Final Demo Verification

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Verify tests fail before implementing
- Commit after each task or logical group
