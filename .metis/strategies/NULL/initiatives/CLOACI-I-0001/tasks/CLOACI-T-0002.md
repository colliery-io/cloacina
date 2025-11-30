---
id: update-database-struct-for-runtime
level: task
title: "Update Database struct for runtime backend detection"
short_code: "CLOACI-T-0002"
created_at: 2025-11-30T02:05:39.227411+00:00
updated_at: 2025-11-30T02:41:07.719123+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Update Database struct for runtime backend detection

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Refactor the `Database` struct to detect the backend type at runtime from the connection URL and use the appropriate connection pool type via `AnyConnection`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Database` struct updated to work with `AnyConnection`
- [ ] Backend detection from URL scheme (`postgres://`, `postgresql://` -> Postgres; `sqlite://`, file path -> SQLite)
- [ ] Connection pool management works for both backends
- [ ] `get_connection()` returns connections compatible with `AnyConnection`
- [ ] Schema support (`get_connection_with_schema()`) works for PostgreSQL, no-op for SQLite
- [ ] Existing public API preserved where possible

## Implementation Notes

### Technical Approach

1. Add `BackendType` enum to track detected backend:
   ```rust
   pub enum BackendType {
       Postgres,
       Sqlite,
   }
   ```

2. Update `Database::new()` to detect backend from URL:
   ```rust
   fn detect_backend(url: &str) -> BackendType {
       if url.starts_with("postgres://") || url.starts_with("postgresql://") {
           BackendType::Postgres
       } else {
           BackendType::Sqlite
       }
   }
   ```

3. Handle `deadpool-diesel` pool management:
   - May need an enum wrapper for the pool similar to `AnyConnection`
   - Or use trait objects for pool abstraction

4. Update connection acquisition methods to return `AnyConnection`

### Files to Modify

- `cloacina/src/database/connection.rs` - Main changes
- `cloacina/src/database/mod.rs` - Update exports

### Dependencies

- Requires CLOACI-T-0001 (AnyConnection enum)

### Risk Considerations

- `deadpool-diesel` has separate `postgres::Pool` and `sqlite::Pool` types
- May need custom pool wrapper or enum
- Connection lifetimes with `AnyConnection` need careful handling

## Status Updates

*To be added during implementation*