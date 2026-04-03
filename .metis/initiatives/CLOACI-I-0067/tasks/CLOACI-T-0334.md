---
id: t3-task-scheduler-unit-tests
level: task
title: "T3: Task scheduler unit tests"
short_code: "CLOACI-T-0334"
created_at: 2026-04-03T02:36:31.774103+00:00
updated_at: 2026-04-03T10:35:47.684793+00:00
parent: CLOACI-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0067
---

# T3: Task scheduler unit tests

## Parent Initiative
[[CLOACI-I-0067]] — Tier 2 (core engine)

## Objective
Add unit tests for task_scheduler/ modules — scheduler_loop.rs (321 lines), state_manager.rs (322 lines), context_manager.rs (241 lines), recovery.rs (386 lines). Currently zero #[test] blocks.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] TriggerRule: serialize/deserialize roundtrip for Always, All, Any, None
- [ ] TriggerCondition: serialize/deserialize for TaskSuccess, TaskFailed, TaskSkipped, ContextValue
- [ ] ValueOperator: serialize/deserialize for all 8 variants
- [ ] evaluate_context_condition: Exists/NotExists on present and missing keys
- [ ] evaluate_context_condition: Equals/NotEquals for strings, numbers, booleans
- [ ] evaluate_context_condition: GreaterThan/LessThan for numbers, non-numbers return false
- [ ] evaluate_context_condition: Contains/NotContains for strings and arrays
- [ ] Integration tests for state_manager, scheduler_loop, recovery (against real DB)

## Source Files
- crates/cloacina/src/task_scheduler/trigger_rules.rs
- crates/cloacina/src/task_scheduler/context_manager.rs
- crates/cloacina/src/task_scheduler/state_manager.rs
- crates/cloacina/src/task_scheduler/scheduler_loop.rs
- crates/cloacina/src/task_scheduler/recovery.rs

## Implementation Notes

- trigger_rules.rs and evaluate_context_condition() are pure logic — unit tests, no DB
- state_manager, scheduler_loop, recovery are all DAL-dependent — need integration tests using the existing TestFixture pattern from crates/cloacina/tests/fixtures.rs
- Tests go in the source files as #[cfg(test)] modules (unit) and crates/cloacina/tests/ (integration)

## Status Updates

### 2026-04-03 — Unit tests complete (37 tests, all passing)

**Tests added:**
- `trigger_rules.rs` (14 tests): serialization roundtrips for TriggerRule (Always, All, Any, None including empty conditions), TriggerCondition (TaskSuccess, TaskFailed, TaskSkipped, ContextValue), ValueOperator (all 8 variants), deserialization from JSON literals
- `context_manager.rs` (23 tests): evaluate_context_condition() covering all 8 ValueOperator variants — Exists/NotExists (present/missing keys), Equals/NotEquals (strings, numbers, booleans, missing keys), GreaterThan/LessThan (numbers, floats, non-numbers, missing keys), Contains/NotContains (string substrings, array elements, non-string/array types)

**Not covered (DB-dependent, deferred):**
- state_manager.rs: update_pipeline_task_readiness, check_task_dependencies, evaluate_trigger_rules — all require DAL + workflow registry
- scheduler_loop.rs: process_active_pipelines, dispatch_ready_tasks, complete_pipeline — all require DAL + Dispatcher
- recovery.rs: recover_orphaned_tasks and sub-functions — all require DAL
These would need integration tests using the TestFixture pattern, similar to existing tests in crates/cloacina/tests/integration/
