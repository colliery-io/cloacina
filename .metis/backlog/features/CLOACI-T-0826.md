---
id: operator-authoring-instantiation
level: task
title: "Operator authoring + instantiation surface (Rust + Python)"
short_code: "CLOACI-T-0826"
created_at: 2026-06-28T23:57:47.816546+00:00
updated_at: 2026-06-29T02:31:37.483810+00:00
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

# Operator authoring + instantiation surface (Rust + Python)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The **instantiation ergonomics**: how a workflow author instantiates an operator with config and wires it into a workflow, in **Rust and Python**.

**AC:** a Rust workflow and a Python workflow each instantiate a built-in operator with config and run it; the surface is documented; params validated at instantiation against the contract schema. Blocked by CLOACI-T-0824 (can parallel CLOACI-T-0825).

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

### 2026-06-28 — `#[operator]` authoring macro (Rust), TASK kind end-to-end

Implemented the **authoring** half of this ticket: a `#[operator(kind = task, name, version)]` proc-macro in `cloacina-macros` that lets an author write the CLEAN operator form and generates the RAW fidius contract the loader already consumes. (The consumer-side **instantiation** ergonomics — `operator!(...)` in Rust + the Python surface from this ticket's original objective — remain the deferred continuation; see below.)

**Author surface** (`examples/operator-contract/task-operator-macro-fixture`):
```rust
#[operator(kind = task, name = "prefix", version = "0.1.0", contract = operator_contract, ...)]
pub struct Prefix {
    #[config] prefix: String,        // bound once per instance at load
    #[param(required)] name: String, // declared input, pulled from the task context
}
impl Prefix {
    fn execute(&self) -> Result<(), OperatorError> {   // the ONLY thing the author writes
        self.set("result", format!("{}{}", self.prefix, self.name));
        Ok(())
    }
}
```

**What the macro generates** (vs the hand-written `task-operator-fixture`):
- the SYNC `#[plugin_interface] TaskOperator` trait (tokens mirror the host's `crate = "fidius_core"` re-declaration verbatim so the fidius interface hash matches) + a `#[plugin_impl(config = <generated Config>)]` impl whose `execute(invocation_json) -> outcome_json` decodes the `TaskInvocation`, pulls the `#[param]` fields out of the context (required/optional honored), binds the `#[config]` fields, runs the author body, and encodes the `TaskOutcome`;
- the `configure` hook binding the `#[config]` fields once at load;
- a `#[config]`-derived `Config` struct + `set`/`get` over an output buffer (interior-mutable, so the `&self` body can write outputs);
- `pub fn __operator_manifest() -> OperatorManifest` carrying `primitive_kind = Task`, `name`, `version`, `interface`, `interface_version`, and `params: Vec<InputSlot>` (required `String` → `{"type":"string"}` slot). The macro cannot write a file; **packaging (CLOACI-T-0827) serializes this to the sidecar `operator.json`**.

**Host/wasm split:** the fidius guest glue is emitted under `#[cfg(target_arch = "wasm32")]`, so the same crate compiles on the host with only the struct + `__operator_manifest()`. That lets a host `emit_manifest` bin materialize `operator.json` from the generated fn (the T-0827 stand-in) without the wasm-only exports.

**Proof** — `crates/cloacina/tests/operator_macro_wasm.rs` (`--features operators-wasm`), mirroring T-0823: builds the macro fixture to a wasm32-wasip2 component, writes `operator.json` from `__operator_manifest()`, loads via the cloacina loader → `Arc<dyn Task>`, runs with `Context { name: "world" }` → asserts `result == "hello, world"`; plus config-binding, missing-required-param fail-closed, and a manifest-shape assertion. **4/4 pass.** Default `cargo check -p cloacina` clean, `cargo tree -i wasmtime` empty (no wasmtime in the default build), `cargo fmt --all` clean.

Also added a tiny serde/wasm-safe `OperatorError` (`Result<(), OperatorError>` body return) to both `cloacina-operator-contract` and the example contract crate.

**Deferred (noted continuation):**
- **Instantiation ergonomics** — `operator!(...)` workflow instantiation in Rust and the Python authoring/instantiation surface (this ticket's original objective). Params-validated-at-instantiation rides on that surface.
- **Other kinds** — `kind = trigger | accumulator | reactor`. Each maps cleanly onto the same shape (own sync trait + `poll`/`ingest`/`evaluate` body, own `*Invocation`/`*Outcome` wire); the trait/body mapping is documented in `operator_attr.rs`. Full codegen + fixtures deferred (their host bridges are themselves a T-0824 continuation). `#[operator]` currently errors clearly for non-task kinds.
- **Richer param schema** — slots use a built-in scalar→JSON-Schema map (schemars-free, so the manifest fn stays wasm-buildable); wiring the full I-0128 `schema_for::<T>()` is a refinement.