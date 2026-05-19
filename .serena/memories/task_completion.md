Task Completion Workflow:
1. Ensure the code compiles successfully: `cargo build`
2. Format the code: `cargo fmt`
3. Run the linter and fix any warnings: `cargo clippy`
4. Run all unit and integration tests: `cargo test`
5. If modifying the TUI demo, manually test `cargo run` to verify terminal rendering and input handling behave as expected.