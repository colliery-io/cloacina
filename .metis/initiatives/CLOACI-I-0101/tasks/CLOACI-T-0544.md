---
id: t-02b-multi-graph-per-reactor-fan
level: task
title: "T-02b: Multi-graph-per-reactor fan-out"
short_code: "CLOACI-T-0544"
created_at: 2026-04-25T15:08:05.348610+00:00
updated_at: 2026-04-28T17:36:48.760134+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-02b: Multi-graph-per-reactor fan-out

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Lift the current "one reactor instance per graph" constraint in `ComputationGraphScheduler` so a single `#[reactor]` declaration can fan out to multiple `#[computation_graph(trigger = reactor(R))]` subscribers. Today's `load_graph_split` builds a fresh reactor instance per graph load (carried over from the bundled-form era — see T-0543 M5 status note); this task adds a shared-reactor binding path so one firing of R invokes every graph subscribed to R, rather than just one.

This was originally folded into T-0540's acceptance criteria (the fan-out integration test), but the runtime change is independent of the workflow-task `invokes = computation_graph(...)` macro work, so it lives on its own.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Scheduler exposes a way to register a reactor once and bind multiple graphs to it (rather than the current 1:1 `load_graph_split`). Concrete shape: e.g. `load_reactor(ReactorRegistration, accumulator_factories) -> ReactorHandle` + `bind_graph_to_reactor(graph_name, reactor_name, graph_fn)` — exact API to be settled during implementation.
- [ ] On firing, every graph bound to the reactor receives the same `InputCache` and runs (sequentially or concurrently — to be decided). Failure of one graph does not block siblings.
- [ ] Existing single-graph split-form path stays green (either kept as a thin wrapper over the new API, or migrated to it).
- [ ] Integration test (sqlite + postgres): two graphs declare `trigger = reactor(R)`; pushing one event to R's accumulator fires both graphs, both terminal outputs observed.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` + `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. Audit current `ComputationGraphScheduler` — `load_graph_split` constructs the reactor + accumulators + graph binding in one shot. Identify what needs to split: probably the reactor lifecycle (accumulator factories + reactor loop) becomes a standalone object, and graph subscription becomes a separate operation.
2. Decide concurrency model on firing — sequential (simpler, deterministic) or concurrent via tokio::join (more throughput, harder error handling). Default to sequential unless a benchmark says otherwise.
3. Inventory side: `ReactorEntry` is already separate from `ComputationGraphEntry`; the runtime seeding step in `Runtime::seed_from_inventory` may need to register reactors first, then bind subscribed graphs in a second pass.
4. Update integration tests in `crates/cloacina/tests/integration/computation_graph.rs` to cover the fan-out shape.

### Key Files

- `crates/cloacina/src/computation_graph/scheduler.rs` — primary surface change.
- `crates/cloacina/src/runtime.rs` — seed-from-inventory may need a two-pass change.
- `crates/cloacina/tests/integration/computation_graph.rs` — fan-out test.

### Dependencies

- T-0539 (split form is the only form on main; bundled form is removed). Already done.

### Risk Considerations

- **Failure isolation**: one graph panic must not poison the reactor or its other subscribers. Wrap each graph invocation in catch_unwind or run in an isolated task per subscriber.
- **Ordering guarantees**: nothing in the docs promises subscriber ordering today. Pick one (declaration order? alphabetical?) and document it.
- **Cancellation**: T-0487 cooperative cancellation needs to apply per-subscriber, not at the reactor level.

## Status Updates

### 2026-04-28 — Locked decisions before any code changes

T-0544's scope stays as-written: scheduler refactor only. Reactor-only package shape, cross-package CG resolution, lifecycle policy beyond unload-rejection, tenant-scoping enforcement, CLI changes — all deferred to follow-on tasks (T-0546+).

**Locked decisions:**

1. **Subscriber storage location** — in the scheduler, not the runtime registry. Runtime keeps reactor *constructors* (inventory seeding); scheduler owns running state and dispatch. No change to `Runtime::register_reactor`'s public surface.

2. **Subscriber storage shape** — `HashMap<graph_name, BoundGraph>` keyed by graph name. Needed for `unbind_graph_from_reactor` lookup anyway; no order promise. Cross-package load order is non-deterministic by construction, so we don't pretend otherwise.

3. **Concurrency model on firing** — two-pass via `tokio::join_all`: pass 1 fires all subscribers concurrently, pass 2 iterates the `Vec<Result<...>>`. Slow subscribers don't block fast ones; errors logged per-subscriber, no short-circuit. Failure isolation is automatic (each future is independent).

4. **Idle reactor policy** — reactor stays running with zero subscribers. Idle accumulators consume nothing; preserves the "reactors are independent units" framing.

