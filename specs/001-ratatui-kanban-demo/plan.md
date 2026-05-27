# Implementation Plan: Ratatui Kanban Demo

**Branch**: `001-ratatui-kanban-demo` | **Date**: 2026-05-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-ratatui-kanban-demo/spec.md`

## Summary

Build a terminal-based Kanban board example using `ratatui` that completely decouples UI rendering from state management using the new `AsyncPubSub` features of `ez_pubsub`.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: `ez_pubsub`, `tokio`, `ratatui`, `crossterm`
**Storage**: N/A (In-memory)
**Testing**: `cargo check --examples`, `cargo run --example kanban_board`
**Target Platform**: Linux/macOS/Windows Terminal
**Project Type**: Example binary (`examples/`)
**Performance Goals**: Responsive 60FPS UI rendering without blocking the async runtime
**Constraints**: Must strictly use `ez_pubsub` for event dispatching between input, state updates, and rendering.
**Scale/Scope**: ~300 LOC single example file

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*
- **Library-First**: N/A (This is a demo showing how to use the existing library).
- **Test-First**: Will ensure example compiles.
- **Async Execution**: Meets requirements of new library design.

## Project Structure

### Documentation (this feature)

```text
specs/001-ratatui-kanban-demo/
├── plan.md              
├── research.md          
├── data-model.md        
├── quickstart.md        
└── tasks.md             
```

### Source Code (repository root)

```text
examples/
└── kanban_board.rs
```

**Structure Decision**: A single new file in the `examples/` directory to showcase usage.