Code Style & Conventions:
- Standard Rust formatting applies. All code should pass `cargo fmt`.
- Follow idiomatic Rust naming conventions: `snake_case` for variables, functions, and modules; `PascalCase` for structs, enums, and traits; `SCREAMING_SNAKE_CASE` for statics and constants.
- Include doc comments (`///`) for public APIs to explain usage.
- Error handling should be idiomatic, avoiding excessive `unwrap()` where errors should be handled gracefully (though the current `lib.rs` demo uses `unwrap()` on locks, production code might require more careful poisoning handling).
- TUI components (via `ratatui`) are structured around a central `App` state that handles events and drawing frames.