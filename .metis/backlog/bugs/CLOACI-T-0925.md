---
id: provider-search-path-is-process
level: task
title: "PROVIDER_SEARCH_PATH is process-global — constructor resolution is tenant-blind"
short_code: "CLOACI-T-0925"
created_at: 2026-08-06T02:07:52.097400+00:00
updated_at: 2026-08-07T21:34:06.142718+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# PROVIDER_SEARCH_PATH is process-global — constructor resolution is tenant-blind

## Objective

Make constructor/provider resolution respect the tenant boundary. Distinct from the name-keying class fixed in T-0921 and continued in T-0924: here the problem is a process-wide search directory, so the resolution INPUT is tenant-blind rather than the registry key.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. PROVIDER_SEARCH_PATH is a process-global (see crates/cloacina/src/registry/loader/constructor_loader.rs — set_provider_search_path / clear_provider_search_path). Every tenant's constructor!(from = "provider@version") resolves against the same directory set, so one tenant's staged providers are visible to another's resolution in a shared server process.
2. Interaction with packaged staging: the reconciler stages a package's OWN bundled provider archives hermetically (empty bundle clears the global path), which mitigates the packaged path — verify how far that mitigation actually extends before designing. The embedded/host-configured path is the exposed case.
3. Interaction with T-0920: that ticket added a runtime pin (wasm|native) enforced at resolution. The pin makes the TRUST TIER explicit but does not scope WHICH provider directory is searched.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Provider search scope is per-tenant (or explicitly documented as a deployment-level, deliberately-shared resource with the reasoning stated)
- [x] A provider staged for tenant A is not resolvable by tenant B's constructor node in the same process (test)
- [x] The packaged hermetic-staging path and the host-configured path are both covered by the chosen model

## Status Updates

- 2026-08-06: Filed from CLOACI-T-0921's deferrals (audit row #19) — classified there as a different problem class from name-keying and left out of scope deliberately.

- 2026-08-06 PHASE 1 (verification, worktree .claude/worktrees/t-0925, branch fix/t-0925-provider-search-tenant-scope). The hermetic-staging mitigation is NARROWER than the ticket assumed; the exposure is REAL, not merely latent. Evidence:

  1. The global. `crates/cloacina/src/registry/loader/constructor_loader.rs:1649` `static PROVIDER_SEARCH_PATH: RwLock<Option<PathBuf>>`; setter/clearer at :1662/:1667; reader `provider_search_path()` :1674 (override → `CLOACINA_PROVIDER_PATH` :1678 → `providers` :1683).

  2. Staging is set-only, never restored. `crates/cloacina/src/registry/reconciler/loading.rs:2246 stage_bundled_providers` unpacks the package's own archives into a leaked TempDir and calls `set_provider_search_path(&providers_root)` (:2302). It NEVER restores the previous value. Empty bundle → `clear_provider_search_path()` (:2262), which does not fail closed — it falls through to env/`providers`. So after any package load the process global points at THAT package's staged root until the next load re-points it.

  3. Loads are NOT serialized across tenants. Serialization holds only WITHIN one reconciler. Each tenant gets its own `DefaultRunner` (`crates/cloacina-server/src/tenant_runner_cache.rs:27-176`), and every runner spawns its own reconciliation loop as an independent tokio task (`crates/cloacina/src/runner/default_runner/service_manager.rs:341-372` → `RegistryReconciler::start_reconciliation_loop`, `registry/reconciler/mod.rs:371`). There is no process-wide load mutex (no static lock anywhere in reconciler/{mod,loading}.rs). The load path is async and yields at many awaits between staging (:439/:531) and the resolution steps (:451 reactors, :465 reactor-bound CGs, :479 constructor nodes) — so tenant B's `set_provider_search_path` can land inside tenant A's load window. This is a live cross-tenant race in the shared server process, not a hypothetical.

  4. Two DEFERRED resolution sites read the global from a different task entirely, i.e. outside any staging window:
     - `crates/cloacina/src/computation_graph/packaging_bridge.rs:479` — the provider-backed stream accumulator is `tokio::spawn`ed at reactor-spawn time and only calls `load_stream_accumulator_source_from_config` (:524 → reads `provider_search_path()` at constructor_loader.rs:1324) when that detached task is first scheduled. Whatever tenant last set the global wins.
     - `crates/cloacina/src/computation_graph/scheduler.rs:241 resolve_reactor_evaluator` → `spawn_blocking` (:251) → `load_reactor_constructor_node_pinned` (:265 → reads the global at constructor_loader.rs:2323).

  5. Tenant identity at the resolution sites: NONE. `load_constructor_node[_pinned]` (:2142/:2172) and `load_reactor_constructor_node[_pinned]` (:2303/:2316) take no tenant. The reconciler knows its tenant only as `self.config.default_tenant_id` (loading.rs:2373 etc.) — one reconciler instance == one tenant — and the macro-emitted call site (`crates/cloacina-macros/src/workflow_attr.rs:1024`) plus the Python bridge (`crates/cloacina-python/src/constructor.rs:162`) have no way to pass one. So a `tenant_id` PARAMETER is not threadable to the authoring call sites; the scope must be ambient there.

