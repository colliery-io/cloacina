---
id: bug-polling-and-batch-accumulators
level: task
title: "BUG: polling and batch accumulators silently degrade to passthrough in packaged graphs"
short_code: "CLOACI-T-0896"
created_at: 2026-07-12T01:36:33.300520+00:00
updated_at: 2026-07-14T22:24:25.278833+00:00
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

# BUG: polling and batch accumulators silently degrade to passthrough in packaged graphs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0891 (2026-07-12, I-0138 feature-coverage push):** the packaged-graph accumulator factory (`computation_graph/packaging_bridge.rs:225`) matches only `"stream"` and `"state"`; everything else — including `"polling"` and `"batch"`, which have real macros (`#[polling_accumulator(interval)]`, `#[batch_accumulator(flush_interval, max_buffer_size)]`) and real python builders (`cloaca.polling_accumulator/batch_accumulator`) — hits the `_ =>` arm and **silently becomes a passthrough accumulator**. A user who declares a batch accumulator in a packaged graph gets per-event firing with no warning: worse than unsupported, it's silently wrong. (Same silent-degradation class as the pre-T-0839 bug for authored specs.)

**Fix:**
1. `packaging_bridge.rs` factory match gains `"polling"` and `"batch"` arms wired to their real factories (they exist for the embedded path — locate `PollingAccumulatorFactory`/`BatchAccumulatorFactory` or the runtime equivalents and thread their configs: `interval`; `flush_interval`/`max_buffer_size`).
2. The `_ =>` fallback should WARN loudly (or fail the load) instead of silently passthrough-ing an unknown declared type.
3. Extend the T-0891 CG feature-tour example to cover polling + batch once functional; its harness lane is the regression net.

**Acceptance:** a packaged graph declaring polling and batch accumulators (macro or `[[metadata.accumulators]]` override) exhibits real polling/batching behavior on the demo stack; unknown types are loud. Related: [[CLOACI-T-0891]], T-0839 (the authored-spec analog of this bug).

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

## Status Updates **[REQUIRED]**

**2026-07-14 — CONFIRMED LIVE (Python-gap sweep).** Built `python-batch-graph` (a `@cloaca.batch_accumulator(flush_interval="1s", max_buffer_size=5)` packaged CG) and ran it on the gold path. It "passes" — but ONLY because the injected events fire the reactor immediately (passthrough behavior), NOT because the buffer flushes. Confirms the `_ =>` fallthrough at `packaging_bridge.rs:225` silently degrades batch → passthrough.

**Effort re-grounded (the factories do NOT exist for the packaged bridge):**
- Only `PassthroughAccumulatorFactory`, `StreamBackendAccumulatorFactory`, `StateAccumulatorFactory` exist in `packaging_bridge.rs`. There is NO `BatchAccumulatorFactory`/`PollingAccumulatorFactory` to "wire" — they must be WRITTEN.
- **Batch (moderate):** the runtime `batch_accumulator_runtime<B: BatchAccumulator>(acc, ctx, socket_rx, flush_rx, config)` exists but is generic over the `BatchAccumulator` TRAIT with no list-collecting impl (tests use a bespoke `SumBatchAccumulator`). A packaged batch factory needs (a) a generic passthrough-style `BatchAccumulator` that folds `serde_json::Value` events into `Vec<serde_json::Value>` and emits on flush, (b) a flush-timer task feeding `flush_rx` on `flush_interval` + max_buffer_size gating, (c) config parsing. ~mirror of `StateAccumulatorFactory` + a timer. Socket-driven, so it fits the existing spawn contract.
- **Polling (hard):** `polling_accumulator_runtime` drives a `poll()` FUNCTION on an interval (not socket events). In the packaged path the poll fn is the PYTHON function — so a packaged polling factory needs an FFI bridge that calls the loaded Python accumulator fn each interval. No socket. Materially more work than batch.
- **`state` works** (StateAccumulatorFactory real) — verified separately by `python-stateful-graph` (that example IS legit).

**2026-07-14 — BATCH DONE (commit d2043bbc).** Implemented `JsonListBatchAccumulator` + `BatchAccumulatorFactory` (socket-driven, mirrors `StateAccumulatorFactory`; flush_interval/max_buffer_size parsed via `parse_duration_str`) and a central `accumulator_factory_for(type, config)` that all four packaged-reactor dispatch sites route through — with a **loud WARN + passthrough fallback for unknown kinds** (accept #2 met). `python-batch-graph` now exercises the REAL batch factory and passes on the gold path; state/passthrough regression-checked live. No FFI change needed for batch.

**POLLING — scope re-grounded, materially LARGER than "harder FFI".** Unlike batch (socket-driven), a polling accumulator must CALL its poll function on an interval; on the packaged path that fn is the PYTHON accumulator fn, and the plugin FFI has NO accumulator-invoke method (only execute_graph/execute_task/invoke_trigger_poll/invoke_triggerless_graph). So packaged polling requires:
1. New FFI method `invoke_accumulator_poll(name) -> Option<bytes>` on `CloacinaPlugin` → **plugin interface version bump 5 → 6** (ABI change; every plugin recompiles; `fidius_validation::test_plugin_info_populated` expectation moves 5→6; loader keeps loading v5 packages via the `NotImplemented` fallback).
2. Python plugin shell implementing it (call the registered Python accumulator poll fn; Some→emit / None→skip).
3. A `PollingAccumulatorFactory` whose `poll()` calls that FFI on the interval — needs the loaded plugin handle threaded into the factory / `AccumulatorSpawnConfig` (today it carries none). `accumulator_factory_for(type, config)` can't build polling without the handle.

~~Cross-crate, ABI-bumping...split POLLING to its own ticket.~~ **SUPERSEDED — no ABI bump needed.**

**2026-07-14 — POLLING DONE, T-0896 COMPLETE (commit b010dbee).** The ABI-bump concern was WRONG (maintainer pushed back — correctly). On the Python path the poll fn lives IN-PROCESS: `ACCUMULATOR_REGISTRY` keeps the callable (only tests drain it; the reconciler imports the module in the server process), so no FFI accumulator-invoke method is needed. Drove it exactly like a Python poll trigger:
- `cloacina/packaging_bridge`: `PollClosure` type + OnceLock builder hook (`register_polling_accumulator_builder`); `ClosurePollingAccumulator` (runs the injected closure under `spawn_blocking` → GIL off the async executor); `PollingAccumulatorFactory` resolves the closure by name at spawn and runs `polling_accumulator_runtime` on the config interval. Wired into `accumulator_factory_for`'s `"polling"` arm.
- `cloacina-python`: `resolve_poll_closure(name)` looks up the registered Python poll fn and wraps it (`call0` → `depythonize` → JSON bytes; None → skip). Installed from `register_authoring`, so BOTH embeddings wire it (pip wheel + server synthetic `ensure_cloaca_module`).
- Example `python-polling-graph` + new `_graph_autofire_steps` lane asserting `poll_reactor` self-fires (no inject). **Verified live: reactor self-fired.**

**T-0896 CLOSED:** batch (d2043bbc) + polling (b010dbee) + loud-WARN fallback. No plugin interface/ABI bump. All accumulator kinds now behave correctly on the packaged path (passthrough/state/batch/polling; stream = kafka/T-0898 track).
