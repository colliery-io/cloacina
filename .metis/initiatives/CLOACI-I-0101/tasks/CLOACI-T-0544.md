---
id: t-02b-multi-graph-per-reactor-fan
level: task
title: "T-02b: Multi-graph-per-reactor fan-out"
short_code: "CLOACI-T-0544"
created_at: 2026-04-25T15:08:05.348610+00:00
updated_at: 2026-04-28T12:24:34.785069+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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
