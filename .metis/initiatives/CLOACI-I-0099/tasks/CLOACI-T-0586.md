---
id: t-03-accumulator-metrics-events
level: task
title: "T-03: Accumulator metrics — events, emit duration, buffer depth, checkpoint writes"
short_code: "CLOACI-T-0586"
created_at: 2026-05-14T13:03:09.571731+00:00
updated_at: 2026-05-14T13:29:47.323938+00:00
parent: CLOACI-I-0099
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0099
---

# T-03: Accumulator metrics — events, emit duration, buffer depth, checkpoint writes

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0099]]

## Objective **[REQUIRED]**

Instrument all four accumulator kinds (passthrough, stream, polling, batch) so operators can see throughput, emit latency, queue depth, and checkpoint pressure.

Metrics to add:
- `cloacina_accumulator_events_total{graph,accumulator,kind}` — counter. Kind ∈ {passthrough, stream, polling, batch}.
- `cloacina_accumulator_emit_duration_seconds{graph,accumulator}` — histogram. End-to-end emit latency.
- `cloacina_accumulator_buffer_depth{graph,accumulator}` — gauge. Current queue depth for batch/stateful accumulators.
- `cloacina_accumulator_checkpoint_writes_total{graph,accumulator}` — counter. Paired with T-0407/T-0408 checkpoint DAL.

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

- [ ] All four metrics emit from each of the four accumulator runtimes.
- [ ] `kind` label exactly matches the four documented values; never derived from user input.
- [ ] Histogram buckets chosen to span the soak-test p99 range without truncation.
- [ ] Unit tests per accumulator kind assert events/duration/checkpoint emit.
- [ ] Promtool format check passes; metrics doc updated.

## Implementation Notes

Depends on T-0584. Touches accumulator runtime + macros in `cloacina-macros` / accumulator crates. Buffer depth gauge only meaningful for batch and stateful stream accumulators — emit 0 for passthrough/polling.

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

- Added `graph_label()` helper that derives the bounded `graph` label from `CheckpointHandle::graph_name()` (falls back to `embedded` sentinel for DAL-less runtimes) — avoided extending `AccumulatorContext` (~38 struct-literal sites across crates + examples + tests).
- New helpers `record_accumulator_event()` and `set_accumulator_buffer_depth()` centralise `events_total` + `emit_duration_seconds` + `buffer_depth` emission.
- Emit sites:
  - `accumulator_runtime_inner` (passthrough/stream — `kind` picked by presence of event source)
  - `polling_accumulator_runtime` (both timer-tick and socket paths emit with `kind=polling`)
  - `flush_batch` (emits once per flush with `kind=batch`); buffer_depth gauge updated on every push/flush/drain.
  - All four runtimes seed `buffer_depth = 0` at startup so dashboards see a stable series.
- `cloacina_accumulator_checkpoint_writes_total` instrumented inside `CheckpointHandle::save` and `persist_boundary` on success only — failed writes stay log-only.
- All four metrics registered with describe_* in `crates/cloacina-server/src/lib.rs`.
- Unit test `test_accumulator_metrics_emit` covers all four `kind` values + the histogram/gauge/counter signatures.
- `docs/operations/metrics.md` rows added; gap note narrowed to reactor + WebSocket.
