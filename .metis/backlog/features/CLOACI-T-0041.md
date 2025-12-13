---
id: compile-time-database-backend
level: task
title: "Compile-time database backend selection for smaller binaries"
short_code: "CLOACI-T-0041"
created_at: 2025-12-13T00:27:19.096088+00:00
updated_at: 2025-12-13T00:27:19.096088+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Compile-time database backend selection for smaller binaries

## Objective

Enable users to compile cloacina with only the database backend they need, reducing binary size and dependencies while maintaining the current default behavior of supporting both backends for runtime selection.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [ ] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Smaller binaries for deployment, reduced attack surface, faster compile times when only one backend is needed
- **Business Value**: Better developer experience, more flexible deployment options
- **Effort Estimate**: M (Medium) - Significant conditional compilation changes but architecture supports it

## Current State

Default features compile both SQLite and PostgreSQL:
```toml
[features]
default = ["macros", "postgres", "sqlite"]
postgres = ["diesel/postgres", "diesel/uuid", "deadpool-diesel/postgres"]
sqlite = ["diesel/sqlite", "diesel/returning_clauses_for_sqlite_3_35", "deadpool-diesel/sqlite", "libsqlite3-sys/bundled"]
```

The codebase uses runtime backend selection via:
- `AnyPool` enum wrapping `PgPool` or `SqlitePool`
- `AnyConnection` enum for Diesel MultiConnection
- `BackendType` enum for URL-based detection
- DAL methods with `match backend { Postgres => ..., Sqlite => ... }` dispatch

## Acceptance Criteria

- [ ] `cargo build -p cloacina --no-default-features --features postgres` compiles with only PostgreSQL
- [ ] `cargo build -p cloacina --no-default-features --features sqlite` compiles with only SQLite
- [ ] `cargo build -p cloacina` (default) compiles both backends (current behavior preserved)
- [ ] Binary size reduction of ~30-50% when single backend compiled
- [ ] All tests pass for each compilation mode
- [ ] CI matrix tests all three modes: postgres-only, sqlite-only, both
- [ ] Documentation updated with feature flag usage

## CI Build Matrix Requirements

Each feature combination must have dedicated CI jobs:

| Mode | Features | Build Test | Unit Tests | Integration Tests |
|------|----------|------------|------------|-------------------|
| PostgreSQL only | `--no-default-features --features postgres` | Required | Required | Required (postgres) |
| SQLite only | `--no-default-features --features sqlite` | Required | Required | Required (sqlite) |
| Both (default) | default features | Required | Required | Required (both) |

### CI Workflow Changes
- Add build matrix in `cloacina.yml` for feature combinations
- Each matrix entry runs: `cargo build`, `cargo test --lib`, integration tests
- Fail fast disabled to catch all feature-specific issues
- Binary size reporting for each build mode (optional but useful)

## Implementation Notes

### Technical Approach

1. **Feature Flag Refactoring** (Cargo.toml)
   - Keep current defaults for backward compatibility
   - Ensure postgres/sqlite features work independently
   - Add `dual-backend` convenience feature for explicit both-backends

2. **Conditional Type Definitions** (database/connection/)
   - `AnyPool`: When both enabled, use enum; when single, use type alias
   - `AnyConnection`: Same pattern
   - `BackendType`: Compile out unused variant

3. **Conditional DAL Methods** (dal/unified/)
   - Use `#[cfg(feature = "postgres")]` for postgres-specific code
   - Use `#[cfg(feature = "sqlite")]` for sqlite-specific code
   - Remove runtime dispatch when single backend

4. **Schema Consolidation** (database/schema.rs)
   - Conditionally compile appropriate schema
   - Keep unified types (DbUuid, etc.) for both

5. **Migration Embedding**
   - Only embed migrations for enabled backend(s)
   - `POSTGRES_MIGRATIONS` behind postgres feature
   - `SQLITE_MIGRATIONS` behind sqlite feature

### Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Feature flag restructuring |
| `database/connection/mod.rs` | Conditional pool/connection types |
| `database/connection/backend.rs` | Conditional BackendType |
| `database/mod.rs` | Conditional migration exports |
| `dal/unified/*.rs` | Conditional backend dispatch |
| `database/schema.rs` | Conditional schema definitions |

### Risk Considerations
- Breaking change if users rely on both backends being available
- CI complexity increases (3 build modes instead of 1)
- Potential for feature flag bugs if not thoroughly tested

## Status Updates

*To be added during implementation*
