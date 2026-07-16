---
id: native-provider-load-path-runtime
level: task
title: "Native provider load path — runtime discriminator + configure_in_process in the constructor loader"
short_code: "CLOACI-T-0902"
created_at: 2026-07-15T12:07:17.071770+00:00
updated_at: 2026-07-15T12:16:57.468823+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Native provider load path — runtime discriminator + configure_in_process in the constructor loader

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Add a NATIVE (in-process cdylib) load path to the constructor/provider loader alongside the WASM one: a provider package declares `runtime = "native"` and loads via fidius `PluginHandle::configure_in_process` / `load_library`+`from_loaded` instead of `load_wasm_configured`. The loaded handle is `fidius_host::PluginHandle` either way, so everything below the `Arc<PluginHandle>` wrappers is reused unchanged.

**Scope:** add a `runtime` discriminator to `ProviderManifest` (`cloacina-constructor-contract`, default `"wasm"`); emit `runtime = "native"` in `packaging/constructor_provider.rs::render_package_toml` (~:268); branch each of the 4 `load_*_constructor` fns (`constructor_loader.rs` :382/664/953/1157) on runtime → native path takes NO grants/egress (inert for native, I-0139 (e)).

**Acceptance:**
- [ ] A native cdylib provider package (`runtime = "native"`) loads via `configure_in_process` and its member answers `call_method` like the WASM path.
- [ ] WASM providers unaffected (`fs-grant-demo` still passes).
- [ ] Native load takes no capability grants; documented/asserted.

Parent: [[CLOACI-I-0139]]. First buildable/testable against a local native fixture (before the real publish home).

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

### 2026-07-15 — active, in progress

**Done (contract layer, verifiable):**
- Added `ProviderRuntime { Wasm (#[default]), Native }` enum + `grants_enforced()` to `cloacina-constructor-contract/src/lib.rs`, and a `#[serde(default)] pub runtime: ProviderRuntime` field on `ProviderManifest` (backward-compatible: pre-native `provider.json` deserializes as Wasm).
- Updated the two `ProviderManifest {..}` literals: the macro emitter (`cloacina-macros/src/constructor_provider.rs` → emits `ProviderRuntime::Wasm`; T-0903 flips to native at package time) and the packaging round-trip test (`Default::default()`).

