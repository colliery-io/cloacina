---
id: constructor-provider-build-side
level: task
title: "Constructor provider build-side: resolve provider Cargo dep → build to wasm → bundle into the .cloacina (S-0015)"
short_code: "CLOACI-T-0836"
created_at: 2026-06-30T15:57:36.954974+00:00
updated_at: 2026-07-02T01:55:28.222984+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0132
---

# Constructor provider build-side: resolve provider Cargo dep → build to wasm → bundle into the .cloacina (S-0015)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0132]]

## Objective **[REQUIRED]**

Implement the build/distribution half of [[CLOACI-S-0015]] (decision [[CLOACI-A-0010]]): make a constructor **provider a normal Cargo dependency** that the consumer's build resolves → builds to a wasm component → **bundles into the `.cloacina`**, so packaged workflows are hermetic and the server resolves constructors against the bundled provider (no provider dir, no network).

This is the **unblock** for packaged constructors end-to-end: it lets a server load + run a `constructor!`-using workflow (the gate for an examples-based server test), and it lets [[CLOACI-T-0832]]'s held `step_load_constructor_nodes` resolve against the bundled provider.

**Scope (per the S-0015 decisions):**
- **Discovery**: collect every `from = "<crate>"` across the package's `constructor!`/`#[reactor]` declarations; map each to the matching `Cargo.toml` dependency; build+bundle ONLY those. Validate each is a real provider (`__constructor_manifest()` export).
- **`from` = the exact Cargo package name** as declared in `Cargo.toml` (no alias); `@version` optional, must be satisfiable by the resolved dep.
- **Locate** each provider crate via `cargo metadata` in the consumer's resolved dep graph (crates.io / path / git uniformly).
- **Build + pack** each via the existing `package_constructor_provider` flow (cargo build → wasm32-wasip2 → fidius pack).
- **Bundle** each as a nested fidius provider package under `providers/<crate>-<version>/` inside the `.cloacina`; record the `from`→bundled-dir map in the workflow manifest.
- (Fast-follow) cache built providers keyed on (crate, version, source, fidius interface hash).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: lets a packaged workflow USE constructors and deploy hermetically — the primary consumer window the whole feature targets.
- **Effort Estimate**: L (new build orchestration in cloacinactl/compiler + the .cloacina bundle format wiring).

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

### 2026-07-01 — STARTED: grounded design (T-0837 done unblocks this)
**Prereq state:** T-0837 fully landed for Rust/embedded — `package_constructor_provider` (build wasm → emit `provider.json` → fidius pack), the loader (`load_task_constructor` name-in-configure + `read_provider_manifest`), and `load_constructor_node` (`provider_search_path` resolution) all exist + green (29 wasm tests). The pieces T-0836 ORCHESTRATES already work; new work is discovery + bundle + wiring.

**Pipeline recon (hook points):**
- `crates/cloacina/src/packaging/mod.rs::package_workflow` = SOURCE packaging (packs the project dir; fidius builds on load). cloacinactl `package`.
- `crates/cloacina-compiler/src/build.rs::run_build` = COMPILER SERVICE: unpacks submitted source `.cloacina` → `cargo build` → **artifact = cdylib `Vec<u8>`** in the registry; server loads it. THIS is where cargo can resolve+build a provider dep (has source Cargo.toml + runs cargo).
- T-0832 already added the READ side: `packaging_bridge::call_get_constructor_metadata(handle)` + `package_loader::extract_constructor_metadata` → each node's `{from, constructor, config,...}` (FFI idx 10 `get_constructor_metadata`).

