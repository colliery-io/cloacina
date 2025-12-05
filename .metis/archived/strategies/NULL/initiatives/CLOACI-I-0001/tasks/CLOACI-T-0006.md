---
id: migrate-remaining-dal-modules-to
level: task
title: "Migrate remaining DAL modules to unified implementation"
short_code: "CLOACI-T-0006"
created_at: 2025-11-30T02:05:39.877962+00:00
updated_at: 2025-12-03T23:35:58.043738+00:00
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

# Migrate remaining DAL modules to unified implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Migrate all remaining DAL modules from the separate `postgres_dal/` and `sqlite_dal/` directories to the unified implementation, using patterns established in CLOACI-T-0005.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `unified/pipeline_execution.rs` fully implemented and tested
- [ ] `unified/task_execution.rs` fully implemented and tested
- [ ] `unified/task_execution_metadata.rs` fully implemented and tested
- [ ] `unified/recovery_event.rs` fully implemented and tested
- [ ] `unified/cron_schedule.rs` fully implemented and tested
- [ ] `unified/cron_execution.rs` fully implemented and tested
- [ ] `unified/workflow_packages.rs` fully implemented and tested
- [ ] `unified/workflow_registry.rs` fully implemented and tested
- [ ] `unified/workflow_registry_storage.rs` fully implemented and tested
- [ ] All integration tests pass for both backends
- [ ] No functionality regression from legacy DALs

## Implementation Notes

### Technical Approach

Apply patterns from CLOACI-T-0005 (`ContextDAL`) to each remaining module:

1. **Backend-specific connection handling**
2. **UUID/timestamp generation patterns**
3. **Insert patterns** (`get_result` vs `execute`)
4. **Query type handling** via `match` on `BackendType`

### Module Complexity Assessment

| Module | Complexity | Notes |
|--------|------------|-------|
| `pipeline_execution` | Medium | Multiple status updates, joins with context |
| `task_execution` | High | Complex queries, retry logic, status transitions |
| `task_execution_metadata` | Low | Simple CRUD |
| `recovery_event` | Low | Simple insert/query |
| `cron_schedule` | Medium | Timestamp handling, atomic claim_and_update |
| `cron_execution` | Medium | Foreign key relationships, lost execution detection |
| `workflow_packages` | Low | Simple CRUD with registry relationship |
| `workflow_registry` | Medium | Binary data handling |
| `workflow_registry_storage` | Medium | Trait implementation |

### Recommended Order

1. `recovery_event` (simplest)
2. `task_execution_metadata` (simple)
3. `workflow_packages` (simple)
4. `pipeline_execution` (medium, core functionality)
5. `workflow_registry` (medium)
6. `workflow_registry_storage` (medium)
7. `cron_schedule` (medium, timestamp-heavy)
8. `cron_execution` (medium, relationships)
9. `task_execution` (most complex, save for last)

### Files to Modify

- All files in `cloacina/src/dal/unified/`

### Dependencies

- Requires CLOACI-T-0005 (ContextDAL patterns established)

### Risk Considerations

- Largest task in the initiative by volume
- Consider splitting into sub-tasks if progress stalls
- Test each module individually before moving to next

## Status Updates

*To be added during implementation*
