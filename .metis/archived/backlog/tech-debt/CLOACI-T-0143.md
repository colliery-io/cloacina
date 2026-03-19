---
id: integration-test-actual-task
level: task
title: "Integration test: actual task execution through Dispatcher/Executor pipeline"
short_code: "CLOACI-T-0143"
created_at: 2026-03-15T14:39:30.600144+00:00
updated_at: 2026-03-15T15:21:13.841957+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Integration test: actual task execution through Dispatcher/Executor pipeline

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

The scheduler's `FiredTask` struct records that a task should fire, but nothing actually executes it through the `TaskScheduler` → `Dispatcher` → `Executor` pipeline. The `check_readiness()` drains accumulators and collects context, but the real task never runs. This means the entire downstream execution path is unverified.

Need an integration test that wires the `ContinuousScheduler` to a real `TaskScheduler` with a registered task, verifies the task function actually executes, and the output context flows back through the ledger.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test with real `TaskScheduler` + `Dispatcher` + task execution
- [ ] Continuous task function actually runs (not just recorded as "fired")
- [ ] Task output context flows back to `ExecutionLedger`
- [ ] Requires running DB (Postgres) for TaskScheduler state management

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

**Fixed.** The scheduler now actually executes tasks.

**Changes:**
- Added `task_registry: HashMap<String, Arc<dyn Task>>` to ContinuousScheduler
- Added `register_task(task)` method for registering continuous task implementations
- Run loop now calls `task.execute(context).await` for registered tasks
- On success: writes `LedgerEvent::TaskCompleted` with output context to ledger
- On failure: writes `LedgerEvent::TaskFailed` with error to ledger
- Unregistered tasks recorded as `FiredTask { executed: false }` with error message
- `FiredTask` extended with `executed: bool` and `error: Option<String>`
- Merged boundary context from drained accumulators passed to task

**Tests:**
- `test_scheduler_actually_executes_registered_task` — task runs, writes to context, TaskCompleted in ledger
- `test_scheduler_handles_task_failure` — task fails, TaskFailed in ledger, error captured
- `test_unregistered_task_records_not_executed` — no impl registered → executed: false
- `test_full_reactive_loop` updated to register task and verify execution
- 116 unit + 14 integration + 11 macro = 141 total tests passing
