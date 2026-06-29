---
id: operator-contract-manifest-schema
level: task
title: "Operator contract + manifest schema (macros emit it)"
short_code: "CLOACI-T-0822"
created_at: 2026-06-28T23:57:40.497369+00:00
updated_at: 2026-06-29T01:25:20.619493+00:00
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

# Operator contract + manifest schema (macros emit it)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Define the operator **contract**: a fidius interface per primitive kind + a manifest schema (operator name, version, **primitive kind** [task|trigger|accumulator|reactor], **param schema** reusing the I-0128 `InputSlot` descriptors, dependencies). Teach the existing `#[task]`/`#[trigger]`/`#[accumulator]`/`#[reactor]` macros (or a thin `operator` variant) to **emit** the contract — the way they already emit packaged-workflow metadata.

**AC:** a Rust/Python operator authored via the macros produces a valid contract (interface + manifest); the contract round-trips (author -> manifest -> read); the schema is language-neutral. Blocked by CLOACI-T-0821.

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

### 2026-06-28 — CONTRACT DESIGN + TASK-primitive first cut (PROVEN; not committed)

The operator CONTRACT is designed and the TASK primitive is implemented end-to-end
as a self-contained, workspace-excluded example under
`examples/operator-contract/` (3 crates, removable). All proofs green. The other
3 primitives + the real macro wiring are deferred (continuation below). NOT
committed — awaiting design review.

#### 1. Operator trait(s): per-primitive SYNC fidius `#[plugin_interface]`

DECISION (want confirmed): **one sync trait per primitive kind**, not a unified
trait. The runtime shapes differ (task: context→context; trigger: ()→fire?;
accumulator: event→buffer; reactor: criteria→fire?) and fidius's vtable is
positional, so a unified trait would carry mostly-`NotImplemented` methods like
`CloacinaPlugin` already does. Per-primitive traits keep each interface minimal,
its interface-hash tight, and the loader's descriptor selection unambiguous.

The async `cloacina_workflow` primitives (`Task`, `Trigger`, accumulator,
reactor) are NOT the WASM contract (they're `#[async_trait]`; the guest has no
runtime — T-0821). Each operator trait is the WASM-compatible SYNC analogue.

TASK trait (implemented, exact signature):
```rust
#[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
pub trait TaskOperator: Send + Sync {
    // JSON(TaskInvocation) in -> JSON(TaskOutcome) out. SYNC.
    fn execute(&self, invocation_json: String) -> String;
}
```
What crosses the boundary (DECISION — want confirmed): a **JSON String**, not a
serde struct. `TaskInvocation { context_json: String }` in,
`TaskOutcome { success, context_json, error }` out — the bytes are
`JSON(TaskInvocation)` / `JSON(TaskOutcome)`. This (a) exactly matches the
de-risked spike wire (String→String), (b) mirrors cloacina's existing
`TaskExecutionRequest`/`TaskExecutionResult` FFI which already carries the
`Context<Value>` as a JSON string, and (c) sidesteps any risk of fidius mapping
arbitrary nested serde structs across the component boundary. `(T,)` 1-tuple
calling convention from the spike holds: `call_method(0, &(inv_json,))`.

Projected sibling shapes (DEFERRED, for review of the pattern):
- `TriggerOperator::poll(&self, config_json: String) -> String` →
  `JSON(TriggerPollOutcome { fire: bool, context_json: Option<String> })`
  (the SYNC analogue of `Trigger::poll → TriggerResult`; `poll_interval` /
  `cron_expression` / `allow_concurrent` are manifest metadata, not per-call).
- `AccumulatorOperator::ingest(&self, event_json: String) -> String` →
  `JSON(AccumulatorOutcome { buffered: bool, emit: Option<String> })`.
- `ReactorOperator::evaluate(&self, cache_json: String) -> String` →
  `JSON(ReactorOutcome { fire: bool, fire_context_json: Option<String> })`.
All four are single-arg JSON-String-in/JSON-String-out, sync.

#### 2. The async↔sync bridge

cloacina's primitives stay async; the WASM call is a blocking host call into
wasmtime. The bridge is an async host adapter that wraps a configured operator
handle and awaits a `spawn_blocking` of the wasmtime call:
```rust
#[async_trait]
impl Task for WasmTaskOperator {           // host-side adapter (runtime task)
    async fn execute(&self, ctx: Context<Value>) -> Result<Context<Value>, TaskError> {
        let inv = serde_json::to_string(&TaskInvocation { context_json: ctx.to_json()? })?;
        let handle = self.handle.clone();   // configured PluginHandle (Send)
        let out_json = tokio::task::spawn_blocking(move || {
            handle.call_method(0, &(inv,))  // BLOCKING wasmtime call
        }).await??;
        let outcome: TaskOutcome = serde_json::from_str(&out_json)?;
        if outcome.success { Context::from_json(outcome.context_json.unwrap()) .. }
        else { Err(TaskError::..(outcome.error.unwrap())) }
    }
}
```
Config binds ONCE at load via `load_wasm_configured` (the `configure` hook +
macro-emitted `fidius-configure` export); per-call `execute` dispatches on the
already-configured persistent store. This adapter is the seam where the operator
plugs back into the existing executor unchanged. (Implementing the live adapter
is part of the runtime-integration task, not this contract task; the test here
exercises the same serialize → blocking-call → deserialize sequence inline.)

