---
id: create-unified-schema-module-with
level: task
title: "Create unified schema module with MultiConnection support"
short_code: "CLOACI-T-0003"
created_at: 2025-11-30T02:05:39.366703+00:00
updated_at: 2025-11-30T02:05:39.366703+00:00
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

# Create unified schema module with MultiConnection support

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Create a unified schema module that works with Diesel's `MultiConnection`, replacing the current separate `postgres_schema` and `sqlite_schema` modules with conditional compilation.

## Acceptance Criteria

- [ ] Single schema definition that compiles for both backends
- [ ] Schema uses `UniversalUuid`, `UniversalTimestamp`, `UniversalBool` wrapper types
- [ ] All table definitions present: `contexts`, `pipeline_executions`, `task_executions`, `recovery_events`, `task_execution_metadata`, `workflow_registry`, `workflow_packages`, `cron_schedules`, `cron_executions`
- [ ] Join relationships properly defined
- [ ] `allow_tables_to_appear_in_same_query!` macro correctly configured
- [ ] Compiles with both features enabled

## Implementation Notes

### Technical Approach

The key challenge is that PostgreSQL and SQLite use different Diesel types:
- PostgreSQL: `Uuid`, `Timestamp`, `Bool`, `Varchar`
- SQLite: `Binary`, `Text`, `Integer`, `Text`

**Option A: Type Aliases with cfg**
```rust
#[cfg(feature = "postgres")]
type DbUuid = diesel::sql_types::Uuid;
#[cfg(feature = "sqlite")]
type DbUuid = diesel::sql_types::Binary;
```
Problem: Doesn't work when both features enabled.

**Option B: Custom SQL Types**
Define custom SQL types that map to the appropriate backend type at runtime. Diesel's `MultiConnection` should handle this.

**Option C: Separate schemas with runtime selection**
Keep both schema definitions, select at runtime based on backend. Less ideal but may be necessary.

Research needed on how `MultiConnection` handles schema type differences.

### Files to Modify

- `cloacina/src/database/schema.rs` - Complete rewrite
- `cloacina/src/database/universal_types.rs` - May need updates for schema integration

### Dependencies

- Requires CLOACI-T-0001 (AnyConnection enum)
- Research: How does Diesel MultiConnection handle differing schema types?

### Risk Considerations

- Schema type system is fundamental - errors here break everything
- May need to keep both schemas and select at runtime if type unification isn't possible
- `diesel::table!` macro expansion differences between backends

## Status Updates

*To be added during implementation*