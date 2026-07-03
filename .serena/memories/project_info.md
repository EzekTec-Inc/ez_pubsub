Project Name: ez_pubsub

Purpose: 
A lightweight, thread-safe publish-subscribe event bus library for Rust. The project provides an instance API (`PubSub<T>`) for typed events and a global bus accessible via macros (`subscribe!`, `broadcast!`, `unsubscribe!`). The main file includes a comprehensive demo, including a terminal user interface (TUI) counter application built using `ratatui`.

Structure:
- `Cargo.toml`: Defines dependencies (`ratatui`, `crossterm`).
- `src/lib.rs`: The core pub/sub logic, including the `PubSub` struct, `SubOption` enum, and global bus macros.
