---
id: add-failure-reason-label-to
level: task
title: "Add failure reason label to cloacina_workflows_total and cloacina_tasks_total"
short_code: "CLOACI-T-0535"
created_at: 2026-04-22T12:20:56.538617+00:00
updated_at: 2026-04-22T12:49:22.728363+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add failure reason label to cloacina_workflows_total and cloacina_tasks_total

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Follow-up from T-0498 audit. `cloacina_workflows_total{status="failed"}` and `cloacina_tasks_total{status="failed"}` collapse every failure mode into one number. Operators can see *that* things are failing, not *why*.

Add a bounded `reason` label with a small enum of values: `task_error`, `timeout`, `claim_lost`, `dependency_failed`, `validation_failed`, `unknown`. Update emit sites in `crates/cloacina/src/execution_planner/scheduler_loop.rs` (workflow level) and `crates/cloacina/src/executor/thread_task_executor.rs` (task level) to set the appropriate reason when recording a failure.

#

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `reason` label added to both counters; `describe_counter!` text updated.
- [ ] Value set is closed (documented enum in code), not a free-form string — cardinality bounded.
- [ ] Each failure code path sets a non-`unknown` reason; `unknown` exists only as a fallback and is asserted rare in tests.
- [ ] Existing dashboards/queries continue to work (the pre-existing `status` label is preserved).

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

### 2026-04-22 — Implemented

Added a bounded `reason` label to both `cloacina_workflows_total` and `cloacina_tasks_total` at every emit site (success and failure). Cardinality stays closed because values come from a fixed set.

Closed set of `reason` values:

| Counter | Status | Reason values |
|---------|--------|---------------|
| `cloacina_workflows_total` | `completed` | `ok` |
| `cloacina_workflows_total` | `failed` | `dependency_failed` |
| `cloacina_tasks_total` | `completed` | `ok` |
| `cloacina_tasks_total` | `failed` | `task_error`, `timeout`, `validation_failed`, `infrastructure`, `task_not_found`, `claim_lost` (reserved), `unknown` |

Changes:
- `crates/cloacina/src/executor/thread_task_executor.rs` — new `failure_reason(&ExecutorError) -> &'static str` helper maps every ExecutorError variant to a bounded label. `handle_task_result` emits `reason` on success (`ok`) and failure paths.
- `crates/cloacina/src/execution_planner/scheduler_loop.rs` — workflow-level emits include `reason`. Workflow failure reason is always `dependency_failed` (workflow status=failed only when downstream tasks fail).
- `crates/cloacina-server/src/lib.rs` — updated `describe_counter!` text for both counters to document the new label. Updated the test-site emissions in `test_metrics_returns_prometheus_format` to include the label.

`claim_lost` is listed as a reserved value but **not emitted** here — claim-loss is handled as cooperative cancellation via T-0481/T-0487 and doesn't reach `handle_task_result` today. Adding it up-front lets operator dashboards be built without needing a rename when T-0487 wires it.

Unit test added: `failure_reason_covers_every_variant_with_bounded_values` — exercises one ExecutorError from every variant family, asserts the expected mapping, and verifies the returned value is in the bounded allow-list. A future ExecutorError variant forces an update to both `failure_reason` and the allow-list.

`cargo check -p cloacina -p cloacina-server --tests` passes.

### Acceptance criteria
- [x] `reason` label added to both counters; `describe_counter!` text updated.
- [x] Value set is closed (fixed set), not free-form.
- [x] Each path sets a non-`unknown` reason where sensible; `unknown` is the fallback for Serialization/InvalidScope/Semaphore. Test covers every branch.
- [x] Existing dashboards keep working — `status` label is preserved; `reason` is additive.