**KEY FINDING — fidius gap (blocks the loader branch):** fidius 0.5.5 exposes `PluginHandle::configure_in_process(desc: &'static PluginDescriptor, cfg)` (configured, but STATICALLY-linked descriptor via `find_in_process_descriptor`) and `PluginHandle::from_loaded(LoadedPlugin)` (DYNAMICALLY dlopen'd via `load_library`, but constructs with EMPTY config — `construct_instance(plugin.descriptor, &[])`). A native PROVIDER is a *dynamically loaded* cdylib that *needs* config binding — neither call alone does both. `configured_cdylib_e2e.rs` only proves the static path.
- **Fix (small, additive, in `../fidius`):** add `CdylibExecutor::from_loaded_with_config(plugin, cfg)` (= `from_loaded` but `construct_instance(plugin.descriptor, cfg)` instead of `&[]`) + a `PluginHandle::configure_from_loaded(plugin, config)` wrapper. `LoadedPlugin.descriptor` is a `*const PluginDescriptor` kept alive by the loaded library; pure compose of existing internals, no ABI bump.

**Next:** add the fidius method; branch the 4 `load_*_constructor` fns on `read_provider_manifest(..).runtime`: Native → `load_library`+`configure_from_loaded` (member config via the same `wrap_member_config` bincode path), skip grant translation (advisory). WASM path unchanged.

**Interlock:** T-0902's own acceptance (a `--native` provider loads e2e) can only be exercised once T-0903 lands the `--native` build/emission path — the two verify together.

### 2026-07-15 — fidius side done, PR up (awaiting release)

- **fidius PR:** colliery-io/fidius#7, branch `feat/configure-from-loaded`. Adds `PluginHandle::configure_from_loaded` + `CdylibExecutor::from_loaded_with_config` + a `ConfiguredGreeter` in the `test-plugin-smoke` dylib + `configured_cdylib_dynamic_e2e.rs` (2 tests, green locally; `cargo clippy -p fidius-host` clean). Bumps fidius workspace 0.5.5 → **0.5.6**.
- **Blocking gate:** cloacina consumes `fidius-host = "0.5.5"` from crates.io (not a path dep). The cloacina loader branch + `cargo` bump to 0.5.6 waits on: fidius CI green → squash-merge → tag `v0.5.6` → `publish.yml` pushes 0.5.6 to crates.io.
- **Coupling confirmed while reading the loader:** the native `#[plugin_impl]` shell is emitted `#[cfg(target_arch = "wasm32")]`-gated today, so there is NO native plugin to `load_library` until T-0903 emits the non-wasm shell. ⇒ T-0902's cloacina loader branch and T-0903's native emission must land + verify TOGETHER against a native fixture. Plan: implement both after 0.5.6 publishes.

### 2026-07-15 — fidius PR #7: CI-drift fixes (maintainer chose "fix wasmtime in this PR")

fidius CI (last green June 30) had TWO latent drift failures unrelated to the feature, both from CI's unpinned `@stable`:
1. **clippy** `byte_char_slices` (1.97) on `arch.rs:136` → `*b"MZ"` (committed `1c76c24`).
2. **WASM lane** `macro_egress_e2e` — newer `wasm32-wasip2` stable emits `wasi:http@0.2.9` guests, host pinned wasmtime 45 provides 0.2.6 → instantiate fails. Proved by building `macro-fetcher` locally (rustc 1.96.1 → guests import `@0.2.6`, matching host; CI's newer stable → 0.2.9). **Fix (`dbe17d8`): bump wasmtime/wasmtime-wasi/wasmtime-wasi-http 45 → 46** (registers `wasi:http@0.2.12`); WASI 0.2 forward-compat ⇒ host is the ceiling, satisfies guests 0.2.6→0.2.12. `HOST_WASI_HTTP 0.2.6→0.2.12` + unit tests updated; fidius-guest vendored wit stays 0.2.6 (pin tripwire green, no re-vendor). 45→46 compiled with NO API breakage; full `-p fidius-host --features wasm` suite green locally (macro_egress 5/5). **Consequence for cloacina:** the fidius 0.5.6 cloacina consumes will carry wasmtime 46 — watch for a wasmtime bump ripple when bumping cloacina's `fidius = 0.5.6`.

fidius **v0.5.6 shipped**: merged (`e1aeb55`), tagged `v0.5.6`, published to crates.io.

### 2026-07-15 — cloacina spine landed (compile-verified), on branch `feat/i0139-native-kafka-provider` (off main, separate from PR #194/I-0105)

Commits: `5c6e5802` (ProviderRuntime + fidius 0.5.6 bump), `724d2fcf` (T-0903 native emission), `205d2dc9` (T-0902 native loader branch).
- **T-0903 emission DONE (compile-verified):** `constructor_provider!` `kind_shell` now emits each kind's shell TWICE — wasm (`crate=fidius_guest`, `cfg wasm32`) + native (`crate=fidius_core`, `cfg not(wasm32)`), mutually exclusive. Native shell → host cdylib plugin named `__Provider{Task,Trigger,Accumulator,Reactor}`.
- **T-0902 loader branch DONE for TASK (compile-verified under `--features constructors-wasm`):** `resolve_native_provider` (scans search path for a `runtime=native` provider.json — fidius's `find_wasm_package` is wasm-only) + `load_native_member` (`load_library` → select holder plugin → `configure_from_loaded`, no grants). Wired as a fast-path in `load_task_constructor`.

### 2026-07-15 (resume) — E2E build surfaced a real bug the compile-verified spine hid

Building the native fixture to satisfy T-0902's acceptance exposed that **`__constructor_make` (the per-member config decoder the shell's `configure` dispatches to) was `#[cfg(target_arch = "wasm32")]`-only** — it uses `#fidius_crate_ident::wire::deserialize` (fidius_guest). But T-0903's NATIVE shell (`crate = fidius_core`, `cfg not(wasm32)`) calls `__constructor_make` un-gated → a REAL native provider crate wouldn't compile. "Compile-verified" only meant the macro's own tokens compiled; `cargo check -p cloacina` never instantiates `constructor_provider!` on the host, so it couldn't catch it — exactly what the E2E is for.
**Fix (constructor_attr.rs, 3 sites — task/trigger/generic):** emit `__constructor_make` TWICE — wasm → `fidius_guest::wire`, native → `::fidius_core::wire` (which re-exports `fidius_guest::wire` verbatim, verified in fidius-core 0.5.6 lib.rs:25). Additive; the wasm variant is unchanged.

### 2026-07-15 (resume, cont.) — E2E GREEN (2/2); THREE bugs the spine hid, all fixed

Built a real native fixture (`examples/constructor-contract/native-task-provider-fixture` — same `#[constructor(kind=task)]`+`constructor_provider!` surface, host cdylib, `fidius-core`/`fidius-macro` host deps) + integration test `crates/cloacina/tests/constructor_provider_native.rs`. Getting it green surfaced THREE real bugs "compile-verified" never caught (none reachable without an actual native consumer crate):
1. **`__constructor_make` wasm-only** (above) — fixed.
2. **Vendored `examples/constructor-contract/constructor-contract` was STALE** — it lacked `ProviderRuntime` + the `runtime` field, but the macro emits `runtime: contract::ProviderRuntime::Wasm`. This silently broke EVERY fixture (wasm too) on rebuild — the wasm fixtures were "compile-verified" before the runtime-emission landed. Synced the vendored contract to `cloacina-constructor-contract` (added the enum + `#[serde(default)] runtime`).
3. **`constructor_provider!` emitted no `fidius_get_registry` export** — a native cdylib the loader `dlopen`s must export it (via dlsym) so `load_library` enumerates the `#[plugin_impl]` holders; the wasm path uses component exports, not dlsym. Added `#[cfg(not(wasm32))] ::fidius_core::fidius_plugin_registry!();` once per crate in the suite shell (`constructor_provider.rs`).

**PROOF:** `native_provider_task_loads_and_runs_in_process` — a `runtime="native"` provider loads via `load_library`+`configure_from_loaded`, `execute({name:"world"})` → `result == "native-world"` (configure-bound `prefix="native-"` + context param round-trip, all in-process). `native_provider_unknown_member_rejected` — fail-closed. **2/2 green.** T-0902 acceptance MET for the TASK kind.

**Still remaining:** other 3 kinds' native branches (accumulator native is the T-0904 prereq); native packaging path (T-0903 remainder: `--native` build + `render_package_toml` runtime stamping + `cloacinactl constructor package --native`). The macro + loader spine is now REAL-verified, not just compile-verified.

**REMAINING for T-0902/T-0903 (NOT done — honest bar):**
1. **E2E proof** — build a native task-provider FIXTURE (a minimal `#[constructor(kind=task)]`+`constructor_provider!` crate depending on `fidius-core`/`fidius-macro`, crate-type cdylib), package with `provider.json` `runtime=native`, load via `load_task_constructor`, assert `execute` round-trips. THIS is T-0902's acceptance; only compile-verified so far.
2. **Other 3 kinds** — trigger/accumulator/reactor native branches (mirror the task fast-path into their load fns; each wraps its own `Wasm*Constructor`). Reactor/trigger native are lower priority for kafka; **accumulator native is prerequisite for T-0904**.
3. **Native packaging path (T-0903 remainder)** — `packaging/constructor_provider.rs::render_package_toml` still hardcodes `runtime="wasm"` + `build_wasm_component` always `--target wasm32-wasip2`; need a `--native` build (host cdylib, no wasm target) that emits `runtime="native"` in package.toml + patches provider.json runtime. Plus `cloacinactl constructor package --native`.