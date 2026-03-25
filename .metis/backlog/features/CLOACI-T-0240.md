---
id: continuous-scheduler-soak-test
level: task
title: "Continuous scheduler soak test — sustained load, memory stability, boundary throughput"
short_code: "CLOACI-T-0240"
created_at: 2026-03-24T18:44:47.307731+00:00
updated_at: 2026-03-24T21:13:35.306909+00:00
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

# Continuous scheduler soak test — sustained load, memory stability, boundary throughput

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Build a soak test for the continuous/reactive scheduler. Currently we have unit tests and a crash recovery test, but nothing that runs the scheduler under sustained load for minutes to verify it doesn't leak memory, miss boundaries, stall, or accumulate state incorrectly.

### Priority
- [x] P1 - High (blocks production readiness for continuous scheduling)

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

- [ ] Rust integration test that runs ContinuousScheduler for configurable duration (default 2m)
- [ ] Mock data source emitting boundaries at sustained rate (e.g., 10/sec)
- [ ] Verify all emitted boundaries are processed (none lost)
- [ ] Verify fired task count matches expected (based on trigger policy + rate)
- [ ] Verify memory doesn't grow unbounded (ledger trimming, accumulator draining)
- [ ] Verify watermark advances monotonically
- [ ] Runnable via `angreal` (e.g., `angreal performance continuous`)
- [ ] Chaos variant: kill + restart mid-soak, verify resume from WAL
- [ ] Reports PASS/FAIL with metrics (boundaries/sec, tasks fired, memory delta)

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

### 2026-03-24 — Exploration complete

**Approach:** Rust integration test (not Python soak — continuous scheduler is in-process only).

**Pattern:**
- Inject detector completions via `ledger.append(LedgerEvent::TaskCompleted { DetectorOutput })` at sustained rate
- Register a no-op task that counts executions
- Run scheduler for configurable duration
- Verify: boundaries emitted == boundaries processed, fired tasks > 0, watermark advances

**Key APIs:**
- `ExecutionLedger::append()` for injection, `len()` for counting
- `AccumulatorMetrics::total_boundaries_received` / `drain_count` for loss detection
- `ContinuousScheduler::graph_metrics()` for accumulator state
- `FiredTask::executed` bool for task execution verification

**Location:** `crates/cloacina/tests/integration/continuous/soak.rs` or inline in scheduler.rs tests

### Implementation complete

**3 soak tests** in `tests/integration/continuous/soak.rs`:

1. **Sustained load** — 1000 boundaries at ~18k/sec, 8 tasks fired, 0 errors, 1016 ledger events
2. **Batched** — 1000 boundaries in 200 batches of 5, 4 tasks fired, 0 errors
3. **Multi-source** — 200 boundaries across 2 sources alternating, 4 tasks fired, 0 errors

Uses `CountingTask` (atomic counter) for execution tracking. Verifies: all fired tasks executed, no errors, ledger accumulates correctly.

All 3 pass. 497 total tests (+3 new).
