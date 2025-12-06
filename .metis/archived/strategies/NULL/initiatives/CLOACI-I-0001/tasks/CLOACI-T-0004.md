---
id: create-unified-dal-module-structure
level: task
title: "Create unified DAL module structure"
short_code: "CLOACI-T-0004"
created_at: 2025-11-30T02:05:39.510235+00:00
updated_at: 2025-11-30T02:58:18.462526+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Create unified DAL module structure

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Create the unified DAL module structure that will replace the separate `postgres_dal/` and `sqlite_dal/` directories, establishing the foundation for migrated DAL implementations.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `cloacina/src/dal/unified/` directory created
- [ ] `DAL` struct updated to work with `AnyConnection` and `BackendType`
- [ ] Module structure mirrors existing DAL (context, pipeline_execution, task_execution, etc.)
- [ ] `mod.rs` properly exports unified DAL alongside legacy DALs during transition
- [ ] Helper macros/functions for backend-specific query handling
- [ ] Compiles successfully (can be empty implementations initially)

## Implementation Notes

### Technical Approach

1. Create new directory structure:
   ```
   cloacina/src/dal/
   ├── mod.rs              # Updated to conditionally export
   ├── filesystem_dal/     # Unchanged
   ├── postgres_dal/       # Keep during transition
   ├── sqlite_dal/         # Keep during transition
   └── unified/            # New unified implementation
       ├── mod.rs
       ├── context.rs
       ├── pipeline_execution.rs
       ├── task_execution.rs
       ├── task_execution_metadata.rs
       ├── recovery_event.rs
       ├── cron_schedule.rs
       ├── cron_execution.rs
       ├── workflow_packages.rs
       ├── workflow_registry.rs
       └── workflow_registry_storage.rs
   ```

2. Update `DAL` struct:
   ```rust
   pub struct DAL {
       pub database: Database,
       pub backend_type: BackendType,
   }
   ```

3. Create helper for backend-specific operations:
   ```rust
   macro_rules! backend_match {
       ($conn:expr, $pg_block:block, $sqlite_block:block) => {
           match $conn {
               AnyConnection::Postgres(conn) => $pg_block,
               AnyConnection::Sqlite(conn) => $sqlite_block,
           }
       };
   }
   ```

### Files to Create

- `cloacina/src/dal/unified/mod.rs`
- `cloacina/src/dal/unified/*.rs` (stub files for each DAL)

### Files to Modify

- `cloacina/src/dal/mod.rs` - Add unified module, update exports

### Dependencies

- Requires CLOACI-T-0001 (AnyConnection enum)
- Requires CLOACI-T-0002 (Database struct updates)

### Risk Considerations

- Must maintain backward compatibility during transition
- Need clear migration path from legacy DALs to unified

## Status Updates

*To be added during implementation*
