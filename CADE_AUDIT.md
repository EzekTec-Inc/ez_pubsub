# CADE Audit Log


## 2026-05-19T19:36:01Z — Moved `async-trait` from dev-dependencies to dependencies in Cargo.toml, removed the unused `block_on` import from src/lib.rs tests, and verified with `cargo test --all-targets`.

**Reason:** Resume prior interrupted work by fixing the async trait dependency issue and cleaning the remaining test warning, then verify all targets pass.

**Files modified:**
- M Cargo.toml
- M src/lib.rs

---
