---
id: packaged-workflow-constructor
level: task
title: "Packaged-workflow constructor support (constructor! beyond embedded)"
short_code: "CLOACI-T-0832"
created_at: 2026-06-29T14:00:00.991521+00:00
updated_at: 2026-06-30T14:10:42.698611+00:00
parent: CLOACI-I-0132
blocked_by: [CLOACI-T-0829]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Packaged-workflow constructor support (constructor! beyond embedded)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`constructor!` currently lowers **embedded-only** (`cfg(not(packaged))`): a packaged (cdylib) workflow doesn't link the constructor loader, so it can't reference constructors. Extend support so packaged workflows can also use `constructor!`.

The Rust consumer surface ([[CLOACI-T-0829]]) wires constructors into the DAG + runtime for embedded workflows (DefaultRunner). The gap is the packaged cdylib path. Lift: make the constructor loader (or a thin shim) available in the packaged-workflow runtime and lower `constructor!` nodes there too.

**AC:** a packaged (.cloacina) workflow using `constructor!` loads + runs its constructor node end-to-end. Additive to embedded-first, not a replacement. Blocked by CLOACI-T-0829.

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

### 2026-06-30 — design (mirrors the packaged REACTOR metadata path)
Human steer: **packaged is the PRIMARY consumer window** for constructors (embedded already works, T-0829/T-0834). Packaged support — Rust first, then Python rides the same package mechanism — is the real next step. Grants (T-0834: `GrantSpec`/`from_pairs`/`translate`/`load_constructor_node(grants)`) plug straight in.

**Current state:** `constructor!` lowers embedded-only — `constructor_node_load_block` (calls `load_constructor_node`) is `cfg(not(packaged))` (workflow_attr.rs). Packaged mode = "a noted follow-on" (workflow_attr.rs:521): a cdylib doesn't link the wasm loader.

**The exact analog — packaged REACTOR metadata (already works):**
- macro emits `ReactorEntry` inventory; `package!()` shell collects them.
- cdylib exposes FFI `get_reactor_metadata()` (method idx 4) → `Vec<ReactorPackageMetadata>` (cloacina-workflow-plugin).
- server: `package_loader.rs:475 extract_reactor_metadata` → `packaging_bridge::call_get_reactor_metadata(&handle)` → installs it. SERVER links wasmtime; cdylib does NOT.

**Design (constructor analog):** cdylib DECLARES constructor nodes as metadata; SERVER resolves them.
1. `ConstructorPackageMetadata` + `ConstructorEntry` inventory type in cloacina-workflow-plugin — `{ workflow, id, from, constructor, config: Vec<(String,JSON)>, grants: Vec<(String,Vec<String>)>, dependencies: Vec<String> }` (grants in the raw `from_pairs` shape).
2. macro packaged arm (workflow_attr.rs): emit `ConstructorEntry` inventory per `constructor!` node in packaged mode. Also surface `ReactorConstructorRef.grants` into `ReactorPackageMetadata` (reactor FFI half).
3. `package!()` shell: new FFI `get_constructor_metadata()` (new method idx) → `Vec<ConstructorPackageMetadata>`.
4. packaging bridge: `call_get_constructor_metadata(&handle)` (mirror reactor).
5. package loader (package_loader.rs): `extract_constructor_metadata` + at workflow registration call `load_constructor_node(id, from, constructor, config, deps, GrantSpec::from_pairs(grants))` → `Arc<dyn Task>` → inject into the rebuilt workflow DAG.
6. grants enforcement rides for free (same load_constructor_node → translate → fidius two-key).
7. packaging (cloacinactl) + a PACKAGED example (mirror fs-grant-demo, built as .cloacina + server-loaded) + tests.

**Open Qs (confirm before building):**
- Provider resolution at the SERVER: packaged `from = "read_file@0.1.0"` must be on the server's `CLOACINA_PROVIDER_PATH` — bundle into the .cloacina, or resolve from the server provider dir? (Reactor constructors already resolve server-side via provider_search_path — reuse.)
- New FFI method idx → plugin interface-version bump? (mirror how get_reactor_metadata was added.)

**Phasing:** P1 metadata type + macro packaged emission · P2 FFI export + bridge + loader resolution · P3 packaged example + tests + reactor FFI grants. One PR each.

### 2026-06-30 — P1a DONE: metadata data shape (compiles clean)
Added to `cloacina-workflow-plugin`: `ConstructorEntry` (inventory_entries.rs — `{ workflow, id, from, constructor, config: fn()->Vec<(String,Value)>, grants: fn()->Vec<(String,Vec<String>)>, dependencies: fn()->Vec<String> }`, `inventory::collect!`) + `ConstructorPackageMetadata` (types.rs — owned/serde projection). Both re-exported from lib.rs. `cargo build -p cloacina-workflow-plugin` clean. fmt clean. Mirrors ReactorEntry/ReactorPackageMetadata exactly.

