---
id: refactor-feature-flags-to-include
level: task
title: "Refactor feature flags to include-backend model"
short_code: "CLOACI-T-0008"
created_at: 2025-11-30T02:05:40.318656+00:00
updated_at: 2025-11-30T02:05:40.318656+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Refactor feature flags to include-backend model

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Refactor Cargo.toml feature flags from "select exactly one backend" to "include backend(s)", enabling single binaries that support both backends while still allowing single-backend builds for size optimization.

## Acceptance Criteria

- [ ] Default features include both `postgres` and `sqlite`
- [ ] `compile_error!` mutual exclusivity checks removed from `lib.rs`
- [ ] Single-backend builds still work (e.g., `--no-default-features --features postgres`)
- [ ] `cargo build` with default features produces a dual-backend binary
- [ ] `cargo test` runs tests for both backends
- [ ] Downstream crates (Python bindings, examples) updated
- [ ] Documentation updated to reflect new feature model

## Implementation Notes

### Technical Approach

1. **Update `cloacina/Cargo.toml`**

   Before:
   ```toml
   [features]
   default = ["macros"]
   postgres = ["diesel/postgres", "diesel/uuid", "deadpool-diesel/postgres"]
   sqlite = ["diesel/sqlite", "diesel/returning_clauses_for_sqlite_3_35", "deadpool-diesel/sqlite", "libsqlite3-sys/bundled"]
   ```

   After:
   ```toml
   [features]
   default = ["macros", "postgres", "sqlite"]  # Both backends by default
   postgres = ["diesel/postgres", "diesel/uuid", "deadpool-diesel/postgres"]
   sqlite = ["diesel/sqlite", "diesel/returning_clauses_for_sqlite_3_35", "deadpool-diesel/sqlite", "libsqlite3-sys/bundled"]
   postgres-only = ["postgres"]  # Convenience for single-backend
   sqlite-only = ["sqlite"]      # Convenience for single-backend
   ```

2. **Remove `compile_error!` in `lib.rs:430-435`**

   ```rust
   // REMOVE THESE:
   #[cfg(all(feature = "postgres", feature = "sqlite"))]
   compile_error!("Cannot enable both...");
   
   #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
   compile_error!("Must enable exactly one...");
   ```

3. **Update conditional compilation throughout codebase**

   Change patterns like:
   ```rust
   #[cfg(feature = "postgres")]
   // postgres-specific code
   ```
   
   To:
   ```rust
   // Now using unified implementation via AnyConnection
   ```

4. **Update dependent crates**

   - `cloaca/Cargo.toml` (Python bindings)
   - `cloacina-macros/Cargo.toml` if affected
   - Example projects

### Files to Modify

- `cloacina/Cargo.toml`
- `cloacina/src/lib.rs` (remove compile_error!)
- `cloaca/Cargo.toml`
- Any example `Cargo.toml` files

### Dependencies

- Requires CLOACI-T-0006 (All DALs migrated to unified)
- Should be done after unified implementation is complete and tested

### Risk Considerations

- Breaking change for users who relied on feature flag behavior
- Must document migration path in release notes
- Ensure binary size increase is acceptable (document tradeoffs)

## Status Updates

*To be added during implementation*