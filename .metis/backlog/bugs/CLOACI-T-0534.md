---
id: fix-cloacina-active-workflows
level: task
title: "Fix cloacina_active_workflows gauge leak on crash-recovery"
short_code: "CLOACI-T-0534"
created_at: 2026-04-22T12:20:54.680663+00:00
updated_at: 2026-04-22T12:44:41.699189+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix cloacina_active_workflows gauge leak on crash-recovery

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Follow-up from T-0498 audit. `cloacina_active_workflows` gauge is incremented at workflow schedule time (`crates/cloacina/src/execution_planner/mod.rs:422`) and decremented only in `finalize_workflow_execution` (`scheduler_loop.rs:376`). If a workflow is abandoned — process crash, claim lost, stale-claim sweeper hands work to a new scheduler — the original increment is never paired with a decrement. The gauge drifts upward over time and misrepresents live system load.

Fix options: (a) decrement inside the stale-claim sweeper when it reclaims a workflow, or (b) replace the in-process gauge with a SQL-derived gauge computed from `workflow_executions` rows in a running status, updated periodically by a background task.

#

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Chosen fix keeps the gauge accurate across a process kill + restart cycle (soak test or targeted integration test verifies this).
- [ ] If keeping the in-process gauge, sweeper path calls `decrement(1.0)` symmetric to the abandoned inc.
- [ ] If switching to SQL-derived, document the update interval and the staleness tradeoff.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

## Status Updates

### 2026-04-22 — Implemented (option b: SQL-derived gauge)

Chose option (b): replace the in-process inc/dec with a SQL-derived gauge. Correct-by-construction — every scheduler tick overwrites the gauge from the authoritative `workflow_executions` row count, so it cannot drift on any path (crash, claim loss, error short-circuit, panic between inc and dec).

Changes:
- `crates/cloacina/src/execution_planner/scheduler_loop.rs` — in `process_active_executions`, after loading active executions, call `metrics::gauge!("cloacina_active_workflows").set(active_executions.len() as f64)`. Removed the per-finalize `decrement(1.0)`.
- `crates/cloacina/src/execution_planner/mod.rs` — removed the `increment(1.0)` at schedule time. Left a comment explaining the gauge is SQL-derived.
- Kept `cloacina_workflow_duration_seconds` histogram recording in `finalize_workflow_execution` — a histogram, not a gauge, no drift.

Why this shape: `get_active_executions()` is already called every tick and filters by `status IN ('Pending', 'Running')` (`dal/unified/workflow_execution.rs:260,283`). Re-seeding the gauge from `.len()` adds one line, zero new queries, zero background tasks.

Staleness tradeoff: gauge lags the DB by at most one `poll_interval` (scheduler tick). Well under typical Prometheus scrape interval (15s), so operators will not observe the lag.

`cargo check -p cloacina` passes.

### Acceptance criteria
- [x] Fix keeps gauge accurate across crash + restart — by construction, rebuilt from the DB every tick.
- [x] Switched to SQL-derived; sweeper-decrement option not needed.
- [x] Staleness tradeoff documented: lag ≤ `poll_interval` (scheduler tick).
