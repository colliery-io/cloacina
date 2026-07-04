---
id: constructor-provider-build-side
level: task
title: "Constructor provider build-side: resolve provider Cargo dep → build to wasm → bundle into the .cloacina (S-0015)"
short_code: "CLOACI-T-0836"
created_at: 2026-06-30T15:57:36.954974+00:00
updated_at: 2026-07-04T03:34:18.024573+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

### 2026-07-01 — INCREMENT (b) DONE: name reconciliation (commit 309e87ac)
`constructor_provider!` `name` is now OPTIONAL, defaulting to `env!("CARGO_PKG_NAME")` (emitted in the provider crate → its own Cargo name). Provider name (load-time `find_wasm_package`) can no longer drift from the Cargo package name (build-time `cargo metadata`). Explicit `name` still overrides. Renamed `examples/constructor-contract/fs-grant-constructor` → `cloacina-provider-fs` (Cargo==provider name == the `cloacina-provider-<x>` convention), dropped its explicit `name` to exercise the default, updated demo/docs/packaged-test refs. **Verified:** `emit_manifest` → `"cloacina-provider-fs"`; `constructor_provider_package_wasm` 5/5 green; `fs-grant-demo` runs (granted read / denied read / granted write via 2nd member). Other 7 wasm test files untouched (explicit-name codegen path unchanged; not fs-related).

### 2026-07-01 — INCREMENT (a-test + version reconciliation) DONE (commit 32a2a0b7)
Mirrored the name fix for VERSION: `constructor_provider!` `version` now optional, defaults to `env!("CARGO_PKG_VERSION")` — so a consumer's `from = "x@<ver>"` pin matches the provider crate's CARGO version (what `cargo metadata` reports), not just the provider.json version. Bumped the fs example Cargo version 0.0.0→0.1.0; dropped its explicit name+version (both default from the crate now). Added `examples/constructor-contract/provider-consumer-fixture` (Cargo-deps the provider) + `tests/provider_bundle.rs` — first REAL-dep-graph coverage: resolve via cargo metadata, unknown + version-pin-mismatch fail closed, `bundle_providers` builds→lays down `providers/<name>-<ver>/` (both members + wasm), dup refs build once. **5/5 green; packaged test still 5/5.** ⇒ the entire build-side CORE (resolve + build + bundle) is done + verified against a real Cargo graph.

