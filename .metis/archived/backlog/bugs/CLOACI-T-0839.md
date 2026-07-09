---
id: packaged-python-state-stream
level: task
title: "Packaged Python state/stream accumulators silently degrade to passthrough server-side (names-only reactor dispatch drops accumulator_type)"
short_code: "CLOACI-T-0839"
created_at: 2026-07-05T16:08:11.000964+00:00
updated_at: 2026-07-05T17:33:46.210109+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-07-05 — implementation (branch fix/t0839-py-state-accumulator-dispatch), option (b)
Chose the registration-enrichment shape: new `AccumulatorSpec { name, accumulator_type, config }` in cloacina-computation-graph + `ReactorRegistration.accumulator_specs: Vec<AccumulatorSpec>` (empty on paths where the kind travels out-of-band — Rust packaged reactors use FFI metadata; 17 construction sites updated).
- **Producer** (cloacina-python `reactor.rs`): the `@cloaca.reactor` registration closure resolves specs LAZILY from `get_registered_accumulators()` (so decorator order doesn't matter), folding state `capacity` into the config map under the same key `state_capacity_from_config` reads.
- **Consumers**: `build_view_python` (loading.rs) and the names-only `dispatch_runtime_reactors_into_scheduler` (packaging_bridge.rs:664 site) now resolve with precedence **manifest override (deployment wins) → authored spec → passthrough**.
- **Tests**: `build_view_python_honors_authored_accumulator_specs` (authored state spec survives; override still beats it) — reconciler lib suite green; new producer-side test `test_reactor_registration_carries_authored_accumulator_specs` (py decorators → registration carries state/capacity=5) in python_reactor_library. cloacina-python + macros + computation-graph check clean.
Remaining: integration-tests compile check, full py test suite, live demo re-verify (py_window N/5 — also closes T-0744's deferred AC), commit/PR.

### 2026-07-05 — 🎯 LIVE-VERIFIED + a SECOND latent bug found and fixed in the same arc
First live retest proved the spec threads through (`buffer_capacity: 5` at spawn = the REAL state runtime) but injects bounced: "state accumulator deserialize error: expected value". **Onion layer #2**: the REST inject route pre-wrapped events in the bincode boundary frame, violating the accumulator SOCKET contract (raw JSON bytes — what WS sends at ws.rs:277 and what every runtime decodes). The pre-wrap (a) made state/batch accumulators reject REST injects outright and (b) DOUBLE-encoded through passthrough — the long-standing undecodable `inputs: null` in the T-0775 fires log, finally explained. Fixed: inject sends `serde_json::to_vec(&event)`; the frame encoding stays on the reactor `fire_with` path (boundary CACHE, where frames are correct).

**FINAL LIVE PROOF (demo stack):** `py_window` (state, capacity=5) after 3 REST injects → `buffer_depth=3 / buffer_capacity=5` in BOTH `/v1/health/accumulators` and the Prometheus gauge; reactor fired per ingest on real windowed boundaries. All four ACs met (stream rides the same resolution path by construction); T-0744's deferred N/5 AC demonstrated in the same run.

**Known residual (separate, cosmetic):** the T-0775 fires-log input CAPTURE decodes only `bincode(json_bytes)` frames; state accumulators emit `bincode(Vec<Value>)` typed frames, so their captured inputs still render `null` in the fires log (worth checking whether py GRAPH input decode has the same gap). COMPLETE.