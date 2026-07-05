---
id: packaged-python-state-stream
level: task
title: "Packaged Python state/stream accumulators silently degrade to passthrough server-side (names-only reactor dispatch drops accumulator_type)"
short_code: "CLOACI-T-0839"
created_at: 2026-07-05T16:08:11.000964+00:00
updated_at: 2026-07-05T16:08:11.000964+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Packaged Python state/stream accumulators silently degrade to passthrough server-side (names-only reactor dispatch drops accumulator_type)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

A packaged Python graph's `@cloaca.state_accumulator(capacity=N)` (and likely `@cloaca.stream_accumulator`) loses its kind when the package is loaded SERVER-SIDE: the accumulator spawns as a plain **passthrough**, so windowing/state semantics silently vanish — every event just flows through one-at-a-time. No error anywhere; the graph "works" but computes the wrong thing.

## Root cause (code-cited, found 2026-07-05 while live-verifying T-0744)

The reconciler's Python branch dispatches reactors via
`dispatch_runtime_reactors_into_scheduler` (`computation_graph/packaging_bridge.rs:664`), which iterates `runtime.get_reactor(name)` registrations that carry **`accumulator_names` only — no `accumulator_type`, no config**. Factory selection (`:681-695`) consults only the manifest `accumulator_overrides` (`[metadata].accumulators` in package.toml) and **defaults to `PassthroughAccumulatorFactory` when no override matches**. The Python-side `PyAccumulatorRegistration` (cloacina-python `computation_graph.rs:122` — name, `accumulator_type: "state"`, config incl. `capacity`) never reaches this dispatch. (A second, type-aware dispatch site at `:755-775` falls back to `acc.accumulator_type` — the names-only site has no such data to fall back to.)

## Live evidence (demo stack, 2026-07-05)

`demo-py-state`'s `py_window` (`capacity=5`): 3 operator injects → reactor fired 3× (`when_any` per event ✓ passthrough behavior), `events_total=3`, but `cloacina_accumulator_buffer_depth{accumulator="py_window"} = 0` and the T-0744 probe reports no buffer — a real `state_accumulator_runtime` maintains the VecDeque and reports depth (unit-proven: `test_state_accumulator_probe_reports_buffer_fill`). The spawned runtime for py_window is provably NOT the state runtime.

Note the embedded/cloaca-driven path (cloacina-python `computation_graph.rs:859`) maps `"state"` correctly — this is specifically the PACKAGED server load path, i.e. the T-0688 parity gap re-opened on the packaged lane. T-0688's original tests presumably covered embedded only.

## Fix sketch

Thread the Python accumulator registrations through to the dispatch: either (a) the Python loader converts its `PyAccumulatorRegistration`s into `cloacina_workflow_plugin::types::AccumulatorConfig` overrides handed to `dispatch_runtime_reactors_into_scheduler` (surgical — matches the existing override channel), or (b) enrich the Runtime's reactor registration with per-accumulator `(type, config)` so the names-only dispatch site can fall back type-aware like `:755` does. Add a packaged-py e2e asserting `py_window` emits LISTS with eviction at capacity (and, with T-0744, `buffer_depth/capacity = N/5` in the health API).

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (silent wrong-results for packaged Python CGs using state/stream accumulators)

### Impact Assessment
- **Affected Users**: anyone running a packaged Python CG with `@cloaca.state_accumulator` / stream accumulators via cloacina-server.
- **Reproduction Steps**:
  1. Seed the demo stack; let `demo-py-state` load.
  2. `POST /v1/health/accumulators/py_window/inject` ×3 with `{"event": {"bid": 1.5, "ask": 1.9}}`.
  3. Observe `cloacina_accumulator_buffer_depth{accumulator="py_window"}` stays 0 and the reactor receives per-event fires, not growing windows.
- **Expected vs Actual**: expected a capacity-5 window (boundary = list, evicting oldest); actual passthrough (boundary = single event, no state, no persistence).

## Acceptance Criteria **[REQUIRED]**

- [ ] A packaged Python `@cloaca.state_accumulator(capacity=N)` loaded via cloacina-server runs `state_accumulator_runtime` (windowed list boundaries, DAL persistence, eviction at N).
- [ ] Same for stream accumulators (kind honored, not passthrough-defaulted).
- [ ] Health API shows `buffer_depth`/`buffer_capacity = N` for the state accumulator (T-0744 fields).
- [ ] Regression test on the packaged-Python load path (not just embedded).

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

*To be added during implementation*