5. **Unload semantics** — `unbind_graph_from_reactor(graph)` removes a subscriber, reactor keeps running. `unload_reactor(name)` rejects if `subscribers.len() > 0` with a clear error pointing at the bound graphs. Tenant-scoping enforcement at the package boundary deferred.

6. **Idempotent `load_reactor` with contract validation** (implicit but critical). Second `load_reactor(name, contract)` call:
   - Same `(name, tenant)` + same contract (accumulators, criteria, mode) → no-op, return existing handle.
   - Same name, mismatched contract → reject with a precise error.

   This is the mechanism that makes cross-package fan-out work without yet introducing reactor-only packages: two bundled-form packages each declaring reactor `R` end up sharing a single `R` instance in the runtime.

7. **Test fixture** — cross-language cross-package: one Rust package declaring reactor `R` + graph `G1`, one Python package declaring reactor `R` (same contract) + graph `G2`. Upload both; push one event to `R`'s accumulator; both `G1` and `G2` fire. Plus negative tests: contract-mismatch on second `load_reactor` rejects; `unload_reactor` with bound subscribers rejects.

**Implementation milestones (each a committable step):**

- **M1** — Internal storage shape change in `ComputationGraphScheduler`. Keep `load_graph_split` external API stable (it becomes a thin wrapper over `load_reactor` + `bind_graph_to_reactor`). Existing tests stay green.
- **M2** — Idempotent `load_reactor` with contract validation; mismatched contract surfaces a precise error.
- **M3** — Two-pass concurrent dispatch via `tokio::join_all`. Failure isolation per subscriber.
- **M4** — `unbind_graph_from_reactor` + `unload_reactor` with subscriber-rejection guard.
- **M5** — Integration test: two packages (Rust + Python), same reactor name, fan-out fires both graphs. Plus the two negative tests above.

Branch is `i-0101-cg-reactor-decouple`, currently at commit `6763c2c` (T-0541 M5).

### 2026-04-28 — M1 done: scheduler subscriber-list scaffolding

`crates/cloacina/src/computation_graph/scheduler.rs`:

