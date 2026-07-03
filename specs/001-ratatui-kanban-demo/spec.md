# Ratatui Kanban Board Demo Spec

## Overview
A new demo application showcasing the `ez_pubsub` library. The demo will implement a Kanban board in the terminal using the `ratatui` crate. It will use the async pub/sub system to decouple UI events (like moving a ticket) from backend state management.

## Requirements
1. **Terminal UI**: Use `ratatui` to draw three columns: "TODO", "IN PROGRESS", "DONE".
2. **Event-Driven Architecture**: 
   - Moving a ticket left/right, creating a ticket, or deleting a ticket must be dispatched as events via `ez_pubsub`.
   - A background state manager subscribes to these events, updates the kanban state, and publishes a "state updated" event.
   - The UI thread subscribes to the "state updated" event to re-render.
3. **Async PubSub usage**: Must use the `AsyncPubSub` trait features recently refactored (concurrent `publish`, `subscribe`).
4. **Keyboard Input**: Basic keybindings (e.g., arrow keys to move focused ticket, 'a' to add, 'q' to quit).