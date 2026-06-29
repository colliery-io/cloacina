---
id: accumulator-reactor-constructor
level: task
title: "Accumulator + reactor constructor execution (scheduler-level wiring)"
short_code: "CLOACI-T-0828"
created_at: 2026-06-29T02:03:51.709831+00:00
updated_at: 2026-06-29T12:35:50.415158+00:00
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

# Accumulator + reactor constructor execution (scheduler-level wiring)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Implement constructor EXECUTION for the **accumulator** and **reactor** primitives — deferred from [[CLOACI-T-0824]] because, unlike task/trigger, they are NOT plain `Runtime` constructors:

- **Accumulator**: a stateful `&mut self` event sink driven by `accumulator_runtime` (no `Runtime` registry). Needs a `WasmAccumulatorConstructor` bridging the async ingest path to the sync WASM `ingest`, threading the configured handle through the accumulator runtime's event loop.
- **Reactor**: stored as a `ReactorRegistration` *descriptor* evaluated by the CG scheduler (not a callable). Needs a `WasmReactorConstructor` exposing the sync WASM `evaluate` as the reactor's firing-criteria callable, threaded through the scheduler.

The sync contract traits (`AccumulatorConstructor::ingest`, `ReactorConstructor::evaluate`) + wire types are already defined in `cloacina-constructor-contract` (T-0824); the bridge shapes are sketched in `constructor_loader.rs`.

**AC:** an accumulator constructor + a reactor constructor each load, register/wire, and run end-to-end (config-bound), feature-gated behind `constructors-wasm`; mirrors the task/trigger bridges. Completes the four-primitive constructor scope for [[CLOACI-I-0132]]. Blocked by CLOACI-T-0824.

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

### 2026-06-29 — Accumulator DONE end-to-end; Reactor bridge + run-loop seam DONE, scheduler threading deferred

Branch `i0132-t0828-accumulator-reactor`. Constructor EXECUTION + AUTHORING for accumulator + reactor implemented. NOT committed (under review).

**Accumulator (fully proven):**
- Bridge `WasmAccumulatorConstructor: impl Accumulator` in `constructor_loader.rs`. `Accumulator::process` is SYNC (runtime calls it inline on its processor task), so UNLIKE task/trigger the blocking wasmtime `ingest` is called DIRECTLY — no `spawn_blocking` (no async method to hang it off). Config bound once at load.
- Wire wrinkle found+fixed: `type Output = Vec<u8>` (boundary JSON bytes), NOT `serde_json::Value` — `BoundarySender` bincode-wraps the output; a `Value` does not round-trip through bincode (`deserialize_any`), and `bincode(json_bytes)` IS the canonical boundary frame the reactor's `capture_fire_inputs`/FFI bridge decode. So the output is directly reactor-consumable.
- Wiring: `load_accumulator_constructor(...)` returns the `impl Accumulator`; driven by the existing `accumulator_runtime` loop (no `Runtime` registry for accumulators — the structural difference vs task/trigger).
- Macro `#[constructor(kind = accumulator)]`: author writes `fn ingest(&self, event_json: &str) -> Result<Option<String>, ConstructorError>` (`Some` emits, `None` buffers); `#[config]`-only fields.
- e2e (`constructor_accumulator_wasm.rs`, 4 PASS): macro fixture → wasm → load → `accumulator_runtime` emits above threshold, buffers below, config-bound.

**Reactor (bridge + run-loop seam proven; scheduler threading deferred):**
- `Reactor` is a concrete struct with hardcoded `WhenAny`/`WhenAll` criteria — no `Reactor` trait. Added a contained pluggable seam: `pub trait ReactorFireDecider { async fn should_fire(&self, &InputCache) -> bool }` + `Reactor::with_evaluator(...)`; the `Latest`-path `should_run` consults it when present, else the existing criteria. Defaults to `None` → existing behavior untouched (44 computation_graph unit tests pass).
- Bridge `WasmReactorConstructor` impls `ReactorFireDecider` + exposes `evaluate(boundaries_json)`; the decision IS async (our trait, not the runtime's sync `process`), so it uses `spawn_blocking` into the WASM `evaluate` — faithful to task/trigger. `load_reactor_constructor(...)`. Config bound at load.
- Macro `#[constructor(kind = reactor)]`: author writes `fn evaluate(&self, boundaries_json: &str) -> Result<Option<String>, ConstructorError>` (`Some(ctx)` fires, `None` holds).
- e2e (`constructor_reactor_wasm.rs`, 4 PASS): `evaluate` bridge config-bound (fire/hold per gate) AND a live `Reactor::with_evaluator(<loaded>)` fires the graph only when the WASM guest says so.
- DEFERRED (reported, NOT faked): threading a reactor constructor through the CG SCHEDULER's package-load path. `scheduler.rs` builds `Reactor::new` from a `ReactorDeclaration`; nothing in the declaration/packaging types carries a reactor-constructor reference yet. Follow-up: add that reference to `ReactorDeclaration`/packaging, have `scheduler.rs::load_reactor` load it and `.with_evaluator(...)`. The seam it plugs into is landed + proven against `Reactor` directly; only the declaration/packaging/scheduler plumbing remains.

**Macro:** now generates task (T-0826) + accumulator + reactor (T-0828); `trigger` still errors (hand-authored vs raw fidius contract). accumulator/reactor take `#[config]`-only fields, payload arrives as the body argument (no output buffer / set/get / `#[param]`).

**Validation (all run):** `cargo fmt --all -- --check` exit 0; default `cargo check -p cloacina` clean + `cargo tree -p cloacina -i wasmtime` absent; `cargo check -p cloacina --features constructors-wasm --tests` clean; accumulator e2e 4 passed; reactor e2e 4 passed; trigger+macro regression pass; `cargo test -p cloacina --lib computation_graph::` 44 passed.

**Files:** `crates/cloacina/src/registry/loader/constructor_loader.rs`, `crates/cloacina/src/computation_graph/reactor.rs`, `crates/cloacina-macros/src/constructor_attr.rs`, `examples/constructor-contract/{accumulator,reactor}-constructor-fixture/`, `examples/constructor-contract/constructor-contract/src/lib.rs`, `crates/cloacina/tests/constructor_{accumulator,reactor}_wasm.rs`.