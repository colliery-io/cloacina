---
id: native-provider-emission-macros
level: task
title: "Native provider emission — macros emit native fidius glue + cloacinactl constructor package native build path"
short_code: "CLOACI-T-0903"
created_at: 2026-07-15T12:09:17.289739+00:00
updated_at: 2026-07-16T02:34:19.042275+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Native provider emission — macros emit native fidius glue + cloacinactl constructor package native build path

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Make `#[constructor]`/`constructor_provider!` emit NATIVE fidius `#[plugin_impl]` glue (host-loadable via `configure_in_process`) as an alternative to the current `#[cfg(target_arch = "wasm32")]` guest glue — the author surface (`#[config]` + body fn) is unchanged; only the emitted glue's target flips. And grow `cloacinactl constructor package` a native build path parallel to the WASM one.

**Scope:** `cloacina-macros/src/constructor_attr.rs` + `constructor_provider.rs` — emit the native plugin_impl glue (gate wasm-glue behind `not(native)` / emit both); `cloacinactl/src/nouns/constructor/mod.rs` + `packaging/constructor_provider.rs` — a `--native`/runtime-aware build that builds crate-type cdylib for the native host target (DROP `--target wasm32-wasip2`), emits `runtime = "native"`, packs the `.cloacina` with the cdylib artifact.

**Acceptance:**
- [ ] A `#[constructor(kind = task)]` crate builds to a native cdylib provider via `cloacinactl constructor package --native` and loads through the T-0902 native path end-to-end (native `read_file` twin of `cloacina-provider-fs`).
- [ ] The WASM emission + `wasm32-wasip2` build path is unchanged.

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0902]] (native load path).

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

### 2026-07-15 — activated; scoped. EMISSION half already DONE via T-0902; only the cloacinactl PACKAGING path remains.