- 2026-08-06 PHASE 2 (implementation landed in the worktree, uncommitted). Design: **explicit parameter first, ambient scope only where a parameter cannot reach, process global demoted to lowest-precedence fallback.**
  - New in `crates/cloacina/src/registry/loader/constructor_loader.rs`: `ProviderScope{Staged(PathBuf),Unbundled}`, `ScopedProviderSearch` (thread-local RAII guard, nestable, mirrors cloacina-python's `registration_scope` from T-0921), `current_provider_scope()` (so a scope can be carried across a thread hop). `provider_search_path()` precedence is now scope → process override → `CLOACINA_PROVIDER_PATH` → `providers`. With no scope installed it is byte-for-byte the old function, so `set_provider_search_path` + embedded/untenanted use is unchanged.
  - Explicit-path primitives added: `load_constructor_node_in`, `load_constructor_node_pinned_in`, `load_reactor_constructor_node_pinned_in`, `load_stream_accumulator_source_from_config_in`. The old ambient-path functions are now thin wrappers over them.
  - `stage_bundled_providers` (reconciler/loading.rs) returns `Option<PathBuf>` and NO LONGER calls `set_provider_search_path` / `clear_provider_search_path`. The reconciler hands that root to every step that resolves: `step_load_reactors`, `step_load_reactor_bound_cgs`, `step_load_constructor_nodes` (which resolves inside `spawn_blocking` with the root moved into the closure).
  - Deferred sites de-raced: `ProviderStreamAccumulatorFactory` now BINDS its root at declaration-build time (`new_in`, or `new` capturing the ambient path) instead of reading a global inside its spawned task; `ComputationGraphScheduler::load_reactor_in` / `load_graph_in` carry the root to `resolve_reactor_evaluator` → `load_reactor_constructor_node_pinned_in`. Additive `_in` variants throughout — every existing signature still exists and delegates with `None`, so no test or embedded caller changed.
  - Python path (the one call site that cannot take a parameter — `cloaca.constructor(..)` runs inside the module import): the reconciler installs `ScopedProviderSearch::for_staged_root(..)` inside the `spawn_blocking` closure, and `crates/cloacina-python/src/loader.rs` captures it before `std::thread::spawn` and re-installs it on the import thread, exactly as it already does for `ScopedRuntime` / `ScopedRegistration`.
  - BONUS FIX found while wiring: the Python **computation-graph** branch (loading.rs ~line 693) never called `stage_bundled_providers` at all, so a Python CG package's provider-backed accumulators resolved against whatever tree the previous load left in the global. It now stages its own bundle and scopes the import to it.
  - The process global SURVIVES, deliberately, as the embedded/single-tenant knob — documented as lowest precedence, with `set_provider_search_path`'s doc now saying a multi-tenant host must not use it.

- 2026-08-06 PHASE 3 (tests, all written and passing locally; wasm32-wasip2 target IS installed on this host so the gated suites actually ran):
  - NEW `crates/cloacina/tests/constructor_provider_tenant_scope_wasm.rs` (3 tests, all PASS): a `prefix` provider staged for tenant A is NOT resolvable by tenant B in the same process (and B's error names B's own directory, never A's); a leftover process-wide override cannot leak into a `Staged`- or `Unbundled`-scoped load; the embedded/untenanted `set_provider_search_path` + `load_constructor_node` path resolves exactly as before.
  - NEW unit tests `provider_scope_tests` in constructor_loader.rs (4 tests, PASS): scope beats the process override; `Unbundled` skips it; scopes nest and restore; capture + re-install carries a scope across a thread hop (the Python-loader contract).
  - SERIAL-ANNOTATION CLEANUP: `crates/cloacina/tests/constructor_reactor_scheduler_wasm.rs` — both tests dropped `#[serial_test::serial(provider_search_path)]` and their `set_/clear_provider_search_path` calls; they now pass their own dir via `load_graph_in`. Both PASS. (The remaining `set_provider_search_path` uses in constructor_workflow_node_wasm.rs / constructor_runtime_pin_wasm.rs / packaged_constructor_e2e.rs deliberately exercise the EMBEDDED surface and were left alone — they carry no serial annotation today and share one process-wide dir by design.)
  - Validation: `cargo check -p cloacina --no-default-features --features postgres,sqlite` clean; `cargo check -p cloacina --features constructors-wasm --tests` clean; `cargo check -p cloacina-python --tests` and `-p cloacina-agent` clean.

- 2026-08-06 FINAL (worktree `.claude/worktrees/t-0925`, branch fix/t-0925-provider-search-tenant-scope, UNCOMMITTED as instructed). Suites run, all green: constructor_provider_tenant_scope_wasm (3), constructor_reactor_scheduler_wasm (2, de-serialized), constructor_workflow_node_wasm (4), constructor_runtime_pin_wasm (7), packaged_constructor_e2e (3), provider_bundle (7), lib `provider_scope_tests` (4) and lib `registry::reconciler` (29, includes the staging→resolution seam tests). `cargo fmt --all` applied; `cargo check` clean for cloacina (no-default + postgres,sqlite), cloacina (constructors-wasm --tests), cloacina-python --tests, cloacina-agent, cloacina-server. Docs: precedence + the "don't use the process knob in a multi-tenant host" rule written into docs/content/engine/constructors/consume-a-provider.md.
  DELIBERATELY LEFT: (a) `crates/cloacina-agent/src/main.rs` still stages via `set_provider_search_path` — a single-tenant agent process, which is exactly what the knob is for; (b) the macro-emitted `load_constructor_node_pinned` in EMBEDDED builds still resolves ambiently (packaged builds emit a declaration instead and never call it, so no tenant crosses there); (c) providers are still staged into a leaked temp dir per package version and constructor nodes still die with the runtime rather than per-package unload — the pre-existing T-0836 caveats, untouched.

  CONCLUSION (honest scoping): "packaged loads are already isolated" is FALSE. The mitigation reaches exactly one thing — the resolution the reconciler itself performs synchronously between its own `set_provider_search_path` and the next await — and even that only when no other tenant's reconciler interleaves. Fix must (a) stop the reconciler mutating process-global state, (b) give the deferred/spawned sites the path that was in effect when their declaration was built, (c) leave the embedded global untouched as a fallback.

- 2026-08-07: COMPLETED — PR #244 merged (squash).

  Shipped as designed in phases 2/3 above. The headline correction stands: the
  ticket's own premise (#2, "hermetic staging mitigates the packaged path, the
  embedded knob is the exposed case") was BACKWARDS. Staging was set-only and
  never restored, loads are not serialized across tenants because each tenant
  gets its own DefaultRunner with an independent reconciliation loop, two
  resolution sites read the global from spawned tasks entirely outside any
  staging window, and the Python CG branch never staged at all. The packaged
  path was the more exposed one.

  REBASE NOTE: merging #237 (T-0919) put this into conflict on
  crates/cloacina-python/src/loader.rs — the third ticket in a row to collide
  there; that file is the crossroads for the import guard, registration scope,
  and provider scope. Resolution was mechanical once read: main had replaced
  the local IMPORT_TIMEOUT_SECS const with import_timeout() from import_guard,
  while this branch still carried the old const beside its provider-scope
  helpers. Kept the helpers, dropped the superseded const, confirmed by grep
  that both timeout sites use import_timeout(), verified with a real
  cargo check -p cloacina-python (the LSP was emitting false syntax errors
  mid-rebase and was not trusted).

  RESIDUALS, unchanged from the FINAL entry: cloacina-agent still uses the
  process knob (single-tenant process — that is the knob's purpose); embedded
  macro-emitted loads still resolve ambiently (packaged builds emit
  declarations instead, so no tenant crosses); the T-0836 caveats about leaked
  per-version temp dirs and runtime-lifetime constructor nodes are untouched.