#### 3. Manifest schema (`OperatorManifest`)

cloacina-defined metadata that travels in the fidius package as a sidecar
`operator.json` (NOT a call payload). The loader (T-0823) reads it to learn which
primitive to register and how. Implemented Rust type:
```
OperatorManifest {
  name, version,
  primitive_kind: PrimitiveKind { Task | Trigger | Accumulator | Reactor },
  interface: String,         // kebab fidius interface, links the descriptor
  interface_version: u32,    // must match descriptor.interface_version
  params: Vec<InputSlot>,    // CLOACI-I-0128 param schema (reused)
  dependencies: Vec<String>,
  description, author,
}
```
`interface` + `interface_version` point the loader at the right
`<Trait>_WASM_DESCRIPTOR`; `primitive_kind` selects which sibling descriptor to
link; `params` reuses I-0128 `InputSlot` (JSON-Schema-typed) for the injectable
runtime surface. In the example `InputSlot` is a structural copy of
`cloacina_api_types::InputSlot` so the example stays self-contained/removable;
the real manifest reuses the canonical type verbatim.

#### 4. Macro emission

DECISION (want confirmed): a dedicated **`#[operator(kind = task, ...)]`**-flavored
attribute rather than overloading `#[task]`/`#[trigger]`/etc. Rationale: an
operator targets a SEPARATE sync trait + a WASM cdylib + a manifest sidecar,
whereas `#[task]` emits an async `Task` impl + inventory entry into a packaged
workflow. The two share the metadata-emission SEAM (the same place
`generate_packaged_registration` builds `PackageTasksMetadata` /
`get_task_metadata`), so `#[operator]` would: (a) emit the per-primitive sync
`#[plugin_interface]`/`#[plugin_impl]` (guest side), and (b) emit an
`OperatorManifest` constructor at the metadata seam (analogous to today's
`get_task_metadata`) that the build writes to `operator.json` — reusing the
existing I-0128 `make_params_fn` InputSlot codegen for `params`. Keeping it
separate avoids forking the (already CG-heavy) `#[task]` codegen and keeps the
operator interface-hash decoupled from the workflow ABI. Open sub-question for
review: a unifying `#[operator(kind=...)]` vs four thin attrs
(`#[task_operator]` …) — leaning to the single `kind`-parameterized attribute.

How this composes with existing packaged-workflow metadata: operators are a
PARALLEL package shape, not a change to `CloacinaPlugin`. A workflow package
keeps emitting `PackageTasksMetadata` via `CloacinaPlugin`; an operator package
emits its own per-primitive interface + `operator.json`. The loader branches on
`runtime = "wasm"` + presence of `operator.json` to take the operator path.

#### Implemented (first cut — TASK only) vs deferred

IMPLEMENTED & PROVEN (`examples/operator-contract/`, 3 standalone crates):
- `operator-contract/` — shared contract: `PrimitiveKind`, `OperatorManifest`
  (+`to_json`/`from_json`), `InputSlot`, `TaskInvocation`, `TaskOutcome`. 2 unit
  tests green.
- `task-operator-fixture/` — sync `TaskOperator` `#[plugin_interface]` +
  `#[plugin_impl(config=Config)]`; reads `name`, writes `result = prefix+name`;
  builds to a **VALID** wasm32-wasip2 component (117 KB).
- `host/tests/task_operator_contract.rs` — **3 tests pass**:
  (1) manifest round-trips (produced → `operator.json` → read) and carries
  `primitive_kind = Task`; (2) configured operator `load_wasm_configured` +
  invoke → `result == "hello, world"`, original context preserved;
  (3) missing required input → clean failed `TaskOutcome` (no trap).

DEFERRED (continuation):
- Trigger / Accumulator / Reactor sibling traits + wire types (shapes sketched
  above) — next once the task shape is confirmed.
- Real macro wiring: the `#[operator(kind=...)]` attribute in `cloacina-macros`
  emitting interface + `operator.json` at the metadata seam (this task proved the
  hand-written shape; the spike + this contract de-risk the codegen target).
- The live async host adapter (`WasmTaskOperator: Task`) wiring into the executor
  — belongs to the runtime-integration task; bridge sequence proven inline here.
- Python operator path (componentize-py) — carried from T-0821, still unexercised.
- Loader (T-0823) consuming `operator.json` to register primitives — unblocked by
  this schema.

Sharp edges: benign `unexpected cfg "host"` warning from the fidius macro
(cosmetic, as in T-0821); wasm fixture must be a standalone `[workspace]`; host
crate cold-compiles wasmtime (~heavy). NOT committed — design review gate.