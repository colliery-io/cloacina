---
id: python-cloaca-constructor
level: task
title: "Python (cloaca) constructor consumption surface"
short_code: "CLOACI-T-0831"
created_at: 2026-06-29T14:00:00.846110+00:00
updated_at: 2026-06-29T14:00:00.846110+00:00
parent: CLOACI-I-0132
blocked_by: ["CLOACI-T-0829"]
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Python (cloaca) constructor consumption surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The Python/cloaca half of the constructor consumption surface (deferred from [[CLOACI-T-0829]], whose Rust `constructor!` surface is landing). Add a cloaca API mirroring `constructor!` — id / from (provider) / constructor / config (name-keyed kwarg) / dependencies — so a Python workflow author can wire a packaged constructor into a workflow as a primitive node.

NOTE: execution is ALREADY language-agnostic — the Rust runtime runs the WASM constructor via `load_constructor_node`. This is purely the cloaca authoring/instantiation surface + binding the same provider-search-path seam (`CLOACINA_PROVIDER_PATH` / `./providers`). Mirror the Rust name-keyed config semantics.

**AC:** a Python (cloaca) workflow references a packaged constructor (config + deps) and runs end-to-end, config bound by name. Blocked by CLOACI-T-0829.

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

### 2026-07-02 — GROUNDED DESIGN (recon done; unblocked — T-0829/0836/0837 all landed)
**Everything below the Python surface is DONE + PROVEN** (T-0837 suite + T-0836 chain): `load_constructor_node(id, from, constructor, config_pairs, deps, GrantSpec)` resolves a provider member through wasmtime with grants; providers bundle via Cargo + `package_providers`; the reconciler resolves packaged Rust decls (Step 5b). Python rides all of it.

**KEY RECON FACT:** Python tasks register into a **scoped Runtime** — `runtime_scope::current_runtime()` → `rt.register_task(namespace, factory)` (`crates/cloacina-python/src/task.rs:667-674`, workflow.rs add_task) — the EXACT seam Step 5b uses for Rust constructor nodes. So the Python surface is thin:

**1. The cloaca API** (mirrors `constructor!`): `cloaca.constructor(id=, from_=, constructor=, config={...}, grants={...}, dependencies=[...])` — a PyO3 fn that (a) builds `GrantSpec::from_pairs`, (b) calls `load_constructor_node` (host-side; needs cloacina's `constructors-wasm`), (c) `rt.register_task(TaskNamespace(tenant, pkg, workflow, id), || node.clone())` into the CURRENT scoped runtime. The Python workflow's DAG assembly then sees it exactly like a `@task`. Config: dict → `Vec<(String, serde_json::Value)>` (name-keyed, same reorder-by-manifest binding underneath).
**2. Dual registration (⚠ memory: cloaca registers in TWO places):** add the fn to BOTH the maturin `#[pymodule]` (lib.rs) AND the synthetic `ensure_cloaca_module` (loader.rs) — the server uses the synthetic one. They drift; update both.
**3. Feature/weight decision:** `constructors-wasm` pulls wasmtime into cloacina-python. Server-embedded interpreter: the server binary already links it (fine). The standalone WHEEL: gate behind a cargo feature (default OFF for the wheel? or ship it — wasmtime adds ~20MB). DECISION NEEDED w/ human.
**4. Provider acquisition for PACKAGED Python consumers (no Cargo.toml!):** the compiler's Python branch skips cargo. Proposal: the compiler SYNTHESIZES a scratch Cargo.toml (`[dependencies] <from> = "<ver>"`) in a temp dir → `cargo metadata` resolves from crates.io/git → reuse `pack_providers` verbatim → `store_package_providers`. Discovery for Python: parse source for `cloaca.constructor(`+`from_="…"` (the `param_parse.rs` source-scan precedent) or a `[providers]` package.toml section (explicit, simpler). DECISION NEEDED.
**5. Reconciler Python branch:** the Rust branch's Step 5b reads FFI decls; Python has no cdylib. But if `cloaca.constructor()` registers the node DURING module import (the scoped-runtime load), NO reconciler change is needed for the embedded-interpreter path — the node is already in the scoped runtime when the workflow assembles. Needs `set_provider_search_path` before the Python module import (fetch+unpack `package_providers` rows in the PYTHON load path — mirror Step 5b's unpack, minus the FFI extraction).
**6. Demo:** a Python workflow consuming `cloacina-provider-fs` read_file+write_file (the session-goal Python half), embedded (`:memory:`) first, then packaged via the server lane.

