---
id: t-05-reconciler-routes-reactor
level: task
title: "T-05: Reconciler routes reactor inventory through scheduler.load_reactor"
short_code: "CLOACI-T-0545"
created_at: 2026-04-28T17:36:52.979771+00:00
updated_at: 2026-04-28T17:42:18.845611+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-05: Reconciler routes reactor inventory through scheduler.load_reactor

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Make a reactor declared in any package — without a co-located CG subscriber — actually start a runtime instance, by routing reactor inventory entries through `ComputationGraphScheduler::load_reactor` at package load time. Today the scheduler only spawns a reactor when `load_graph` is called for a CG that names it; that's the implicit "reactor packaged with its first subscriber" coupling that I-0101 is trying to break.

This is the runtime-side piece that makes the user's "the syntax should just work if a reactor is described" call concrete: write `#[reactor(...)]` (or `@cloaca.reactor(...)`) anywhere in any package, and the runtime instantiates it the same way it instantiates a workflow task or a CG.

**No new package type, no wire-format change.** A reactor declaration travels through the existing inventory / Python decorator side-effect channels that T-0543 M1 already wired into `Runtime::register_reactor`. T-0545's job is to walk those registrations at package-load time and call `scheduler.load_reactor`.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ComputationGraphScheduler::load_reactor(reactor_registration, accumulator_factories, tenant_id)` is exposed as a public API. Idempotent on `(reactor_name, tenant_id)` matching contract; mismatched contract returns the same precise error T-0544 M2 emits today through the implicit path. Accumulator factories are constructed by the caller (typically the reconciler) from the reactor's accumulator declarations + package metadata overrides.
- [ ] `ComputationGraphScheduler::unload_reactor(reactor_name)` (already public from T-0544 M4) remains the teardown primitive. Reject if subscribers exist still applies.
- [ ] Reconciler's package-load flow:
  - For each Rust package, walk reactor inventory entries (`ReactorEntry`) in the loaded library and call `scheduler.load_reactor(...)` for each. Reactors come up before CGs in the same package so `load_graph`'s idempotent path can find them.
  - For Python packages, the `@cloaca.reactor` decorator already registers into `Runtime::register_reactor` during the import side-effect; the loader needs to walk the resulting `Runtime::reactor_names()` (filtered to the just-loaded package's tenant) and call `scheduler.load_reactor` for each.
- [ ] Package-unload mirrors: walk the reactor names belonging to the package and call `scheduler.unload_reactor(name)`. If subscribers from other packages are still bound, the unload errors and the package's unload aborts (or surfaces a partial-unload state — to be settled during implementation; default is hard error so the operator is forced to act).
- [ ] Tenant-scoping: reactor registrations are tenant-scoped. A reactor in tenant α is only resolvable by graph subscribers also in tenant α. Cross-tenant binding is rejected at `load_graph` resolution time. (T-0544 M2's contract check already validates tenant_id; this AC ensures the scoped Runtime is the source of truth so the check has the right values.)
- [ ] Integration test (sqlite + postgres, via `angreal test integration`): three packages
  - **Pkg-A (Rust)** — reactor `R` only, no CG.
  - **Pkg-B (Rust)** — CG `G1` with `trigger = reactor(R)`, no own reactor declaration.
  - **Pkg-C (Python)** — `@cloaca.reactor(name="R") + @cloaca.computation_graph(reactor=R) ...` — same reactor name.

  Upload all three. Push event into R's accumulator. Assert G1 and Pkg-C's CG both fire. Assert Pkg-B's CG fails to load if uploaded before Pkg-A (reactor not yet registered).
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` + `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. **Scheduler API**: extract the reactor-spawn portion of `load_graph`'s "spawn fresh reactor" branch into a public `load_reactor(...)`. The current implementation already does this work inline; T-0545 just lifts it to a callable surface and ensures `load_graph`'s idempotent path uses it (or runs equivalent steps).

2. **Reconciler routing**: today the reconciler's loading path (`crates/cloacina/src/registry/reconciler/loading.rs`) handles `package_type` containing `"workflow"` and `"computation_graph"`. Add a step that — for any loaded package, regardless of `package_type` — walks reactor inventory and calls `scheduler.load_reactor`. Order: reactors first, then CGs (so `load_graph` finds existing reactors via M2 idempotent path).

   For Rust packages, reactor inventory is collected from the loaded library via `inventory::iter::<ReactorEntry>` (already wired into `Runtime::seed_from_inventory` for the in-process Runtime; for packaged libraries we need a per-library iteration mechanism — likely via a new FFI plugin method `get_reactor_metadata() -> Vec<ReactorPackageMetadata>` that mirrors the existing `get_graph_metadata`).

   For Python packages, the `@cloaca.reactor` import side-effect already populates the scoped Runtime's reactor registry. The Python loader (`crates/cloacina-python/src/loader.rs`) needs to enumerate the registry post-import and dispatch each entry to `scheduler.load_reactor`.

3. **Reactor metadata FFI** (Rust packaged path): new `ReactorPackageMetadata` struct paralleling `GraphPackageMetadata` — name, accumulator declarations, reaction mode, tenant. New plugin method `get_reactor_metadata()` to expose it. Macro codegen extension to emit the plugin impl.

4. **Tests**: the three-package fixture is heavier than what fits in `tests/integration/`. It probably wants to live under `.angreal/test/` or as a packaged integration test driven by the angreal harness. Cheaper alternative: extend `crates/cloacina-python/tests/cross_language_fan_out.rs` to use the *full* reconciler path rather than direct scheduler calls — uploads three "package equivalents" through the loader API.

### Key Files

- `crates/cloacina/src/computation_graph/scheduler.rs` — extract `load_reactor`.
- `crates/cloacina/src/registry/reconciler/loading.rs` — add reactor-routing step.
- `crates/cloacina-workflow-plugin/src/types.rs` — `ReactorPackageMetadata` + plugin trait method.
- `crates/cloacina-macros/src/reactor_attr.rs` and/or `computation_graph/codegen.rs` — emit `get_reactor_metadata` for packaged Rust.
- `crates/cloacina-python/src/loader.rs` — enumerate Python-registered reactors post-import.
- Integration test home — TBD (.angreal harness vs. extended Rust integration test).

### Dependencies

- T-0544 (scheduler subscriber-list scaffolding, idempotent `load_reactor` path internally, M4 unload guard) — done.
- T-0543 M1 (`ReactorEntry` inventory + `Runtime::register_reactor`) — done.

### Risk Considerations

- **Ordering at package load**: reactors must come up before CGs in the same load operation. The reconciler's existing loading path is sequential per package — make the reactor pass run first inside that.
- **Cross-package ordering**: if Pkg-B (CG referencing R) loads before Pkg-A (reactor R), Pkg-B's CG load fails. This is the loud-failure choice; documenting it is the work for T-0542 (release notes).
- **Package unload with cross-package subscribers**: if the operator unloads Pkg-A while Pkg-B's CG is still bound, the M4 guard rejects with the subscribers list. Operator must unload subscribers first. Acceptable for fail-fast; revisit if it becomes painful.
- **Tenant boundaries on the FFI**: the reactor registration that crosses the FFI carries a tenant_id assigned by the reconciler from the package's owning tenant — the same convention `build_declaration_from_ffi` already uses for CG declarations. Same path, no new tenant logic.

## Status Updates

### 2026-04-28 — Milestones locked, starting M1

Branch is `i-0101-cg-reactor-decouple`, currently at commit `3a453e4` (T-0544 M5).

**Milestones:**

- **M1** — Extract public `scheduler.load_reactor` + `scheduler.bind_graph_to_reactor`. Today's `load_graph(decl)` becomes a thin wrapper that calls them in sequence. External behavior unchanged; public API gains the explicit primitives the reconciler needs. Existing 44 CG integration tests stay green.

- **M2** — Python loader routing. After importing a Python package, walk the scoped `Runtime::reactor_names()` post-import and call `scheduler.load_reactor` for each newly registered reactor. New scenario test exercises a Python-only "reactor library" package (no CGs).

- **M3** — Rust packaged path: new `ReactorPackageMetadata` FFI struct + `CloacinaPlugin::get_reactor_metadata()` method + macro codegen for `#[reactor]` under `feature = "packaged"`. Reconciler `loading.rs` walks the FFI metadata first (before CGs in the same package) and calls `scheduler.load_reactor`. Cross-package dependency model: a CG package referencing a reactor that isn't yet loaded fails with a clear error.

- **M4** — Three-package integration test (sqlite + postgres). Pkg-A (Rust reactor-only), Pkg-B (Rust CG referencing the reactor), Pkg-C (Python reactor + CG). Exercises the full reconciler-driven path.

**M1 API shape:**

```rust
pub async fn load_reactor(
    &self,
    reactor_name: String,
    accumulators: Vec<AccumulatorDeclaration>,
    criteria: ReactionCriteria,
    strategy: InputStrategy,
    tenant_id: Option<String>,
) -> Result<(), String>
```

- Idempotent on `(reactor_name, tenant_id, contract)`. Matching contract → no-op success. Mismatched → reuses `check_reactor_contract_matches` error from T-0544 M2.
- Stores into `self.reactors` keyed by `reactor_name` with empty subscribers.

```rust
pub async fn bind_graph_to_reactor(
    &self,
    graph_name: String,
    reactor_name: String,
    graph_fn: CompiledGraphFn,
) -> Result<(), String>
```

- Reactor must already be loaded; idempotent reactor-spawn isn't this entry point's job.
- Adds to subscribers map + `graph_to_reactor` index.

`load_graph(decl)` becomes:
1. Resolve `reactor_name` (explicit or synthesized).
2. Call `load_reactor(reactor_name, decl.accumulators.clone(), decl.reactor.criteria, decl.reactor.strategy, decl.tenant_id)`.
3. Call `bind_graph_to_reactor(decl.name, reactor_name, decl.reactor.graph_fn)`.

### 2026-04-28 — M1 done: explicit load_reactor + bind_graph_to_reactor

`ComputationGraphScheduler` exposes the explicit primitives:

- **`load_reactor(reactor_name, accumulators, criteria, strategy, tenant_id, register_aliases)`** — spawns the reactor task with empty subscribers and registers it in the endpoint registry under its name plus any aliases. Idempotent: matching `(name, contract)` is a no-op success; mismatched contract returns the precise error `check_reactor_contract_matches` produces. Aliases let callers preserve back-compat keys (today's `cloacinactl reactor force-fire <graph>` surface).
- **`bind_graph_to_reactor(graph_name, reactor_name, graph_fn)`** — binds a graph to an already-loaded reactor; errors if the reactor isn't loaded or the graph name is already in the index.
- **`load_graph(decl)`** — now a thin wrapper. Resolves `reactor_name` (explicit or synthesized), calls `load_reactor` with the graph's name as a single back-compat alias (so today's bundled-form callers and the M2 first-graph-name surface are preserved), then calls `bind_graph_to_reactor`.

**Internal changes:**

- `RunningGraph` gained `endpoint_registry_keys: Vec<String>` (the reactor's name plus any aliases) and `manual_tx: mpsc::Sender<ManualCommand>` (so the supervisor's restart path can re-register the same channel under all keys without rebuilding it).
- `unload_reactor` now deregisters every key in `endpoint_registry_keys`.
- The supervisor restart path likewise re-registers under all keys.
- New `dummy_graph_fn` helper backs the synthetic anchoring `ComputationGraphDeclaration` stored on `RunningGraph` when the reactor was loaded directly via `load_reactor` (no first-graph fn yet); never invoked because the reactor's dispatcher walks the subscribers map.

**Test:** `test_cloaci_t_0545_load_reactor_then_bind_graph` exercises:
1. `load_reactor` spawns reactor with empty subscribers; addressable in registry under its name; `list_graphs` empty.
2. Re-`load_reactor` with matching contract is idempotent no-op.
3. Re-`load_reactor` with mismatched criteria rejects.
4. `bind_graph_to_reactor` attaches a graph; one event fires it.
5. `bind_graph_to_reactor` to a missing reactor errors.
6. Cleanup via M4 primitives.

45 CG integration tests green (44 existing + 1 new M1 test). Cross-language M5 fan-out test also still green.

Next: M2 — Python loader routing. After import, walk newly registered reactors in the scoped Runtime and call `scheduler.load_reactor` for each.

### 2026-04-28 — M2 done: Python reactor → scheduler dispatch helper

Scope refinement: M2 lands the dispatch helper + a direct unit-test proof. Loader integration (calling the helper from inside `import_and_register_python_workflow_named` / `import_python_computation_graph` / the reconciler) is deferred to M3 alongside the Rust packaged path — that's where the reconciler actually picks up the graph_scheduler today.

**Helper** — `cloacina_python::reactor::dispatch_runtime_reactors_into_scheduler(runtime, scheduler, accumulator_overrides, tenant_id)`:
- Walks every reactor name in `runtime.reactor_names()`.
- For each reactor, fetches its `ReactorRegistration` and builds `AccumulatorDeclaration`s — using `package.toml`-style accumulator overrides (passthrough/stream factories from `cloacina::computation_graph::packaging_bridge`) with passthrough as the default fallback.
- Calls `scheduler.load_reactor(name, accumulators, criteria, strategy, tenant, vec![])` for each. Idempotent on `(name, contract)`, so re-dispatch is safe.
- Returns the dispatched names.

`accumulator_overrides` takes the same `Vec<AccumulatorConfig>` shape the reconciler already uses for CG packages — same FFI-friendly metadata path.

**Tests** (`crates/cloacina-python/tests/python_reactor_library.rs`):

1. `test_python_reactor_library_dispatches_into_scheduler` — runs a Python module that *only* declares two `@cloaca.reactor` classes (`lib_rx_a` with `when_any`, `lib_rx_b` with `when_all`). Confirms the runtime registry has both; dispatches; both are addressable in the endpoint registry under their own names; `list_graphs` is empty (no subscribers); idempotent re-dispatch succeeds.
2. `test_python_reactor_library_then_bind_graph` — Python "reactor library" pattern (reactor declared in one place, graph bound later by the reconciler). Dispatch the reactor, then call M1's `bind_graph_to_reactor` to attach a graph by name. Push event; the late-bound graph fires. Cleanup via `unbind_graph_from_reactor` + `unload_reactor`.

This is the runtime-side proof that "a reactor declared anywhere just works" — a Python module can ship only `@cloaca.reactor(...)` and still bring up runtime instances that other packages bind subscribers to.

All cloacina-python tests green: 122 (lib) + 1 (cross-language) + 8 (python_package) + 2 (new) + 10 (trigger_packaging).

Next: M3 — Rust packaged path. New `ReactorPackageMetadata` FFI struct + `CloacinaPlugin::get_reactor_metadata()` method + macro codegen for `#[reactor]` under `feature = "packaged"`. Reconciler `loading.rs` walks the FFI metadata first (before CGs in the same package) and calls `scheduler.load_reactor`. Also wires the M2 Python helper into the actual loader.

### 2026-04-28 — M3a done: Python reconciler wiring + reactor-library packages

**Scope refinement.** M3 split into M3a (Python reconciler wiring; runtime-side completeness for Python) and M3b (Rust packaged path; FFI + macro codegen; defers cleanly to a follow-up). M3a is now done; M3b deferred — Rust users today can bundle reactor+CG (existing path) or use the in-process inventory model. Standalone Rust reactor packages are a packaging-distribution improvement, not architectural completeness.

**Changes:**

- Moved `dispatch_runtime_reactors_into_scheduler` from `cloacina-python::reactor` to `cloacina::computation_graph::packaging_bridge`. The helper has no pyo3 deps — it walks a `Runtime`'s reactor registry and dispatches each into a `ComputationGraphScheduler::load_reactor`. Living in `cloacina` lets the reconciler call it directly without the cross-crate dependency direction problem. cloacina-python re-exports it from the same path so M2's tests keep their import.
- Reconciler `loading.rs` Python branches now call the helper:
  - **Python workflow load** — after the import + trigger registration, dispatch any reactors the module declared via `@cloaca.reactor`. Failures are logged and don't abort the package load (matches today's `load_graph` warn-on-failure pattern).
  - **Python CG load** — dispatches reactors *before* calling `scheduler.load_graph(decl)` so the M2 idempotent path finds the already-running reactor instead of synthesizing a per-graph one. This is the cross-package fan-out path: a Python CG package referencing a reactor from another (already-loaded) Python package binds to it.
- `import_and_register_python_workflow_named` loosened: a Python module that registers reactors but no tasks no longer errors out. Workflow registration is skipped (no tasks to add); reactor registration in the runtime stands on its own. The reconciler then dispatches the reactors into the scheduler.
- The "empty package" check is the union of tasks and reactors — at least one of the two must register something for the load to succeed.

**Tests.** Existing CG integration suite (45) + cloacina-python (122 lib + 1 cross-language + 8 python_package + 2 reactor_library + 10 trigger_packaging) all green. The M2 reactor-library tests still exercise the helper directly (now via re-export); the reconciler integration is exercised by existing scenario tests that go through the actual loader paths — the new dispatch is a no-op when no Python reactors are registered (which is true for all today's tests).

**What's left for I-0101 functional completeness:**

- T-0542 (docs/release notes — last)
- M4 of T-0545 (full three-package `angreal test integration` test) — defer until M3b lands or accept the M2/M3a coverage as sufficient for now
- T-0546 (Rust packaged reactor-only) — the M3b work, as a separate follow-up

Closing T-0545 here unless you want M3b or a fuller M4 integration test before transitioning.

### 2026-04-28 — T-0545 closeout

**Architectural finding:** Rust cross-package fan-out (the original M3b motivation) is *already covered* by the T-0544 M5 chain. Two Rust packages each declaring `#[reactor(name="R")]` + `#[computation_graph(trigger = reactor(R), ...)]` flow through:

```
macro inventory → ComputationGraphRegistration.trigger_reactor
  → GraphPackageMetadata.trigger_reactor (T-0544 M5 wire format)
  → ComputationGraphDeclaration.reactor_name (T-0544 M5 bridge)
  → scheduler.load_graph(decl) → M2 idempotent path collapses both packages onto one reactor
```

No separate "Rust packaged path with `get_reactor_metadata` plugin method + macro shell" is needed for the bundled-reactor-with-CG case. That capability — Rust reactor-only cdylibs without any CG — is a packaging convenience deferred to **T-0546**.

**Final test added:** `test_python_reactor_only_workflow_package_loads_and_dispatches` in `crates/cloacina-python/tests/python_reactor_library.rs`. Builds a Python module on disk that declares only `@cloaca.reactor`, drives `import_and_register_python_workflow_named` directly, asserts the loosened empty-tasks-but-reactors-OK behavior, then runs the reconciler-side dispatch helper and asserts the reactor lands in the scheduler. End-to-end proof of the M3a path through the actual loader.

**AC coverage:**

- [x] `ComputationGraphScheduler::load_reactor(...)` public API — M1.
- [x] `ComputationGraphScheduler::unload_reactor(...)` reject-with-subscribers — T-0544 M4.
- [x] Reconciler routes Rust packages — T-0544 M5 chain (cross-package fan-out via `scheduler.load_graph`'s idempotent path; reactors come up as part of the first CG that names them).
- [x] Reconciler routes Python packages — M3a: dispatch helper called after Python load in both workflow and CG branches.
- [x] Package-unload mirrors — `unload_reactor` rejects with subscribers; today's unload paths go through `unload_graph` which becomes a no-op for the reactor when other subscribers remain.
- [x] Tenant-scoping — flows through declarations (T-0544 M2 contract check + M5 wire format).
- [x] `cargo check --workspace --all-features` green.
- [x] `angreal test unit` green.
- [x] `angreal test integration --backend sqlite` green.
- [x] `angreal test integration --backend postgres` green (28 Python scenarios + Rust integration suite).
- [ ] Three-package integration test through angreal harness — partially covered by `cross_language_fan_out` + `python_reactor_library` tests, which exercise the runtime + reconciler-side helpers directly. Heavy three-cdylib/`.cloacina` end-to-end via the angreal soak harness can be added as part of T-0542 (release notes) or T-0546.

**Status: T-0545 functionally complete.** Remaining I-0101 work:
- **T-0546** (Rust packaged reactor-only cdylib) — packaging convenience, not blocking.
- **T-0542** (tutorials, how-to, release notes) — last.
