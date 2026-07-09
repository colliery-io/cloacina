---
id: constructor-loader-registry-load
level: task
title: "Constructor loader + registry (load_wasm_configured -> register the primitive)"
short_code: "CLOACI-T-0823"
created_at: 2026-06-28T23:57:41.833766+00:00
updated_at: 2026-06-29T01:48:57.400152+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Constructor loader + registry (load_wasm_configured -> register the primitive)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Extend cloacina's loader to load a WASM operator via `load_wasm_configured`, read its contract/manifest, and **register the configured primitive** into the `Runtime` registries (task/trigger/accumulator/reactor) — the dynamic analog of the macro (the metadata->register path already exists for packaged workflows via the `CloacinaPlugin` interface).

**AC:** loading an operator package registers the declared primitive(s) with bound config; routing correct (a trigger-operator registers as a trigger, etc.); errors fail-closed + clear. Blocked by CLOACI-T-0822.

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

### 2026-06-28 — TASK-operator loader + executor bridge: PROVEN end-to-end (not committed)

A WASM **task operator** now loads + runs as a cloacina `Task`, end-to-end,
behind a default-OFF feature. NOT committed — review gate.

#### Contract promoted to a real crate
`crates/cloacina-operator-contract` (new workspace member): `OperatorManifest`,
`PrimitiveKind`, `TaskInvocation`, `TaskOutcome`, `METHOD_EXECUTE`,
`TASK_OPERATOR_INTERFACE_VERSION`, and the `TaskOperator` sync-trait shape
(documented; the `#[plugin_interface]` declaration itself is bound to a fidius
`crate =` target so it lives host-/guest-side, not here). Reuses the **canonical**
`cloacina_api_types::InputSlot` (re-exported), not a vendored copy. serde-only
deps -> wasm-buildable. 2 unit tests + 1 doctest green.

#### Loader + executor adapter (feature `operators-wasm`, default OFF)
`crates/cloacina/src/registry/loader/operator_loader.rs`:
- Host-side `#[fidius_macro::plugin_interface(version=1, crate="fidius_core")]
  trait TaskOperator` -> emits `__fidius_TaskOperator::TaskOperator_WASM_DESCRIPTOR`.
- `load_task_operator(search_path, package_name, &config) -> Arc<dyn Task>`:
  reads sidecar `operator.json`, requires `primitive_kind == Task` +
  matching `interface_version` (else fail-closed `LoaderError`), then
  `load_wasm_configured(component, &config)` (config bound ONCE at load) and
  wraps the handle.
- `WasmTaskOperator: Task` — the async/sync bridge: `Context.to_json()` ->
  `JSON(TaskInvocation)` -> `tokio::spawn_blocking` the wasmtime `call_method(0,
  (inv,))` -> parse `JSON(TaskOutcome)` -> `Context::from_json` on success, or a
  `TaskError::ExecutionFailed` from a failed outcome. Handle held in `Arc` so a
  `'static + Send` clone goes into `spawn_blocking`; the WASM backend serializes
  calls behind its own store mutex.

Feature wiring: `operators-wasm = ["dep:fidius-macro",
"dep:cloacina-operator-contract", "fidius-host/wasm"]`. fidius-host has no
default features, so the default cloacina build pulls neither wasmtime nor
fidius-macro.

#### Gating proof (no-wasmtime-by-default)
- `cargo tree -p cloacina -i wasmtime` -> "did not match any packages" (absent).
- `cargo tree -p cloacina --features operators-wasm -i wasmtime` -> `wasmtime
  v45.0.3` via `fidius-host`. Symmetric for `fidius-macro`.
- `cargo check -p cloacina` (default) -> Finished, clean.

#### End-to-end (feature on)
`crates/cloacina/tests/operator_loader_wasm.rs` (reuses the proven fixture
`examples/operator-contract/task-operator-fixture`, built to wasm32-wasip2,
staged as `.wasm` + `package.toml` + `operator.json`):
- `wasm_task_operator_runs_as_cloacina_task` — load -> `Arc<dyn Task>` -> `execute`
  `Context{name:"world"}` -> `result == "hello, world"`, `name` preserved.
- `config_binds_at_load_so_instances_differ` — `hello, ` vs `goodbye, `
  instances differ.
- `non_task_primitive_fails_closed` — a Trigger-kind manifest is rejected.
- `cargo test -p cloacina --features operators-wasm --test operator_loader_wasm`
  -> `3 passed; 0 failed`. (Benign `unexpected cfg "host"` warning from the
  fixture's wasm build only; the loader module suppresses it.)

#### Deferred (unchanged scope)
- **CLOACI-T-0824**: trigger `poll` / accumulator `ingest` / reactor `evaluate`
  bridges + wire types, and wiring a loaded operator into the Runtime/scheduler
  registries (this task returns a `Arc<dyn Task>`; it does not register it).
- **CLOACI-T-0826**: the `#[operator(kind=...)]` authoring macro (manifests are
  hand-built in the test; the macro emits them at the metadata seam).
- The async `CloacinaPlugin` packaged-workflow path is untouched (parallel shape).
- Generic config across arbitrary guest structs stays out of scope: fidius
  config wire is bincode (the dev=JSON/release=bincode note is stale — fidius
  is always bincode now), so the loader is generic `<C: Serialize>` and the test
  passes a concrete `Config{prefix}` matching the guest.