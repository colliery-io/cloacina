---
id: t-02-supervisor-component-health
level: task
title: "T-02: Supervisor + component health metrics — restart counter and health-state gauge"
short_code: "CLOACI-T-0585"
created_at: 2026-05-14T13:03:05.751343+00:00
updated_at: 2026-05-14T13:24:12.058253+00:00
parent: CLOACI-I-0099
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0099
---

# T-02: Supervisor + component health metrics — restart counter and health-state gauge

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0099]]

## Objective **[REQUIRED]**

Wire metrics into the supervisor and component health state machine (T-0410/T-0419) so operators can see restarts and component health from Prometheus.

Metrics to add:
- `cloacina_supervisor_restarts_total{graph,component,reason}` — counter. Component ∈ {accumulator, reactor}. Reason bounded: {panic, error, shutdown_timeout}.
- `cloacina_component_health{graph,component,state}` — gauge (0/1 per state). State ∈ {healthy, degraded, starting, stopped, crashed}.

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

- [ ] Restart counter increments at every supervisor restart site with a bounded `reason` label.
- [ ] Health gauge follows the existing state machine: exactly one state per `(graph, component)` is `1`, others are `0`.
- [ ] Labels derived from package metadata only — no event keys or tenant IDs.
- [ ] Unit test simulates a panic + restart and asserts counter increments and gauge transitions.
- [ ] Promtool format check passes; metrics doc updated.

## Implementation Notes

Depends on T-0584 establishing the recorder/registration pattern. Builds directly on the supervisor wiring from I-0081 / T-0412.

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

- `cloacina_supervisor_restarts_total{graph,component,reason}` emitted in `crates/cloacina/src/computation_graph/scheduler.rs` at reactor and accumulator restart paths. Reason derived from `JoinError::is_panic()` via `classify_join_result()`; reactor handle taken via `mem::replace` so the await is non-blocking. `shutdown_timeout` reserved for graceful-shutdown path.
- `cloacina_component_health{graph,component,state}` gauge:
  - Added `AccumulatorHealth::as_state_label()` and `ReactorHealth::as_state_label()` projecting existing state machines onto `{healthy, degraded, starting}`.
  - `emit_component_health()` writes 1 to current state and 0 to every other state value — centralised invariant.
  - `ComputationGraphScheduler::emit_health_metrics()` walks graphs + accumulators, called from `start_supervision` after every tick.
  - `crashed` emitted on circuit-breaker open; `stopped` emitted in `unload_reactor()` across `endpoint_registry_keys`.
- Registered both metrics with `describe_counter!` / `describe_gauge!` in `crates/cloacina-server/src/lib.rs`.
- Unit test `test_supervisor_health_metrics_emit` covers both bounded `reason` values and all five `state` values.
- `docs/operations/metrics.md`: new rows added; "Current gaps" narrowed to runtime metrics + WS.