**DESIGN (S-0015/A-0010):** (1) after building the cdylib, load it + `call_get_constructor_metadata` → unique `from` set. (2) `cargo metadata --format-version 1` in the source dir → each `from` (exact pkg name, opt `@version`) → resolved manifest dir. (3) reuse `package_constructor_provider` per provider (validate real provider: `provider.json`). (4) unpack each under `providers/<crate>-<version>/` alongside the cdylib in the built ARTIFACT + record `from`→dir in the manifest. **⇒ compiler artifact grows from bare cdylib bytes into a bundle (cdylib + providers/ + manifest)** — the format decision that ripples to the server load path + T-0832's held `step_load_constructor_nodes` (which sets `provider_search_path` to the bundled `providers/`). (5) fast-follow: cache by (crate, version, source, fidius interface hash).

**BUILD ORDER:** (a) [THIS INCREMENT] additive core `packaging::provider_bundle`: `resolve_provider_crate(consumer_dir, from, ver?)->dir` (cargo metadata) + `bundle_providers(consumer_dir, from_list, dest)->Vec<BundledProvider>` (resolve→`package_constructor_provider`→unpack into `dest/providers/<crate>-<ver>/`), tested against a path-dep consumer of `cloacina-provider-fs`. (b) bundle/artifact format + manifest field. (c) compiler `run_build` wiring (discover via FFI → bundle → store). (d) T-0832 consumption: point `provider_search_path` at the bundle. (e) e2e: packaged Rust `constructor!` workflow loads+runs on the server. (f) Python (T-0831) reuses the bundle.

### 2026-07-01 — INCREMENT (a) DONE: `packaging::provider_bundle` (compiles green)
New module `crates/cloacina/src/packaging/provider_bundle.rs` (gated `constructor-packaging`, wasmtime-free — uses `fidius_core::package::unpack_package` directly, not the loader wrapper):
- `ProviderRef::parse("name[@version]")` (+ unit test).
- `resolve_provider_crate(consumer_dir, &ProviderRef) -> PathBuf`: shells `cargo metadata --format-version 1`, finds the package by exact name (+ advisory version prefix/equal filter), returns its crate dir. Path/git/crates.io uniform.
- `bundle_providers(consumer_dir, &[ProviderRef], dest, release) -> Vec<BundledProvider>`: de-dups by name, resolves → `package_constructor_provider` → `unpack_package` into `dest/providers/<name>-<ver>/`, reads back `provider.json` for authoritative name/version/members. `BundledProvider { from, crate_dir, provider_name, version, bundled_dir, constructors }`.
`cargo check -p cloacina --features constructor-packaging` ✅.

### ⚠️ KEY FINDING — Cargo-name vs provider-name must reconcile (blocks e2e)
`from` is resolved TWO ways: build-time = **Cargo package name** (via `cargo metadata`, A-0010); load-time = **fidius `[package].name`** = the provider name in `constructor_provider!(name=...)` (via `find_wasm_package`). For the PACKAGED path these MUST be the same string. The `fs-grant-constructor` fixture violates it (Cargo name `fs-grant-constructor` ≠ provider name `cloacina-provider-fs`) — fine for T-0837 embedded (which resolves purely by provider name) but breaks packaged resolution.
**RECOMMENDATION (increment b):** make `constructor_provider!` DEFAULT `name` to `env!("CARGO_PKG_NAME")` (the macro runs in the provider crate, so this is the provider's own Cargo name) and treat an explicit `name` as an override that SHOULD match; then RENAME the fs example crate dir + Cargo `name` to `cloacina-provider-fs` so Cargo-name == provider-name == the `cloacina-provider-<x>` convention. That unifies embedded + packaged `from` on ONE name and makes the fixture a correct convention exemplar (ripples: the 8 wasm tests' fixture dir/artifact refs + the demo — mechanical).

### NEXT: (a-test) a minimal consumer fixture (path-dep on the renamed provider) + integration test of resolve+bundle; (b) the name reconciliation above; (c) compiler `run_build` wiring (discover via `call_get_constructor_metadata` → `bundle_providers` → grow the artifact into a bundle); (d) T-0832 `provider_search_path`→bundle; (e) e2e packaged Rust `constructor!` on the server; (f) Python.