### SERVER-SIDE PLUMBING RECON (for increment c — the remaining big lift)
- **Built-artifact storage** = `dal.upsert_artifact(name, version, target, digest=sha256(cdylib), bytes)` (`compiler/loopp.rs`) → `package_artifacts` table, PER-TARGET cdylib bytes. But constructor providers are **wasm = architecture-NEUTRAL**, so bundling them per-target duplicates them. ⇒ decision: store providers ONCE per (name,version) (arch-neutral) vs bundle-into-each-target-artifact (simpler, duplicates).
- **Server load path** = `registry/reconciler/loading.rs::load_package`: writes `loaded_workflow.package_data` (an ARCHIVE) to temp → **unpacks** → dlopens the cdylib (`write_and_dlopen` @ loading.rs:64 via `fidius_host::loader::load_library`). **This unpack point is exactly where `set_provider_search_path(<unpacked>/providers)` belongs** — IF the unpacked archive carries a `providers/` tree. So the cleanest wiring may be to bundle providers into the archive that `load_package` already unpacks (the arch-neutral `package_data`), not the per-target cdylib blob. Need to confirm what `package_data` is (source vs built) + how it relates to `package_artifacts`.
- `step_load_constructor_nodes` (T-0832's held step) is NOT yet in `loading.rs` — it's a planned hook; likely lands next to the dlopen where the runtime registry is populated, calling `set_provider_search_path` then resolving each constructor node via `load_constructor_node`.

### 2026-07-02 — BUILD-SIDE CHAIN PROVEN E2E (library level) — commit 305e0488
`tests/packaged_constructor_e2e.rs` (constructors-wasm) ties the whole build side into ONE flow against the real consumer fixture: resolve `cloacina-provider-fs` from its Cargo graph → build to wasm → `bundle_providers` into `providers/` → `set_provider_search_path` → `load_constructor_node("read_file")` (what `constructor!` lowers to) → RUN with an fs grant (reads the file). + fail-closed: no grant denies the read (WASI preopen denial), unknown member names the suite. **3/3 green.** ⇒ everything from a Cargo dep to a running, grant-gated, sandboxed constructor node works; the ONLY remaining gap is the server/registry STORAGE that carries the `providers/` bundle from compiler-build to the server load path.

### 2026-07-02 — INCREMENT (c) EXECUTION PLAN (locked; sqlite-verifiable, in progress)
**Recon complete — every hook verified in code:**
- `package_artifacts` = the template: sqlite migration 027 / postgres 031, unified `schema.rs` table block (DbUuid/DbBinary/DbTimestamp), models.rs `PackageArtifact`/`New…`, DAL `upsert_artifact` (delete+insert txn, `dispatch_backend!` both backends) in `dal/unified/workflow_packages.rs:803`.
- Next migration numbers: **sqlite 028, postgres 039** (independent numbering).
- Rust load path (`reconciler/loading.rs`): `loaded_workflow.compiled_data` = the compiler-built cdylib; steps 1–5 (cron/triggers/reactors/CGs) then **Step 6 `step_load_workflows`** → `register_package_tasks` (dlopen + registrar) → `register_package_workflows` → `create_workflow_from_host_registry_static` (loading.rs:1161) which **enumerates `runtime.task_namespaces()` filtered by (package, workflow, tenant)** — so constructor nodes registered under `TaskNamespace(tenant, pkg, workflow, id)` BEFORE step 6 are picked into the DAG automatically (deps included). This is exactly the held T-0832 design.
- `extract_constructor_metadata` (package_loader.rs:529) + `call_get_constructor_metadata` (packaging_bridge.rs:148, NotImplemented→[]) are UNGATED; only `load_constructor_node` needs `constructors-wasm`.
- Packaged cdylibs emit `ConstructorEntry` DECLARATIONS only (workflow_attr.rs:995 — no TaskEntry in packaged mode), so server-side injection is REQUIRED for packaged constructor workflows.

### 2026-07-02 — INCREMENT (c) STEPS 1–4 DONE + 2 REAL T-0832 FFI BUGS FIXED (commits 04282434, 0f59ecf5)
**Storage (04282434):** migrations sqlite 028 + postgres 039 `package_providers`; schema.rs block; models `PackageProvider/New…`; DAL `upsert_provider`/`get_providers_for_package` (both backends); `WorkflowRegistry` trait default-empty `get_package_providers` + DB-backed override; reconciler **`step_load_constructor_nodes`** (Step 5b, before workflow build) — extract FFI decls → fetch+unpack bundled providers → `set_provider_search_path` → `load_constructor_node` per decl (spawn_blocking) → `runtime.register_task(TaskNamespace(tenant,pkg,workflow,id))`. Fail-closed both ways (no feature / no bundle).
**E2E PROVEN (0f59ecf5):** new `packaged-consumer-fixture` (packaged cdylib w/ `constructor!` + downstream task, `__WORKSPACE__` template) + 2 tests in loading.rs `mod constructor_nodes` (gated constructors-wasm): the happy path EXECUTES the resolved node (reads the granted file through the bundled+sandboxed provider) + decls-without-bundle fails closed. **2/2 green; all 25 reconciler tests green.** Run: `cargo test -p cloacina --features constructors-wasm --lib constructor_nodes`.
**⚠️ TWO REAL LATENT T-0832 BUGS caught by this first live exercise, both FIXED:**
1. **vtable order**: fidius vtable = IMPL-BLOCK order; host METHOD_* constants = TRAIT order. The shell had `get_constructor_metadata` at impl position 5, silently shifting methods 5–10 for every v4-plugin cdylib. Moved to END of impl (slot 10) + warning comment in the shell. RULE: new plugin-shell methods must be APPENDED, never inserted.
2. **wire type**: `ConstructorPackageMetadata.config` carried `serde_json::Value` — bincode FFI can't deserialize it (`deserialize_any`). Now `Vec<(String, String)>` w/ JSON-encoded values; shell encodes (`v.to_string()`), host `serde_json::from_str`s back before binding.

### 2026-07-02 — (5) COMPILER WIRING DONE (commit 34a1237f) → **T-0836 CODE PATH COMPLETE**
`provider_bundle` gains `discover_provider_refs` (source scan anchored on `constructor!`/`#[reactor(` → `from = "…"`; unanchored strings ignored — 2 unit tests) + `pack_providers` (resolve→build→PACKED archives) + `PackedProvider`; `ProviderPackageResult` gains `provider_version`. `WorkflowRegistryImpl::store_package_providers` (sha256 hash, one row per provider). Compiler `run_build` Rust-success arm: discover → `pack_providers` (spawn_blocking) → store; declared-but-unbundleable provider FAILS the build (fail closed). Compiler's cloacina dep gains `constructor-packaging`.

**THE FULL CHAIN NOW EXISTS END-TO-END:** author (suite macro) → package → compiler discovers+bundles → `package_providers` → reconciler Step 5b unpacks+resolves → the node RUNS. Verified this session: discovery 3/3, provider_bundle integration 5/5, reconciler 25/25 incl. Step 5b e2e 2/2 (node executes, reads granted file), compiler check green, both DB backends green.

### 2026-07-03 — LIVE-DEMO ENABLEMENT (in progress)
- **Server + agent now link the loader**: `constructors-wasm` EXPLICIT on both `cloacina-server` and `cloacina-agent` cloacina deps (was only transitive via cloacina-python's always-on wheel decision — fragile). Compile checks running.
- **Compiler images get the wasm target**: `rustup target add wasm32-wasip2` in `docker/Dockerfile.compiler` AND `Dockerfile.demo`'s `workspace` stage (the demo compiler/fixtures runtime).
- **`packaged-consumer-fixture` is now canonical A-0010**: the provider is a real Cargo dep (`__WORKSPACE__` path), so the compiler's source-scan→cargo-metadata flow resolves it (the reconciler e2e test re-verifies with the dep present).
- **NEW demo fixture `examples/fixtures/demo-constructor-rust`** + packer entry in `pack-demo-fixtures.sh`: workflow `constructor_demo` — `reader` = `cloacina-provider-fs`/`read_file` with `grants={fs=["ro:/etc"]}` reading `/etc/os-release` (present in every container), downstream `summarize` task. Compiles staged (host, `__WORKSPACE__`→repo). The demo stack (`docker compose -f docker/docker-compose.demo.yml up --build` / `angreal ui up`) now exercises the FULL live chain: harness uploads → compiler discovers+bundles the provider (wasm target in-image) → server resolves Step 5b → sandboxed grant-gated execution visible in the UI.

### 2026-07-03 — LIVE VERIFICATION DEFERRED (human decision; local Docker disk)
Attempted `angreal ui up` twice. **Positive signal: the full workspace release build COMPILED inside the demo image** (incl. server/agent with constructors-wasm + the new fixture); both attempts died on `No space left on device` in the Docker VM — consumed by the user's unrelated running containers (40GB writable) + images. Freed everything cloacina-owned (12.3GB build cache + ~6GB stale fleet images); not enough headroom for the 3 workspace images. Human chose to DEFER the live run rather than grow the VM disk / prune their stack. **Everything is code-complete + locally test-verified (50+ green tests)**; the live demo-stack run (which now carries `demo-constructor-rust`) is the single remaining item and will also exercise in CI/demo lanes. To run later: free ~15-20GB in Docker (or enlarge the VM disk) → `angreal ui up` → watch compiler logs for "bundling constructor providers" → `constructor_demo` executes in the UI (sandboxed /etc/os-release read).

### 2026-07-04 — LIVE RUN: 6 of 7 stages verified; store blocked by VM disk (definitive)
Five live attempts on the demo stack. **VERIFIED LIVE, each stage from real logs:** (1) fixtures packed `demo-constructor-rust.cloacina` ✓ (2) harness uploaded + compiler claimed ✓ (3) cdylib built in-container ✓ (4) **`bundling constructor providers … ["cloacina-provider-fs"]` — live discovery** ✓ (5) provider wasm build in-container ✓ (6) pack ✓ → (7) `store_package_providers` INSERT fails: `could not extend file: No space left on device` — the Docker VM (62.7GB) is at absolute zero at build-peak; the shortfall is single-digit MBs after reclaiming 25+GB (build caches ×2, ~11GB stale images, compiler layers ×2, all optional services stopped). NOT a code failure.
**THIRD real bug caught by the live lane (FIXED, commit 2bb2a2ec):** `build_wasm_component` ignored `CARGO_TARGET_DIR` (the compiler image sets a shared /workspace/target) → artifact not found at `<crate>/target`. Now honors the env (relative→cwd semantics). Bridged the running container via symlink to verify the rest of the chain live.
**RESUME RUNBOOK (one user action + one command):** grow the Docker Desktop disk (Settings→Resources, +20GB) → `docker compose -f docker/docker-compose.demo.yml up -d --no-build` → `psql: update workflow_packages set build_status='pending', build_error=null where package_name='demo-constructor-rust'` → the compiler (NOTE: image still has the pre-fix binary; the container symlink `/workspace/examples/constructor-contract/cloacina-provider-fs/target → /workspace/target` must be re-created after any container recreation, OR rebuild the image to pick up 2bb2a2ec) → watch `package_providers` for the row → server reconcile tick does Step 5b → `constructor_demo` executes. Stack left STOPPED (compiler/harness/kafka) with images built; postgres/server/dex still up.

### 2026-07-04 — LIVE RUN CONTINUED: store ✓, Step 5b ✓ (twice), execution reached the sandbox; 2 NEW findings
With the disk bumped (87GB): **build succeeded, `package_providers` row stored (cloacina-provider-fs 0.1.0, 85KB)**, and the server's **Step 5b ran live twice**: `Unpacked 1 bundled provider(s)` + `Registered packaged constructor node public::demo-constructor-rust::constructor_demo::reader`. Fired `constructor_demo` via the API.
**FINDING #4 — AGENTS CANNOT EXECUTE CONSTRUCTOR NODES (real follow-on):** with `CLOACINA_DEFAULT_EXECUTOR=fleet`, the reader task dispatched to an agent, whose load path (task_registrar "host-managed approach") does NOT run Step 5b and whose registry has no provider bundles (the fleet protocol doesn't ship `package_providers`) → `"task …reader not registered after loading package (registered: [summarize])"`. NEEDS: (a) provider-bundle delivery to agents (fleet protocol or server API fetch) + (b) constructor-node resolution in the agent load path. Until then, constructor workflows require in-process execution (`default` executor).
**FINDING #5 — WASI symlink fail-closed (demo bug, GOOD security behavior):** with the in-process executor, the node EXECUTED in the sandbox; the read of `/etc/os-release` failed `Operation not permitted` because it's a SYMLINK → `/usr/lib/os-release`, outside the `ro:/etc` grant — the sandbox correctly refusing a path escape. Fixture updated to read `/etc/hostname` (regular bind-mounted file) + bumped to 0.1.1.

### 2026-07-04 — CLOSING
The build/distribution half of S-0015 is fully shipped + live-verified (see the 7/7 entry below). Residuals dispositioned: **agent/fleet execution → [[CLOACI-T-0838]]** (filed, with the live evidence); **provider build cache** — S-0015 fast-follow, revisit when provider counts make cold builds hurt; **unload doesn't unregister constructor nodes** — documented caveat in `step_load_constructor_nodes` docs, acceptable v1 (nodes drop with the runtime); **semver-req matching for `@version` pins** → [[CLOACI-T-0833]]. Five real integration findings from the live lane, all fixed or filed. COMPLETE.

### 2026-07-04 — 🏁 FULL LIVE CHAIN VERIFIED 7/7 (fresh stack, project `cloacina-demo`)
After a full volume reset (stale pre-upgrade volume was the ABI-200 noise source; fresh seed built ALL packages clean): `demo-constructor-rust 0.1.2` (hostname fixture + E0502 fix, commit 6ca2518b) uploaded via the API → compiler claim → cdylib build → provider discovery+bundle → **`package_providers` row (0.1.2, 85157 bytes)** → server Step 5b (`Registered packaged constructor node …constructor_demo::reader`) → **execution `0dc34eb5` `Completed`** with final context `{"contents":"2605abd458ca\n","sandbox_read_bytes":13,"sandbox_read_hostname":"2605abd458ca"}` — the container hostname read from `/etc/hostname` INSIDE the WASM sandbox through the `ro:/etc` grant, from the provider bundled at compile time. In-process executor (`CLOACINA_DEFAULT_EXECUTOR=default` via compose override — the fleet/agent leg is finding #4's follow-on). Compose project renamed `cloacina-demo` (b5d652ad).

**STILL OPEN (verification + polish, not code):** (a) LIVE compiler-service verification (docker demo lane — needs the wasm32-wasip2 target in the compiler image + a demo package with a crates.io/git provider dep; path deps inside submitted archives don't resolve in the compiler's temp dir); (b) migration 028 run-verified only implicitly (the reconciler test stubs the registry) — a DAL round-trip test or any sqlite `DefaultRunner` boot exercises it; (c) unload does not unregister constructor nodes (documented caveat); (d) provider build cache (fast-follow per spec); (e) PYTHON consumer surface (T-0831) — reuses this entire chain.

**ORIGINAL IMPLEMENTATION ORDER (for reference; each kept green):**
1. **Migrations** `create_package_providers` (sqlite 028 + postgres 039): id, package_name, version, tenant_id NULLable, provider_name, provider_version, content_hash, provider_data(BLOB=the packed provider `.cloacina` archive), created_at; UNIQUE(package_name, version, COALESCE(tenant_id,''), provider_name).
2. **schema.rs** table block + **models.rs** PackageProvider/NewPackageProvider + **DAL** `upsert_provider` (mirror upsert_artifact) + `get_providers_for_package(name, ver, tenant) -> Vec<PackageProvider>`.
3. **Reconciler `step_load_constructor_nodes(metadata, library_data, tenant)`** inserted between Step 5 and Step 6 in the Rust branch: extract constructor metadata (FFI idx 10; empty → fast return) → fetch provider rows → write each archive to temp + `unpack_provider_archive` into `<work>/providers/` → `set_provider_search_path` → per entry `load_constructor_node(id, from, constructor, config, deps→TaskNamespace(tenant,pkg,workflow,dep), GrantSpec::from_pairs(grants))` → `runtime.register_task(ns, move || node.clone())`. Gate the resolution body `#[cfg(feature = "constructors-wasm")]`; if metadata non-empty and feature OFF → hard error (fail closed). KNOWN CAVEATS (document): global `provider_search_path` races concurrent loads (reconciler is sequential today); constructor nodes registered directly on the runtime are not unregistered by the task-registrar unload path (follow-on).
4. **e2e fixture + sqlite test**: packaged consumer workflow crate (cdylib, `packaged` feature, `constructor!(from="cloacina-provider-fs", constructor="read_file", …)` + a downstream `#[task]`); TEST stages it (rewrites the provider path-dep to an absolute path in a temp copy), cargo-builds the cdylib, stores compiled_data + `bundle_providers`-produced provider archives via DAL, `load_package`, asserts the workflow assembled with the constructor node + downstream task (and ideally executes it via DefaultRunner sqlite).
5. **Compiler wiring** (code + compile-check; LIVE verification is the docker demo lane): in `run_build` for Rust packages, `discover_provider_refs(source_dir)` (source scan anchored on `constructor!(`/`#[reactor(` → `from = "…"`) → `bundle_providers` (keep the packed archives) → `upsert_provider` per row. Needs `cloacina-compiler`'s cloacina dep to add the `constructor-packaging` feature (serde-only, no wasmtime). NOTE: path-dep providers inside a submitted archive won't resolve in the compiler's temp dir — crates.io/git providers are the real compiler-service story; path deps are the local/test story.

### (superseded) NEXT (increment c — the ONLY remaining piece; pure server/registry infra, needs a focused pass w/ a DB):
1. RESOLVED: `package_data` = the SOURCE archive (unpacked in `load_package` for the manifest); the cdylib comes from the per-target `package_artifacts` blob (compiler-built). ⇒ compiler-built wasm providers have NO home today. **Two options for (c):** (A) a NEW arch-neutral `package_providers` blob per (name,version) — DAL table + migration + reconciler fetch + unpack → `set_provider_search_path`; or (B) the compiler re-packs source+`providers/` into a "built package_data" that `load_package` already unpacks (no new table, but changes package_data semantics from source→built). Lean (A) — cleaner separation, arch-neutral, doesn't overload package_data; ~1 migration + DAL method + one fetch/unpack in the load path. This is where the next focused pass starts.
2. Compiler `run_build`: after cargo-building the cdylib, load it + `call_get_constructor_metadata` → unique `from` set → `provider_bundle::bundle_providers` → attach the `providers/` tree to the stored package.
3. `load_package`: `set_provider_search_path(<unpacked>/providers)` before node resolution (the held T-0832 step).
4. e2e: a packaged Rust workflow crate (`constructor!` + a real `cargo` dep on `cloacina-provider-fs`) loads+runs on the server.
5. Python (T-0831) reuses it all.

### (superseded) earlier NEXT (increment c is the big one — artifact/bundle format):
- **(c) compiler `run_build` wiring:** after cargo-building the consumer cdylib, load it + `call_get_constructor_metadata` → unique `from` set → `provider_bundle::bundle_providers` into a `providers/` tree; grow the stored artifact from bare cdylib `Vec<u8>` to a bundle (cdylib + providers/ + a manifest field mapping from→dir). **This artifact-format change is the next real decision** — where the bundle lives, how registry storage + the server load path carry it.
- **(d) T-0832 consumption:** `step_load_constructor_nodes` calls `set_provider_search_path(<bundle>/providers)` before resolving nodes.
- **(e) e2e:** a packaged Rust `constructor!` workflow (a consumer example with a real `cargo` dep on `cloacina-provider-fs`) loads+runs on the server. This same consumer unblocks the deferred `provider_bundle` integration test (resolve+bundle against a real Cargo dep).
- **(f) Python (T-0831):** reuses the identical bundle + loader; the lift is the Python `constructor!`-equivalent authoring/consumer surface in cloaca.
