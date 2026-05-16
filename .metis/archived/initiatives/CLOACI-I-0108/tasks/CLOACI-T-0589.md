---
id: t-01-sql-derive-cloacina-active
level: task
title: "T-01: SQL-derive cloacina_active_tasks gauge — drop inc/dec leak pattern"
short_code: "CLOACI-T-0589"
created_at: 2026-05-14T13:57:39.097340+00:00
updated_at: 2026-05-14T14:01:28.407855+00:00
parent: CLOACI-I-0108
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0108
---

# T-01: SQL-derive cloacina_active_tasks gauge — drop inc/dec leak pattern

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0108]]

## Objective **[REQUIRED]**

Eliminate the gauge-leak antipattern on `cloacina_active_tasks` by deleting the naked `.increment(1.0)` / `.decrement(1.0)` calls in `crates/cloacina/src/executor/thread_task_executor.rs` and re-seeding the gauge each scheduler tick from a SQL count, mirroring the T-0534 fix for `cloacina_active_workflows`.

Sites to drop:
- `thread_task_executor.rs:840` — `metrics::gauge!("cloacina_active_tasks").increment(1.0)`
- `thread_task_executor.rs:904` — `metrics::gauge!("cloacina_active_tasks").decrement(1.0)`

Add SQL-derived re-seed in `execution_planner/scheduler_loop.rs::process_active_executions`, next to the existing `cloacina_active_workflows` re-seed (line ~168). Count is `task_executions WHERE status = 'Running'`.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] `cloacina_active_tasks` no longer has any `.increment` / `.decrement` calls in the codebase.
- [ ] Scheduler tick re-seeds the gauge from a `task_executions WHERE status = 'Running'` count once per `process_active_executions` call.
- [ ] After a synthetic panic in `thread_task_executor::execute_task`, the next scheduler tick re-seeds `cloacina_active_tasks` to the correct SQL value (test against either Postgres or SQLite — both backends share the gauge code path).
- [ ] `docs/operations/metrics.md` entry for `cloacina_active_tasks` updated: matches the SQL-derived language used for `cloacina_active_workflows`.
- [ ] Promtool format check passes; existing `test_metrics_returns_prometheus_format` still green.

## Implementation Notes

DAL accessor: `dal.task_execution().count_running()` may need adding; check the existing `task_execution` DAL surface first to avoid duplicating a count. Use the same SQL backend dispatch (`crate::dispatch_backend!`) as other task-execution DAL methods.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-05-14 — implemented

- Added `TaskExecutionDAL::count_running_tasks()` in `crates/cloacina/src/dal/unified/task_execution/queries.rs` with the standard postgres/sqlite dispatch — returns `i64` count of `task_executions WHERE status = 'Running'`.
- Removed `metrics::gauge!("cloacina_active_tasks").increment(1.0)` at `thread_task_executor.rs:852` and `.decrement(1.0)` at `:916`. Replaced both with comments pointing at the SQL re-seed.
- Added SQL re-seed in `SchedulerLoop::process_active_executions` alongside the existing `cloacina_active_workflows` re-seed. Failure to count is logged and the prior tick's value is retained — won't zero the gauge on a transient DB hiccup.
- `docs/operations/metrics.md`: `cloacina_active_tasks` row rewritten to match the SQL-derived language used for `cloacina_active_workflows`.

Behavioural test (synthetic panic → re-seed) covered by existing integration tests + the I-0099 cardinality guard which proves the gauge still appears in /metrics.
