---
id: constructor-consumption-surface
level: task
title: "Constructor consumption surface — instantiate constructors in workflows (Rust + Python)"
short_code: "CLOACI-T-0829"
created_at: 2026-06-29T11:16:30.362953+00:00
updated_at: 2026-06-29T14:22:13.345728+00:00
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

# Constructor consumption surface — instantiate constructors in workflows (Rust + Python)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The surface for a workflow author to USE (consume) a constructor — the gap flagged during T-0827 planning (Python consumption, plus Rust). Today the framework loads + runs constructors (task/trigger) and the `#[constructor]` AUTHORING macro exists, but there is NO consumer surface in either language: a workflow author cannot yet declaratively wire a packaged constructor into a workflow.

**Scope:**
- **Rust consumer** — a declarative form (e.g. `constructor!(id=.., from="provider@ver", constructor="name", config={..}, dependencies=[..])`) inside `#[workflow]` that references a packaged constructor (a provider package from [[CLOACI-T-0827]]) as a primitive node — resolved + registered at build/load, executed by the runtime.
- **Python consumer (cloaca)** — the equivalent so a Python workflow can instantiate + wire a constructor. NOTE: execution is ALREADY language-agnostic (the Rust runtime runs the WASM constructor); this is purely the cloaca authoring/instantiation surface, not a new mechanism.
- `#[constructor]` for the **trigger / accumulator / reactor** authoring kinds (the macro currently errors for non-task), so the other primitives are authorable as constructors.

**AC:** a Rust `#[workflow]` references a packaged constructor (config + deps) and it runs end-to-end; a Python (cloaca) workflow does the same; `#[constructor(kind=trigger)]` authors a trigger constructor. May decompose (Rust / Python / macro-kinds) if large. Blocked by CLOACI-T-0827; accumulator/reactor kinds also blocked by CLOACI-T-0828.

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

### 2026-06-29 — Rust half landed (consumer surface + trigger kind)

Branch `i0132-t0829-consumer-surface`. The **Rust consumer surface** + the
`#[constructor(kind = trigger)]` authoring kind are implemented, built, and tested
end-to-end. The **Python/cloaca consumer surface is deferred** to a follow-on (noted
below). Not committed — pending review.

**Landed `constructor!` syntax** (inside `#[workflow]`):
```rust
constructor!(
    id = "greet",                    // DAG node id dependents reference
    from = "prefix@0.1.0",           // provider package (name[@version])
    constructor = "prefix",          // which constructor inside the provider
    config = { prefix = "hello, " }, // bound once at load (positional, see below)
    dependencies = ["load_user"],    // wired into the DAG like a #[task]
);
```

**How it resolves / registers / wires (the seam):**
- `cloacina-macros/src/workflow_attr.rs` parses `constructor!(...)` items inside the
  `#[workflow]` module, strips them from the emitted module, and folds their ids +
  deps into dependency-validation + cycle-detection (a `#[task]` may depend on a
  constructor node and vice-versa).
- Each node lowers to BOTH (mirroring `#[task]`): (1) added to the `Workflow` DAG in
  the embedded workflow constructor (topological planning + dep edges), and (2) a
  `TaskEntry` inventory submission so `Runtime::get_task` resolves it for execution.
  Both call new `cloacina::registry::loader::load_constructor_node(...)`, memoized in a
  per-site `OnceLock` to avoid re-loading the WASM component.
- `load_constructor_node` (in `registry/loader/constructor_loader.rs`, behind
  `constructors-wasm`) resolves `from` against a provider search-path directory, loads
  the named constructor via the existing packaged loader (`load_task_constructor`),
  validates the resolved `constructor.json` name == requested `constructor`, and wraps
  it as a `ConstructorNode` (overrides `id`/`dependencies`, delegates `execute`).
- **`from` resolution seam (flagged for confirmation):** ONE provider directory chosen
  by precedence: `set_provider_search_path()` override → `CLOACINA_PROVIDER_PATH` env →
  `./providers` default. Embedded-first (no registry service; a provider is a dir of
  unpacked packages, like `load_task_constructor`). `@version` is stripped (advisory;
  version pinning is a follow-on). Constructor-node lowering is EMBEDDED-only
  (`#[cfg(not(feature = "packaged"))]`); packaged-cdylib nodes are a follow-on.
- **config binding is POSITIONAL (flagged):** fidius binds config via bincode (not
  self-describing), so the macro emits `config` values as a TUPLE in written order —
  bincode-identical to the guest config struct PROVIDED the author lists them in the
  constructor's `#[config]` declaration order. The manifest carries `params`, not the
  `#[config]` schema, so name-keyed config is a noted follow-on.

**`#[constructor(kind = trigger)]`** (`constructor_attr.rs::expand_trigger`): mirrors the
task kind onto the trigger contract — method `poll`, wire `TriggerInvocation`/`PollOutcome`,
author body `fn poll(&self) -> Result<bool, ConstructorError>` (Ok(true) fires with the
`set` fire-context, Ok(false) skips). `#[param]` rejected; `#[config]`-only. The macro now
code-generates all four kinds.

**Validation (all run, all green):**
- `cargo check -p cloacina` clean; `cargo tree -p cloacina -i wasmtime` absent.
- `cargo fmt --all -- --check` exit 0.
- New `examples/constructor-contract/trigger-constructor-macro-fixture` builds to
  wasm32-wasip2 + emits its manifest on host.
- `cargo test -p cloacina --features constructors-wasm --test constructor_trigger_macro_wasm`
  — 3/3 (trigger constructor loads + fires/skips per config).
- `cargo test -p cloacina --features constructors-wasm --test constructor_workflow_node_wasm`
  — 1/1: a `#[workflow]` packages the macro fixture as a provider, wires it via
  `constructor!` between `load_user` and `notify`, runs to completion on embedded
  `DefaultRunner`/SQLite — `result == "hello, world"` visible to the dependent `#[task]`.
- Existing `constructor_macro_wasm` still 4/4 (no loader regression).

**Deferred (Python/cloaca consumer surface):** a cloaca authoring API mirroring
`constructor!` — instantiate a packaged constructor with id/from/constructor/config/
dependencies and add it to a Python workflow. Execution is already language-agnostic
(the Rust runtime runs the WASM constructor via `load_constructor_node`), so this is the
cloaca instantiation surface + binding the same provider-search-path seam.