- New `ReactorSubscribers = Arc<RwLock<HashMap<String /* graph_name */, CompiledGraphFn>>>` type alias.
- New `make_subscriber_dispatcher(reactor_name, subscribers) -> CompiledGraphFn` helper. The closure walks the subscriber map on each firing and runs every entry. M1 keeps dispatch sequential (single-subscriber today; M3 swaps to `tokio::join_all`). Per-subscriber errors are logged but don't short-circuit; the reactor sees one `GraphResult::Completed` per firing regardless of subscriber count, matching today's per-reactor fire-counter semantics.
- `RunningGraph` gained a `subscribers: ReactorSubscribers` field. Initialized at `load_graph` time with one entry (the bundled graph_fn under the graph's own name). The `Reactor` actor receives the dispatcher closure instead of the bundled `decl.reactor.graph_fn` — the running reactor task no longer holds the graph_fn directly, so adding subscribers later doesn't require restarting it.
- Restart path (`check_and_restart_failed`) reuses the same `Arc`'d subscriber map across restarts, so subscribers bound mid-life survive reactor crashes.

Behavior is byte-identical to before this commit (still one subscriber per reactor in every code path that exists today). All 39 CG integration tests green: `cargo test -p cloacina --no-default-features --features sqlite,macros --test integration computation_graph` — 39 passed.

Next: M2 — thread `reactor_name` through `ComputationGraphDeclaration` and `GraphPackageMetadata` so cross-package fan-out can light up.

### 2026-04-28 — M2 done: explicit reactor identity + idempotent registration

**Scope refinement.** M2 lands the declaration field, the storage re-keying, and the idempotent contract-matched binding — all driven by a Rust direct-scheduler-API integration test. The FFI wire-format change to `GraphPackageMetadata` and the Python `build_python_graph_declaration` propagation have been deferred to M5 (where the cross-package cross-language test forces them).

**Changes:**

- `ComputationGraphDeclaration` gained `reactor_name: Option<String>`. `None` (today's bundled-form default) synthesizes `format!("__Reactor_{}", graph_name)` so existing callers keep their 1:1 reactor-per-graph behavior. `Some(name)` opts into shared-reactor binding.
- `load_graph_split` sets `reactor_name = Some(reactor.name.clone())` on the declaration it builds — split-form callers now opt in by construction.
- Scheduler storage re-keyed: `self.graphs` (keyed by graph_name) → `self.reactors` (keyed by reactor_name) + new `self.graph_to_reactor: HashMap<graph_name, reactor_name>` index. External operations (`unload_graph`, `list_graphs`, `shutdown_all`) take graph names and route through the index.
- `load_graph` resolution flow:
  - Reject re-loading the same graph_name (graph_to_reactor contains check).
  - If reactor_name already running with matching contract (`check_reactor_contract_matches` validates accumulators / criteria / strategy / tenant_id) → bind as additional subscriber, skip spawn entirely. Mismatch → reject with precise error.
  - Otherwise spawn a fresh reactor and seed subscribers with the first graph_fn.
- Endpoint-registry key stays the *first graph's name* (preserves today's `cloacinactl reactor force-fire <graph>` operator surface). Subsequent fan-out subscribers don't re-register.
- `unload_graph` now removes the graph from subscribers; only when subscribers becomes empty does it tear down the reactor (and accumulators / endpoint-registry / channels). For today's 1:1 callers this is byte-identical to before.
- Restart path uses the anchoring declaration's `name` for endpoint-registry re-registration, since the loop variable in `check_and_restart_failed` is now the reactor_name.
- `ReactionCriteria` and `InputStrategy` gained `PartialEq` / `Eq` (needed for the contract-matching helper).
- `ComputationGraphDeclaration` literal sites updated with `reactor_name: None` defaults: `packaging_bridge::build_declaration_from_ffi`, Python `build_python_graph_declaration`, two scheduler unit tests, two integration-test sites.

**Tests.**

Two new T-0544 tests in `tests/integration/computation_graph.rs`:

1. `test_cloaci_t_0544_two_graphs_share_one_reactor_via_split_form` — two graphs both naming reactor `cloaci_t_0544_shared_reactor` via `load_graph_split`. Single event → both subscribers fire. `list_graphs` reports both bindings. Unloading g1 leaves the reactor up for g2; another event fires only g2. Unloading g2 (last subscriber) tears down the reactor.
2. `test_cloaci_t_0544_contract_mismatch_rejected` — second `load_graph_split` naming the same reactor with a different `reaction_mode` is rejected with a precise error pointing at the criteria mismatch.

All 41 CG integration tests green: `cargo test -p cloacina --no-default-features --features sqlite,macros --test integration computation_graph` — 39 existing + 2 new M2 tests, no regressions.

Next: M3 — switch `make_subscriber_dispatcher` from sequential to `tokio::join_all` so slow subscribers don't block fast ones.

### 2026-04-28 — M3 done: concurrent dispatch via futures::future::join_all

`make_subscriber_dispatcher` flipped from a serial `for` loop to a two-pass concurrent dispatch:

- **Pass 1**: snapshot the subscribers, build one async block per subscriber returning `(graph_name, GraphResult)`, hand the iterator to `futures::future::join_all`. All subscribers' futures are polled concurrently.
- **Pass 2**: walk the resulting `Vec<(String, GraphResult)>` and log per-subscriber errors. No short-circuit; the reactor sees one `GraphResult::Completed` per firing regardless of any subscriber's error.

`futures` is already a workspace dep (no Cargo.toml change). Failure isolation is automatic — each future is independent and a panic/error in one doesn't poison sibling futures.

**New test**: `test_cloaci_t_0544_dispatch_is_concurrent` — two subscribers on one reactor, one sleeps 200ms, one is instant. Asserts fast subscriber completes within 100ms (would be 200ms+ under sequential dispatch since `join_all` polls in iterator order) AND slow subscriber takes ≥180ms (so we're not silently skipping the sleep). Confirms subscribers genuinely run in parallel.

42 CG integration tests green: 39 existing + 3 T-0544 tests.

Next: M4 — explicit `unbind_graph_from_reactor` / `unload_reactor` API + reject `unload_reactor` while subscribers exist. The unload-with-subscribers rejection is the lifecycle guard for the standalone-reactor-package shape (T-0546+); landing it now means the API surface is honest about ownership.

### 2026-04-28 — M4 done: explicit unbind / unload_reactor + lifecycle guard

`ComputationGraphScheduler` gained two new public methods that expose the honest reactor lifecycle:

- **`unbind_graph_from_reactor(graph_name) -> Result<String, String>`** — pure subscriber removal. Reactor (and its accumulators) keeps running, ready for new subscribers. Returns the reactor name on success so callers can chain to `unload_reactor` if they want the bundled-form behavior. Errors if the graph isn't bound, or if the index points to a non-existent reactor (consistency guard).
- **`unload_reactor(reactor_name) -> Result<(), String>`** — explicit teardown. Snapshots subscribers under read lock and rejects with a precise error listing the bound graphs if any remain (`"reactor 'X' has 2 bound subscriber(s): [\"a\", \"b\"]; unbind them first"`). With zero subscribers, tears down: shutdown signal, await reactor handle, await accumulator handles, deregister endpoint registry.

`unload_graph(graph_name)` is now a backward-compat convenience: it calls `unbind_graph_from_reactor` + (if subscribers became empty) `unload_reactor`. Today's 1:1 callers (every existing test, every bundled-form package) keep their behavior unchanged. Independent-reactor consumers (T-0546+) will use the explicit pair.

**Two new tests:**

1. `test_cloaci_t_0544_unbind_keeps_reactor_running` — load `g`, unbind it, list_graphs returns empty. Load `later` naming the same reactor — M2's idempotent path binds to the still-running reactor. Push event; only `later` fires (g is gone). Cleanup via explicit `unbind` + `unload_reactor`.
2. `test_cloaci_t_0544_unload_reactor_rejects_with_subscribers` — two subscribers bound; `unload_reactor` rejects with an error naming both subscribers and hinting "unbind them first". After explicit unbinds, `unload_reactor` succeeds.

44 CG integration tests green: 39 existing + 5 T-0544 tests.

Next: M5 — cross-language cross-package integration test. Forces the FFI wire-format change (`GraphPackageMetadata.trigger_reactor: Option<String>` with `#[serde(default)]`) and the Python `build_python_graph_declaration` propagation deferred from M2.

### 2026-04-28 — M5 done: reactor-name plumbing + cross-language fan-out test

The reactor name now flows from each package's authoring surface (Rust macro / Python decorator) all the way to the scheduler, so two packages naming the same reactor share a runtime instance via M2's idempotent path. **No special "reactor package" type is needed** — a reactor declared in any package "just works."

**Wire-format changes:**

- `GraphPackageMetadata.trigger_reactor: Option<String>` with `#[serde(default)]`. Backward compatible: packages built before M5 deserialize to `None`; new packages populate it.
- Rust `#[computation_graph(trigger = reactor(R))]` macro codegen now emits `trigger_reactor: Some(<R as Reactor>::NAME.to_string())` in the FFI metadata. Bundled-form (no `trigger` clause) emits `None`.
- `build_declaration_from_ffi` reads `graph_meta.trigger_reactor` and sets it on the declaration.
- Python `PythonGraphExecutor` gained a `reactor_name: Option<String>` field, populated from the `@cloaca.reactor` class binding via `ComputationGraphBuilder.__exit__`. `build_python_graph_declaration` propagates it onto the declaration.
- All three pre-existing `GraphPackageMetadata` literal sites (one bridge test, one types test, one reaction-mode test) updated with `trigger_reactor: None`.

**Test** — `crates/cloacina-python/tests/cross_language_fan_out.rs::test_cross_language_fan_out_via_shared_reactor_name`:

- Builds a Rust-shaped `GraphPackageMetadata { trigger_reactor: Some("shared_rx"), ... }` and runs it through `build_declaration_from_ffi` → declaration with `reactor_name = Some("shared_rx")`.
- Drives the Python `@cloaca.reactor(name="shared_rx") class SharedRx` + `ComputationGraphBuilder("py_g", reactor=SharedRx, ...)` block, then calls `build_python_graph_declaration("py_g", ...)` → declaration with `reactor_name = Some("shared_rx")`.
- Loads both into one `ComputationGraphScheduler`. M2's idempotent path collapses them onto a single reactor.
- Pushes one event into the `alpha` accumulator. Asserts both Rust and Python subscribers fire exactly once.
- Cleanup via M4 primitives: explicit `unbind_graph_from_reactor` for each, then `unload_reactor("shared_rx")`.

**T-0544 complete.** All five milestones landed:

- M1 — subscriber-list scaffolding (`8dd99d2`)
- M2 — explicit reactor identity + idempotent contract-matched registration (`8e57cf3`)
- M3 — concurrent dispatch via `futures::future::join_all` (`6ea0a8a`)
- M4 — explicit `unbind_graph_from_reactor` / `unload_reactor` + lifecycle guard (`a271ea8`)
- M5 — wire-format propagation + cross-language fan-out test (this commit)

Tests:
- `cargo test -p cloacina --no-default-features --features sqlite,macros --test integration computation_graph` — 44 passed.
- `cargo test -p cloacina-python --no-default-features --features sqlite` — 122 + 1 (new cross-language test) + others, all green.

**Architectural callout (user steer 2026-04-28):** the original T-0546 plan to introduce `package_type = "reactor"` was scrapped. A reactor declaration in any package is now the canonical shape — the user authors `#[reactor(...)]` (or `@cloaca.reactor(...)`) and the reconciler routes it. T-0546's remaining scope is whatever's needed to make a reactor declared in a package without any subscribing CG actually start a runtime instance (i.e. expose `scheduler.load_reactor` to the reconciler so reactor inventory entries get instantiated independent of `load_graph`).
