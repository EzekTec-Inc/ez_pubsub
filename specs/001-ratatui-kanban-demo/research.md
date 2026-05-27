# Phase 0: Research

## Ratatui + Tokio Event Loop
- **Decision**: Use `tokio::sync::mpsc` alongside `crossterm::event::read()` mapped into an async task, converting physical key presses into `ez_pubsub` broadcasts.
- **Rationale**: `crossterm` event polling is blocking. It needs to be spawned in a dedicated blocking thread that sends events to the async runtime, which then broadcasts via `PubSub`.
- **Alternatives considered**: Using synchronous `ez_pubsub`, but we recently enforced an `async` only API (`AsyncPubSub`).

## Backend State Separation
- **Decision**: Keep the Kanban `State` struct wrapped in an `Arc<RwLock<State>>` exclusively owned by a background task/subscriber.
- **Rationale**: Demonstrates how pub/sub decouples data owners from UI renderers.
- **Alternatives considered**: Storing state directly in the Ratatui App struct. Rejected because it bypasses the event-driven requirement.