### 2026-06-30 — P1b + P2-core DONE
**P1b (macro packaged emission):** `build_constructor_inventory_entries` (workflow_attr.rs) now emits, per node, BOTH the embedded `TaskEntry` (`cfg(not packaged)`, unchanged) AND a packaged `ConstructorEntry` (`cfg(feature="packaged")`) carrying workflow/id/from/constructor + config (lowered via `::cloacina_workflow_plugin::serde_json::json!`) + grants (raw pairs) + deps. Added `pub use serde_json;` to cloacina-workflow-plugin so the packaged cdylib needs no direct serde_json dep. cloacina-macros + plugin compile clean.

**P2-core (ABI bump + shell):** `CloacinaPlugin` interface **version 3 → 4**; added `get_constructor_metadata() -> Vec<ConstructorPackageMetadata>` at **method index 10**, `#[optional(since = 4)]` (older plugins → NotImplemented → host treats as "no constructor nodes"). Added the `package!()` shell body (walks `inventory::iter::<ConstructorEntry>` → `ConstructorPackageMetadata`). Legacy per-workflow `_WorkflowPlugin` impl is DEAD (`let _ = packaged_registration;` @ workflow_attr.rs:632 — never emitted), so no update needed there. cloacina-workflow-plugin compiles clean; cloacina host build verifying.

**RELEASE NOTE (owed):** CloacinaPlugin FFI v3→v4 — every packaged workflow plugin must be rebuilt to load against this server. (Auto-rebuild signal = [[CLOACI-T-0835]], slated this release.)

**REMAINING (P2 cont. + P3):**
- `packaging_bridge::call_get_constructor_metadata(&handle)` (mirror `call_get_reactor_metadata`, packaging_bridge.rs:129 — method index 10).
- `package_loader::extract_constructor_metadata` (mirror `extract_reactor_metadata`, package_loader.rs:475).
### 2026-06-30 — P2 bridge + loader DONE; registration-wiring design found
- `METHOD_GET_CONSTRUCTOR_METADATA = 10` (cloacina-workflow-plugin lib.rs).
- `packaging_bridge::call_get_constructor_metadata(&handle)` (NotImplemented→empty fallback).
- `package_loader::extract_constructor_metadata` (mirrors extract_reactor_metadata).
- cloacina builds clean (default + constructors-wasm) with the v4 interface + bridge + loader.

**Registration-wiring design (the "deep part" — turns out CLEAN):** packaged workflows are assembled by `create_workflow_from_host_registry_static` (reconciler/loading.rs:1161), which enumerates the **runtime task registry** for the package/workflow and `add_task`s each. So no DAG-assembly change is needed — the wiring is:
1. Add `constructors: Vec<ConstructorPackageMetadata>` to `PackageLoadView`; populate in `build_view_rust` via `extract_constructor_metadata` (Python `build_view` → empty for now).
2. A new reconciler step `step_load_constructor_nodes`: for each entry, `load_constructor_node(id, from, constructor, config, deps, GrantSpec::from_pairs(grants))` → `runtime.register_task(TaskNamespace::new(tenant, package, workflow, id), move || node.clone())`. Run BEFORE `create_workflow_from_host_registry_static` so the existing assembly picks them up.
3. Provider resolution: set the server's `provider_search_path` (reuse the reactor-constructor server path — the open Q resolves to "server provider dir, not bundled").
Constructor nodes resolve + run SERVER-SIDE (host-side `Arc<dyn Task>` wrapping the wasm handle), distinct from the FFI-dispatched packaged tasks — both coexist in the one workflow via the registry.

### 2026-06-30 — RE-SCOPED by [[CLOACI-A-0010]] (provider distribution = Cargo deps)
Human reframed provider resolution as the **distribution layer**, decided it = Cargo's dependency model (crates.io + path/git), independently versioned, build-time resolve+build+**bundle into the .cloacina** → hermetic package, dumb server. Captured: ADR [[CLOACI-A-0010]] (decided) + spec [[CLOACI-S-0015]] (discovery, the build/distribution mechanics).

**Impact on T-0832:**
- **STANDS (done, green):** P1a metadata types · P1b macro packaged `ConstructorEntry` emission · P2 FFI v3→v4 `get_constructor_metadata` + shell · bridge `call_get_constructor_metadata` · loader `extract_constructor_metadata`. These carry the constructor DECLARATION through the package regardless of provider sourcing — unaffected.
- **HELD (would be throwaway):** the final `step_load_constructor_nodes` server-side resolution — do NOT wire it against an external `CLOACINA_PROVIDER_PATH`. Per A-0010 it must resolve against the **bundled** provider in the unpacked package. So this step waits on S-0015's bundle format + the build-side that puts the provider in the package.
- **MOVED OUT:** the build-side (cloacinactl/compiler resolve provider Cargo dep → build wasm → bundle) is its own work under [[CLOACI-S-0015]] — NOT T-0832.

**REMAINING for T-0832 (once S-0015 bundle format exists):** `step_load_constructor_nodes` resolving the bundled provider + `PackageLoadView.constructors` field · packaged example (.cloacina with a bundled provider + server load) + tests · reactor `grants` into `ReactorPackageMetadata` (packaged reactor arm drops ref to None — reactor_attr.rs:474).

**NET:** T-0832 is paused pending the S-0015 distribution/build work; its plumbing (P1/P2) is in and green.
