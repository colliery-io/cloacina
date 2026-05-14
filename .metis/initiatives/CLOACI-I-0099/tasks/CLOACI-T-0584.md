---
id: t-01-scheduler-loop-metrics-claim
level: task
title: "T-01: Scheduler loop metrics — claim attempts, heartbeats, stale-claim sweeps"
short_code: "CLOACI-T-0584"
created_at: 2026-05-14T13:03:01.628185+00:00
updated_at: 2026-05-14T13:12:04.368377+00:00
parent: CLOACI-I-0099
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0099
---

# T-01: Scheduler loop metrics — claim attempts, heartbeats, stale-claim sweeps

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0099]]

## Objective **[REQUIRED]**

Emit Prometheus metrics for the scheduler claim/heartbeat/sweep loop. Establishes the registration pattern that subsequent I-0099 tasks will follow.

Metrics to add:
- `cloacina_scheduler_claim_attempts_total{outcome}` — counter. Outcome ∈ {claimed, contended, empty}.
- `cloacina_scheduler_heartbeat_writes_total` — counter.
- `cloacina_scheduler_stale_claims_swept_total` — counter (sweeper that subsumed RecoveryManager per T-0502).

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

## Acceptance Criteria **[REQUIRED]**

- [ ] All three metrics registered in the central recorder and emitted from the scheduler claim/heartbeat/sweep code paths.
- [ ] Outcome labels for `claim_attempts_total` are bounded to the three documented enum values; no free-form strings.
- [ ] Unit test asserts each counter increments under the expected code path.
- [ ] `angreal test metrics-format` (promtool /metrics scrape, T-0536) still passes.
- [ ] `docs/operations/metrics.md` (T-0537) updated with the three new entries.

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

- Registered three new counters in `crates/cloacina-server/src/lib.rs` alongside existing `describe_counter!` block.
- Emit sites:
  - `crates/cloacina/src/executor/thread_task_executor.rs` — `cloacina_scheduler_claim_attempts_total{outcome="claimed"|"contended"}` in `claim_for_runner` match arms; `cloacina_scheduler_heartbeat_writes_total` in heartbeat loop on `Ok`.
  - `crates/cloacina/src/execution_planner/scheduler_loop.rs` — `cloacina_scheduler_claim_attempts_total{outcome="empty"}` when `dispatch_ready_tasks` finds no ready tasks.
  - `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs` — `cloacina_scheduler_stale_claims_swept_total` after each successful `mark_ready`.
- Unit test `test_scheduler_loop_metrics_emit` added to `cloacina-server/src/lib.rs` tests — emits each new metric, scrapes `/metrics`, asserts presence + bounded `outcome` labels.
- `docs/operations/metrics.md` updated: three new rows in the counters table; two new PromQL example queries; "Current gaps" note revised to remove scheduler-loop signals.

Establishes the pattern for T-0585..T-0588: register via `metrics::describe_*!` in server startup, emit via `metrics::counter!`/`gauge!`/`histogram!` macros at the call site, then add a server-level test that exercises the emit + verifies `/metrics` output.