**HUMAN DECISIONS (2026-07-02):** (3) → **ship wasmtime in the wheel** (`constructors-wasm` always on in cloacina-python; Python is core, no feature matrix). (4) → **explicit `[providers]` section in package.toml** (deterministic; doubles as the version pin; source refs validated against it).

### 2026-07-02 — (a)+(b) DONE: `cloaca.constructor()` LANDED + PROVEN (commit 5a7cb46d)
**The Python half of the session goal is demonstrated.** New `crates/cloacina-python/src/constructor.rs`: `cloaca.constructor(id=, from_=, constructor=, config={}, grants={}, dependencies=[])` — `current_workflow_context()` → config dict via `pythonize::depythonize` (written order, name-keyed) → grants dict → `GrantSpec::from_pairs` → deps (strings or task fns) → `py.allow_threads(load_constructor_node(...))` → `rt.register_task(TaskNamespace(tenant,pkg,workflow,id))` into the scoped Runtime. Registered in BOTH module surfaces (maturin `#[pymodule]` lib.rs + synthetic `ensure_cloaca_module` loader.rs, keep-in-sync notes). `constructors-wasm` ALWAYS-ON in cloacina-python (human decision).
**Tests `constructor_consumption.rs` 3/3 green:** Python workflow resolves `read_file` from the bundled `cloacina-provider-fs` and EXECUTES it (grant-gated sandbox read) · no grant fails closed at execute · unknown member raises ValueError naming the suite. Run: `cargo test -p cloacina-python --test constructor_consumption`.
**Learned:** WorkflowBuilder `__exit__` masks in-flight exceptions with its own empty-workflow validation error — a failed `cloaca.constructor` must be caught inside the block (or the workflow ends empty). Acceptable v1 semantics; documented in the test.

### 2026-07-02/03 — PACKAGED-PYTHON LANE IN PROGRESS (plan locked)
Recon: the Python import happens in `loading.rs` Python branch via `runtime.load_workflow_package(...)` (spawn_blocking, ~line 405) — providers must be staged (`set_provider_search_path`) BEFORE that call so `cloaca.constructor()` resolves during module import.
### 2026-07-03 — PACKAGED-PYTHON LANE CODE-COMPLETE (commit 915dcbad, all planned steps landed)
(1) `pack_providers_from_specs` ✅ (synthesized scratch project, specs verbatim — version/path/git uniform). (2) `CloacinaMetadata.providers: HashMap<String, ProviderDep>` ✅ (untagged Version|Detailed{version/path/git/tag/branch/rev} + `to_toml_value()`; re-exported; all struct literals patched). (3) compiler python arm ✅ (reads `[metadata.providers]` from the untyped manifest via serde → packs → `store_package_providers`; unbundleable fails the build). (4) reconciler ✅ — step-5b's fetch/unpack/set extracted into shared `stage_bundled_providers()` (returns count; fail-closed when bundled-but-no-feature); the PYTHON branch stages providers BEFORE `load_workflow_package` so `cloaca.constructor()` resolves during import. (5) tests ✅: provider_bundle 7/7 (new: from-specs path resolution + unknown-spec fail-closed), reconciler 25/25 (Step 5b through the shared helper), cloaca 3/3.

**REMAINING for full T-0831 close:** the LIVE packaged-Python demo through a real server (docker demo lane: compiler image needs the wasm32-wasip2 target; a Python `.cloacina` with `[metadata.providers]` + `cloaca.constructor` → compile → load → execute). Same live-verification bucket as the Rust compiler-service check in T-0836. Everything below it is code-complete + test-verified.
