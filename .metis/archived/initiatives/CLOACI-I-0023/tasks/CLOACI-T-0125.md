---
id: continuousscheduler-run-loop-and
level: task
title: "ContinuousScheduler run loop and TaskScheduler integration"
short_code: "CLOACI-T-0125"
created_at: 2026-03-15T11:46:40.473813+00:00
updated_at: 2026-03-15T12:14:28.793062+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# ContinuousScheduler run loop and TaskScheduler integration

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement the `ContinuousScheduler` struct and its `tokio::select!` run loop as specified in CLOACI-S-0008. This is the core orchestrator that observes detector completions, routes boundaries to accumulators, checks task readiness, and submits work to the existing `TaskScheduler`.

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

- [ ] `ContinuousScheduler` struct with `graph`, `execution_ledger`, `exit_edges`, `task_scheduler`
- [ ] Run loop steps 1-5 from S-0008 (simplified: no watermark checks, no late arrival routing)
- [ ] Observes `ExecutionLedger` for detector workflow completions, extracts `DetectorOutput` from context
- [ ] Routes `Change` boundaries to per-edge accumulators via `accumulator.receive()`
- [ ] Checks task readiness per `JoinMode::Any` (any accumulator `is_ready()`)
- [ ] On ready: drains accumulators, submits task to `TaskScheduler` with merged context
- [ ] Completion callback writes `LedgerEvent::TaskCompleted` to ledger
- [ ] Exit edge support: task completion fires one-shot workflow via existing pipeline
- [ ] Integration test: mock detector → accumulator → continuous task execution cycle

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

## Implementation Notes

### Technical Approach
- In `continuous/scheduler.rs`
- Follows `TriggerScheduler` pattern: `tokio::select!` loop, runs as spawned task
- Polls ledger on interval (sub-second, ledger is in-memory)
- Maintains cursor into `ExecutionLedger` to avoid re-processing
- Context merging uses existing `ContextManager` pipeline
- `DataSourceMap` injected into continuous task execution path

### Dependencies
- T-0120 (Accumulator), T-0121 (TriggerPolicy), T-0122 (ExecutionLedger), T-0124 (DataSourceGraph)

## Status Updates

- Created `continuous/scheduler.rs` with `ContinuousScheduler` and `ContinuousSchedulerConfig`
- `tokio::select!` run loop: polls ledger, processes DetectorOutput, routes boundaries, checks readiness, fires tasks
- Supports `JoinMode::Any` and `JoinMode::All` readiness checks
- `process_detector_output()`: extracts DetectorOutput from completed task context, routes Change boundaries to all matching accumulators
- `check_readiness()`: iterates tasks, checks accumulator readiness per JoinMode, drains ready accumulators
- `FiredTask` struct tracks what was fired and when
- Exit edge support via `add_exit_edge()` (wiring to existing pipeline deferred to T-0126)
- Uses `watch::Receiver<bool>` for shutdown signal (matches TriggerScheduler pattern)
- 3 passing tests: detector output processing, run loop with shutdown, empty graph