**(A) Native EMISSION (macros) — effectively COMPLETE (done under T-0902's work + prior 724d2fcf).**
- `constructor_provider!` + `#[constructor]` already emit BOTH shells: wasm (`crate=fidius_guest`, `cfg wasm32`) and native (`crate=fidius_core`, gated `#[cfg(all(not(target_arch="wasm32"), feature="native"))]`). `__constructor_make` emitted for both wire codecs; native cdylib exports `fidius_get_registry` via `fidius_plugin_registry!()`. Author surface unchanged.
- **PROVEN**: `native-task-provider-fixture` builds host cdylib, loads+runs E2E for TWO kinds (task+accumulator) — `constructor_provider_native` 4/4 green; `constructor_provider_package_wasm` 5/5 confirms WASM emission unchanged. (A)'s acceptance met.
- **The `native` cargo feature is the opt-in contract**: native provider crate declares `features=["native"]` (default-on) + `fidius-core`/`fidius-macro` host deps. This is the authoring shape (B) + docs must teach.

**(B) cloacinactl `constructor package --native` — NOT DONE (the real remaining work).**
Current `crates/cloacina/src/packaging/constructor_provider.rs` is WASM-hardwired:
- `ProviderPackageOptions` has no runtime/native flag.
- `build_wasm_component` always `cargo build --lib --target wasm32-wasip2`, locates `target/wasm32-wasip2/<profile>/<crate>.wasm`.
- `render_package_toml` hardcodes `runtime="wasm"` + `[wasm] component=..`.
- `package_constructor_provider` leaves `provider.runtime` at serde default (Wasm).

**Plan for (B):**
1. Add `runtime: ProviderRuntime` (or `native: bool`) to `ProviderPackageOptions` (+ `new_native`).
2. `build_native_cdylib(crate_dir, release)` — `cargo build --lib --features native` (NO `--target`, host triple), locate `target/<profile>/lib<crate>.{dylib|so|dll}` (honor `CARGO_TARGET_DIR`; host has no target-triple subdir). Reuse `crate_name`.
3. Branch `package_constructor_provider` on runtime: native → build cdylib, set `provider.runtime=Native`, `provider.component=<dylib file>`.
4. `render_package_toml` → `runtime="native"` + a `[native]` section (confirm shape vs fidius — see UNKNOWN).
5. `cloacinactl/src/nouns/constructor/mod.rs` — add `--native` flag to `package`, thread into options.
6. E2E: native `read_file` twin of `cloacina-provider-fs`, packaged via new path, loaded through `load_task_constructor_from_package`, assert execute round-trip (acceptance #1).

**GOOD NEWS — load-from-archive already works.** `load_task_constructor_from_package` (constructor_loader.rs:608) `unpack_provider_archive` → delegates to `load_task_constructor(dest,..)`, which auto-takes the native fast-path (`resolve_native_provider` scans the unpacked dir for `runtime=native` provider.json). NO loader change needed for packaged native load — works the moment the archive unpacks with a correct provider.json + dylib.

**⚠️ BLOCKING UNKNOWN (investigate FIRST next session):** does `fidius_core::package::pack_package`/`unpack_package` faithfully round-trip a `package.toml` with `runtime="native"` and a NON-wasm component (`.dylib`), WITHOUT a `[wasm].component` section? If fidius pack requires `[wasm]`, native packing fails → needs a fidius change (T-0902 pattern repeating: a fidius gap only a real consumer surfaces). **Read `~/.cargo/registry/src/*/fidius-core-0.5.6/src/package.rs`** (`pack_package`/`unpack_package`/manifest parse) before writing (B). Could not read this turn — Bash safety classifier was down (opus unavailable); use Read on the resolved path next time.

**Also verify:** `native` feature must be ON during packaging `cargo build --lib`; run `emit_manifest` with `--features native` for consistency.

### 2026-07-15 — BLOCKING UNKNOWN RESOLVED: fidius 0.5.6 needs NO change. Two-manifest model confirmed.

Read `fidius-core-0.5.6/src/package.rs`:
- **fidius `package.toml` runtime vocabulary is `rust` / `python` / `wasm`** (`PackageRuntime`), NOT `native`. `runtime_strict()` REJECTS unknown values — so writing `runtime = "native"` in package.toml would ERROR at pack time. A native cdylib is fidius's DEFAULT **`runtime = "rust"`** (cdylib + PluginRegistry). `validate_runtime()` for Rust rejects both `[wasm]` and `[python]` sections and requires neither.
- **`pack_package`** (`:560`) → `load_manifest_untyped` (validates runtime) → `collect_archive_files` walks the ENTIRE dir (skips `target`/`.git`/`.sig`), tars EVERY file. `unpack_package` (`:632`) restores all. ⇒ provider.json + the `.dylib` round-trip faithfully with no per-file manifest reference needed.
- **cloacina bypasses fidius's package loader for native**: `load_native_member` calls `load_library(dir.join(provider.component))` directly, reading `component` from cloacina's `provider.json` (ProviderManifest), whose `runtime` field is the `wasm`/`native` discriminator. So the fidius package.toml `[wasm]`/component section is irrelevant to the native load path.

**⇒ TWO-MANIFEST MODEL (the crux of (B)):**
- `package.toml` (fidius): native → `runtime = "rust"` (or omit), NO `[wasm]` section, keep `[metadata] category=constructor, primitive_kind=..`.
- `provider.json` (cloacina): native → `runtime = "native"` + `component = "<lib….dylib>"`.

**(B) is UNBLOCKED — refined plan:** `render_package_toml` gains a runtime arg → emit `runtime="rust"` + drop `[wasm]` for native; `package_constructor_provider` branches build (cdylib vs wasm) + sets `provider.runtime=Native`+`component=<dylib>`; add `runtime`/`native` to `ProviderPackageOptions`; `build_native_cdylib` = `cargo build --lib --features native` (host triple, `target/<profile>/lib<crate>.{dylib|so|dll}`). No fidius bump, no loader change.

### 2026-07-15 — (B) IMPLEMENTED + E2E GREEN → `d0f7a67e`. T-0903 DONE.

Implemented exactly per the refined plan:
- `packaging/constructor_provider.rs`: `ProviderPackageOptions.runtime` (+ `new_native()`); `package_constructor_provider` branches build (`build_wasm_component` vs new `build_native_cdylib`), stamps `provider.runtime=Native`+`component=<dylib>`; `render_package_toml(runtime)` → `runtime="rust"` + NO `[wasm]` for native (`fidius_runtime_str`); `emit_manifest_json` runs `--features native` for native. `ProviderRuntime` re-exported from the packaging module (cloacinactl names it without a contract-crate dep — [[feedback_macro_generated_deps_invisible]]).
- `cloacinactl constructor package --native`: new flag → `opts.runtime`. Module + fn docs updated for the two-manifest model.
- Fixed the field addition across all 3 non-test call sites (provider_bundle ×2, reconciler/loading) + 4 wasm test literals (all default to Wasm — behavior unchanged).

**ACCEPTANCE — both MET:**
- [x] A `#[constructor(kind=task)]` crate builds to a native cdylib provider via the `--native` path AND loads through the T-0902 native path E2E. **PROOF** `constructor_provider_native_package.rs` (needs `constructor-packaging`+`constructors-wasm`): packages the native fixture via `package_constructor_provider(new_native)` → pack → `load_task_constructor_from_package` (unpack → native fast-path → `load_library`+`configure_from_loaded`) → `execute({name:"world"})` → `"native-world"`. **1/1 green.** (Reused `native-task-provider-fixture`, itself a kind=task native provider, rather than authoring a separate `read_file` twin — proves the identical thing more directly.)
- [x] WASM emission + `wasm32-wasip2` build path unchanged — `constructor_provider_package_wasm` **5/5** + packaging unit tests **3/3** (incl. new `native_package_toml_uses_rust_runtime_and_no_wasm_section`).

**No fidius change, no loader change** — the two-manifest model (fidius package.toml `runtime="rust"` cdylib + cloacina provider.json `runtime="native"`) fit fidius 0.5.6 as-is; contrast T-0902 which needed a fidius method. Emission half (A) was already done + proven under T-0902.

**Downstream:** T-0904 (stream accumulator via `call_streaming`) — the native accumulator drive is proven (T-0902); T-0905 (per-arch native artifact selection); T-0906 (cloacina-provider-kafka); T-0907 (kafka proof + authoring docs — the `features=["native"]` + fidius-core host-dep authoring shape belongs there).