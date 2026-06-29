---
id: configured-wasm-execution-in-the
level: task
title: "Configured WASM execution in the runtime/scheduler"
short_code: "CLOACI-T-0824"
created_at: 2026-06-28T23:57:42.943615+00:00
updated_at: 2026-06-29T02:03:09.558810+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Configured WASM execution in the runtime/scheduler

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Bridge the runtime/scheduler to invoke a **configured WASM operator** as its bound primitive: `Task::execute`, `Trigger::poll`, the accumulator event loop, the reactor firing -> a call into the configured WASM instance (fidius wire).

**AC:** a registered WASM operator runs as each primitive kind (task executes; trigger polls + fires; accumulator buffers; reactor fires) within a workflow/schedule; the sandbox holds; failures surface as the primitive's normal error path. Blocked by CLOACI-T-0823.

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

### 2026-06-28 — TRIGGER bridge + Runtime registration landed (active)

Built on T-0823's idioms (spawn_blocking async->sync bridge, host-side
`#[plugin_interface]` re-declaration, fail-closed `LoaderError`s). All WASM
stays behind the default-OFF `operators-wasm` feature.

**Done / proven (must-haves):**
- Contract crate (`cloacina-operator-contract`): added `TriggerInvocation` /
  `PollOutcome` wire types (+ `fire`/`skip`/`err` ctors), `METHOD_POLL`,
  `TRIGGER_OPERATOR_INTERFACE_VERSION`; round-trip unit tests pass.
- Trigger bridge (`operator_loader.rs`): host-side `TriggerOperator`
  `#[plugin_interface]` trait (`poll`); `WasmTriggerOperator: impl Trigger`
  whose async `poll()` spawn_blocks into the sync WASM `poll`, mapping
  `PollOutcome` -> `TriggerResult::Fire(ctx)`/`Skip`, outcome `error` ->
  `TriggerError::PollError`. `load_trigger_operator(search_path, package,
  config, TriggerBinding)`.
- Runtime registration: `load_operator(runtime, search_path, package, config,
  OperatorBinding)` reads `operator.json`, dispatches on `primitive_kind`,
  registers Task -> `register_task(namespace)` / Trigger ->
  `register_trigger(name)`. Mismatched binding fails closed.
- Fixture + e2e (`examples/operator-contract/trigger-operator-fixture`,
  `tests/operator_trigger_wasm.rs`): trigger fixture compiles to wasm32-wasip2,
  loads, polls Fire/Skip config-bound; registration lands the primitive in the
  correct `Runtime` registry by `primitive_kind`. 6 tests pass; existing task
  e2e (3) still green; contract unit tests (4) pass.

**Design wrinkle - poll_interval sourcing:** the operator manifest has no
poll-interval field (it describes the component, not a deployment), and the
WASM guest never needs the cadence. So host-side scheduling metadata
(poll_interval / allow_concurrent / workflow_name / cron_expression) is bound
via a separate `TriggerBinding` at load - the trigger analogue of "config binds
once". The guest `poll` decides *whether* to fire; the host decides *how often*.

**Validation (exact):**
- `cargo test -p cloacina-operator-contract` -> 4 passed.
- `cargo check -p cloacina` -> clean; `cargo tree -p cloacina` wasmtime count =
  0 (default), 43 (with `--features operators-wasm`).
- `cargo test -p cloacina --features operators-wasm --test operator_trigger_wasm`
  -> 6 passed.

**Deferred (clearly-noted continuation):** ACCUMULATOR + REACTOR. Their sync
contract traits (`AccumulatorOperator::ingest` / `ReactorOperator::evaluate`)
and wire types (`AccumulatorInvocation`/`AccumulatorOutcome`,
`ReactorInvocation`/`ReactorOutcome`) are defined and the `Wasm*Operator`
bridge shapes are documented in `operator_loader.rs`, but full impl + fixtures
are left because neither is a plain `Runtime` constructor: the accumulator is a
stateful `&mut self` event sink driven by `accumulator_runtime` (no `Runtime`
registry), and a reactor is represented as a `ReactorRegistration` descriptor
(not a callable) evaluated by the CG scheduler. Wiring those needs threading a
callable evaluator / event loop through the scheduler - beyond this task's
surface. Not committed (per reviewer request).