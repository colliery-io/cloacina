---
id: create-anyconnection
level: task
title: "Create AnyConnection MultiConnection enum"
short_code: "CLOACI-T-0001"
created_at: 2025-11-30T02:05:39.093013+00:00
updated_at: 2025-11-30T02:16:05.903237+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Create AnyConnection MultiConnection enum

## Parent Initiative

[[CLOACI-I-0001]] - Migrate to Runtime Database Backend Selection

## Objective

Create a Diesel `MultiConnection` enum that wraps both `PgConnection` and `SqliteConnection`, enabling runtime database backend selection based on connection string URL scheme.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AnyConnection` enum created in `cloacina/src/database/connection.rs`
- [ ] Derives `diesel::MultiConnection` macro
- [ ] Wraps `PgConnection` and `SqliteConnection` variants
- [ ] Compiles successfully with both `postgres` and `sqlite` features enabled simultaneously
- [ ] Basic connection establishment works for both backends
- [ ] URL scheme detection implemented (`postgres://` -> Postgres, `sqlite://` or file path -> SQLite)

## Implementation Notes

### Technical Approach

1. Update `Cargo.toml` to allow both features simultaneously (temporary - will refactor in CLOACI-T-0008)
2. Create the `AnyConnection` enum:
   ```rust
   #[derive(diesel::MultiConnection)]
   pub enum AnyConnection {
       Postgres(diesel::PgConnection),
       Sqlite(diesel::SqliteConnection),
   }
   ```
3. Add URL scheme detection helper function
4. Temporarily maintain backward compatibility with existing code

### Files to Modify

- `cloacina/Cargo.toml` - Update feature configuration
- `cloacina/src/database/connection.rs` - Add `AnyConnection` enum
- `cloacina/src/lib.rs` - Remove/modify `compile_error!` macros (lines 430-435)

### Dependencies

- Diesel 2.1+ with `MultiConnection` derive support (already satisfied)
- Both `diesel/postgres` and `diesel/sqlite` features must be enabled

### Risk Considerations

- Breaking change to existing API until migration is complete
- Connection pooling with `deadpool-diesel` may need adaptation for `AnyConnection`
- Must verify `MultiConnection` works with all Diesel query patterns used in DAL

## Status Updates

*To be added during implementation*