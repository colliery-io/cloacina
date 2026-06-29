---
id: thread-reactor-constructors
level: task
title: "Thread reactor constructors through the CG scheduler package-load path"
short_code: "CLOACI-T-0830"
created_at: 2026-06-29T12:35:51.298733+00:00
updated_at: 2026-06-29T15:11:13.112433+00:00
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

# Thread reactor constructors through the CG scheduler package-load path

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Complete the reactor-constructor consumer path deferred from [[CLOACI-T-0828]]: thread a reactor-constructor reference from a packaged workflow's declaration through the CG scheduler so a reactor authored as a WASM constructor fires via the normal package-load path — not just via a direct `Reactor::with_evaluator(...)` test harness.

**What T-0828 landed (the mechanism, proven):** `Reactor::with_evaluator(decider)` + the `ReactorFireDecider` seam, and `WasmReactorConstructor` (the WASM `evaluate` bridge) — both proven against `Reactor` directly (a live evaluator gates firing).

**The gap:** `scheduler.rs` builds `Reactor::new` from a `ReactorDeclaration`, and nothing in the declaration / packaging / manifest types carries a reactor-constructor reference, so a packaged reactor constructor is never wired to its evaluator.

**Scope:** add a reactor-constructor reference to `ReactorDeclaration` (+ the packaging/manifest types that produce it), and have `scheduler.rs::load_reactor` load the constructor + call `.with_evaluator(<loaded>)`.

**AC:** a packaged workflow that declares a reactor constructor loads + fires via the scheduler end-to-end (config-bound, behind `constructors-wasm`). Blocked by CLOACI-T-0828 (done); relates to [[CLOACI-T-0827]] (packaging) + [[CLOACI-T-0829]] (consumer surface).

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

- [x] `ReactorDeclaration` carries a reactor-constructor reference
  (`ReactorConstructorRef { from, constructor, config }`), populated by the
  `#[reactor(from=.., constructor=.., config={..})]` authoring path via
  `ReactorRegistration`.
- [x] `scheduler.rs::load_reactor` loads the constructor (resolving `from` via the
  T-0829 provider seam, binding config BY NAME) and installs it via
  `Reactor::with_evaluator`; the resolved decider survives reactor restart.
- [x] E2E (gated `constructors-wasm`): a graph declaring a reactor constructor loads +
  fires via the scheduler — the WASM `evaluate` gates firing, config bound by name.
- [x] Behind `constructors-wasm` (default OFF): default `cargo check -p cloacina` clean,
  `cargo tree -p cloacina -i wasmtime` absent; the 44 CG unit tests stay green; the
  non-constructor `Reactor::new` path is unchanged when no ref is present.

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

### 2026-06-29 — Implemented + tested (ready for review; not committed)

**Gap closed:** a reactor authored as a WASM `#[constructor(kind = reactor)]` now
fires via the CG scheduler's package-load path, not just a direct
`Reactor::with_evaluator(...)` harness.

**`ReactorDeclaration` change:** added `constructor: Option<ReactorConstructorRef>`.
New `ReactorConstructorRef { from, constructor, config }` lives in the leaf crate
`cloacina-computation-graph` (always compiled — only String/serde_json::Value, no
wasmtime types) and is also carried on `ReactorRegistration`. Re-exported as
`cloacina::ReactorConstructorRef`.

**Authoring path chosen:** extended the `#[reactor]` macro with optional
`from` / `constructor` / `config = { name = value }` (both-or-neither validation;
`config` needs `constructor`). The macro lowers config to serde_json literals in the
LIB inventory submission (packaged submission carries None — provider resolution is
an embedded-host concern). Most consistent with T-0829: reuses its name-keyed config
+ provider resolution wholesale. Chose this over `constructor!(kind=reactor)` because
a reactor is the scheduler's firing engine, not a runtime-registered DAG node — it
needs the declaration→scheduler path (`#[reactor]`→`ReactorRegistration`→
`dispatch_runtime_reactors_into_scheduler`).

**Scheduler threading (`load_reactor`):** added a `constructor` param; calls the new
feature-gated `load_reactor_constructor_node` (in `constructor_loader.rs`, reusing
`bind_config_by_name` + `load_reactor_constructor`) on spawn_blocking → resolve ref →
`Arc<dyn ReactorFireDecider>`, then `.with_evaluator(..)`. Resolved decider stored on
`RunningGraph.evaluator` and re-applied on supervisor restart (no WASM re-load).
Resolution runs before any spawn (bad ref fails the load cleanly). Feature-OFF build
with a ref present fails closed. `load_graph` + `dispatch_runtime_reactors_into_scheduler`
pass the ref through; `load_graph_split` / FFI `dispatch_package_reactors_into_scheduler`
/ Python pass None.

**E2E (`tests/constructor_reactor_scheduler_wasm.rs`, gated `constructors-wasm`):**
stage `reactor-constructor-fixture` as a provider, build a declaration carrying the
ref (gate=5.0 bound BY NAME), `load_graph`, push boundaries via the normal
accumulator→reactor path: below-gate (1.0) does NOT fire, above-gate (9.0) DOES — the
WASM `evaluate` gates firing through the scheduler. A second test proves a
constructor-name mismatch fails the load closed.

**Validation (all green):** `cargo check -p cloacina` clean + `cargo tree -p cloacina
-i wasmtime` absent (present only with `--features constructors-wasm`); lib
`computation_graph::` 44 passed; CG integration 45 passed; `cloacina-macros reactor`
20 passed (5 new); e2e 2 passed; existing `constructor_reactor_wasm` 4 passed
(no regression); `cargo fmt --all -- --check` exit 0; `cargo check -p cloacina-python`
clean.

**Deferred (reported, not faked):** threading the ref through the FFI
`ReactorPackageMetadata` shape (Rust cdylib packaging) — needs new serialized fields
+ signing. Embedded/runtime inventory authoring path is complete; FFI packaged path
dispatches as a native dirty-flag reactor for now.

**Confirm:** `criteria` on a constructor-backed `ReactorDeclaration` is vestigial (the
WASM `evaluate` replaces it). Kept required in the macro for minimal churn, documented
as ignored-when-constructor-present. Flag if you'd prefer it optional.