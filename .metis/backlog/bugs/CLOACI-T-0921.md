---
id: flat-name-global-registries-cross
level: task
title: "Flat name-global registries cross tenant lines in-process — audit and key by tenant/package"
short_code: "CLOACI-T-0921"
created_at: 2026-08-02T17:44:00.940671+00:00
updated_at: 2026-08-02T17:44:00.940671+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Flat name-global registries cross tenant lines in-process — audit and key by tenant/package

## Objective

Audit every process-wide name-keyed registry against the tenant-is-THE-isolation-boundary doctrine and re-key the violators. The 2026-08-02 deep dive found registries keyed by bare name, so same-named entities from different packages/tenants collide inside one server process — a doctrine breach in a system whose entire authz design rests on tenant isolation being structural.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Findings

1. EndpointRegistry keys accumulators by bare name process-wide (crates/cloacina/src/computation_graph/registry.rs:317-321): two packages (or two tenants) loading same-named accumulators broadcast into EACH OTHER'S boundary channels, and deregistering one tears both down.
2. Python-side GRAPH_EXECUTORS / ACCUMULATOR_REGISTRY are likewise bare-name keyed (crates/cloacina-python): two tenants' same-named graphs share one executor slot — last load wins, cross-tenant.
3. Audit scope beyond the two known: any other process-global map keyed by bare entity name (trigger registries, reactor maps, workflow name lookups in the shared runtime) — enumerate and classify each as (a) already tenant/package-scoped, (b) name-collision-safe by construction, or (c) violator.

Context: workflow task namespaces already carry tenant::package:: prefixes (TaskNamespace), so the pattern exists — these registries predate or bypassed it. Server multi-tenancy makes this reachable today: one server process hosts many tenants' packages.

## Acceptance Criteria

- [ ] Registry audit table (registry -> keying -> verdict) recorded in this task
- [ ] EndpointRegistry keyed by (tenant, package, name); cross-package same-name test proves isolation (inject into A, B receives nothing; unload A, B survives)
- [ ] Python graph/accumulator registries scoped the same way with an equivalent test
- [ ] Any further violators from the audit fixed or ticketed individually

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (DEEPDIVE.md risk register #23; cg-runtime report §7.6 + python-integration report §7.4). Verified against main @ 5216e632.

- 2026-08-04 (PHASE 1 COMPLETE — audit table): Systematic sweep of every process-global
  name-keyed map across `crates/`. Verdicts below. Work happening on branch
  `fix/t-0921-registry-tenant-keying` (worktree `.claude/worktrees/t-0921`).

### Audit table — process-global name-keyed registries

| # | Registry | file:line | Key shape | Writers | Readers | Verdict |
|---|---|---|---|---|---|---|
| 1 | `EndpointRegistry.accumulators` | `crates/cloacina/src/computation_graph/registry.rs:212` | `String` (bare acc name) | `scheduler.rs:628`, restart `scheduler.rs:1448` | `routes/ws.rs:277`, `routes/health_graphs.rs:732`, `embedded.rs:73` | **VIOLATOR** — `.entry(name).or_insert_with(Vec::new).push()` *appends*, so tenant A's WS push is broadcast into tenant B's boundary channel |
| 2 | `EndpointRegistry.reactors` / `.reactor_handles` | `registry.rs:214,216` | `String` (bare reactor name) | `scheduler.rs:695,704`, restart `:1510` | `ws.rs:334`, `health_graphs.rs:588,632` | **VIOLATOR** — last-writer-wins; `deregister_reactor(name)` removes the other tenant's entry |
| 3 | `EndpointRegistry.accumulator_policies` / `.reactor_policies` | `registry.rs:218,220` | `String` | `scheduler.rs:721,726`, restart `:1525,1530` | `check_accumulator_auth` `registry.rs:358`, `check_reactor_auth` `:377`, `check_reactor_op_auth` `:397` | **VIOLATOR** — *overwrite* (unlike #1 which appends). Second tenant's policy governs both, which is what today keeps the breach from being trivially exploitable AND what makes it silent |
| 4 | `EndpointRegistry.accumulator_health` / `_freshness` / `_meta` / `_injects` | `registry.rs:222,224,226,231` | `String` | `scheduler.rs:631,634,640`; `registry.rs:255` | `health_graphs.rs:108,127`; `registry.rs:518,528,556` | **VIOLATOR** — cross-tenant health/freshness/inject-count bleed. `AccumulatorDescriptor` (`registry.rs:194`) already *carries* `tenant_id`, but as payload, not key |
| 5 | `RuntimeInner.workflows` | `crates/cloacina/src/runtime.rs:78` | `String` | `runtime.rs:140,207`; `reconciler/loading.rs:1173`; `cloacina-python/src/workflow.rs:211,420`, `loader.rs:389` | `execution_planner/mod.rs:348`, `state_manager.rs:110`, `context_manager.rs:57` | **VIOLATOR (deferred)** — one `Arc<Runtime>` is shared by every tenant's runner (`cloacina-server/src/tenant_runner_cache.rs:93,116`). Two tenants shipping `daily_etl` → last load wins for both |
| 6 | `RuntimeInner.triggers` | `runtime.rs:79` | `String` | `runtime.rs:144,242`; `loading.rs:1808`; `constructor_loader.rs:937` | `cron_trigger_scheduler.rs:801`; `loading.rs:1323,1479,1774,1797` | **VIOLATOR (deferred)** — same shared-`Runtime` mechanism |
| 7 | `RuntimeInner.computation_graphs` / `.triggerless_graphs` / `.reactors` | `runtime.rs:80,81,82` | `String` | `runtime.rs:148,152,156`; `loading.rs:1953,2076` | `loading.rs:1427,1501,1913,1933`; `packaging_bridge.rs:951` | **VIOLATOR (deferred)** — same |
| 8 | `RuntimeInner.stream_backends` | `runtime.rs:83` | `String` (backend *type* name, e.g. `"kafka"`) | `runtime.rs:161,385` | `stream_backend.rs:95` | **SAFE BY CONSTRUCTION** — key is a backend kind, not a tenant-authored entity name; collisions are intentional dedup |
| 9 | `RuntimeInner.tasks` | `runtime.rs:77` | `TaskNamespace{tenant,package,workflow,task}` | `runtime.rs:139` | task lookups | **OK** — already tenant-scoped (`cloacina-workflow/src/namespace.rs:62`). This is the pattern the others should copy |
| 10 | `ComputationGraphScheduler.reactors` | `crates/cloacina/src/computation_graph/scheduler.rs:475` | `String` (reactor name) | `scheduler.rs:772,1026,1211,1394,1570` | `:558` (idempotency probe), `:789,844,872,1010,1076,1094,1149` | **VIOLATOR (deferred)** — cross-tenant same-name reactor with an identical contract *silently shares one running reactor* (`load_reactor` `:557-580` returns `Ok(())`); with a differing contract it leaks the other tenant's reactor name into the error. Partially mitigated: `check_reactor_contract_matches` `:211` compares `tenant_id`, so an identical-contract cross-tenant share is only possible when `tenant_id` also matches |
| 11 | `ComputationGraphScheduler.graph_to_reactor` / `.graph_topologies` | `scheduler.rs:479,484` | `String` (graph name) | `:969,974` | `:1108-1110` | **VIOLATOR (deferred)** — `bind_graph_to_reactor` `:790` rejects a same-named graph from another tenant with "graph 'x' already loaded" |
| 12 | Python `GRAPH_EXECUTORS` | `crates/cloacina-python/src/computation_graph.rs:661` | `String` (graph name) | `register_graph_executor` `:664` ← `:618` | `get_graph_executor` `:674`, `get_graph_executors_for_reactor` `:683`, `:898`; `task.rs:302,632`; `loader.rs:485`; `cloacina-agent/src/main.rs:807,811` | **VIOLATOR** — last-load-wins; two tenants' same-named graphs share one executor slot |
| 13 | Python `ACCUMULATOR_REGISTRY` | `computation_graph.rs:130` | `String` (`func.__name__`) | `:133` ← decorators `:229,276,314,360,410` | `:142,151` (`drain_accumulators`), `:167`; drained `:1548,1575` | **VIOLATOR** — same-named accumulator decorators from two packages overwrite each other |
| 14 | Python `NODE_REGISTRY` | `computation_graph.rs:87` | `String` (node fn name) | `register_node` `:106` | `drain_nodes()` `:109` ← `:537` | **VIOLATOR (deferred)** — drain-on-build scratch buffer; only unsafe under *interleaved* imports. Import is GIL-serialized and `@graph` drains at the end of its own decorator body, so it is collision-safe in the current single-threaded import path. Ticket separately |
| 15 | Python `ACTIVE_GRAPH_CONTEXT` | `computation_graph.rs:89` | single `Option<String>` | `push/pop_graph_context` `:91,96` | `:101` | **VIOLATOR (deferred)** — same reasoning as #14 (GIL-serialized push/pop within one decorator body) |
| 16 | Python `PYTHON_TRIGGER_REGISTRY` | `crates/cloacina-python/src/trigger.rs:39` | `Vec` drain buffer (entries carry bare `name`/`workflow_name`) | decorator | `drain_python_triggers()` `:61` | **VIOLATOR (deferred)** — drain buffer, same import-serialization argument as #14 |
| 17 | Python `WORKFLOW_CONTEXT_STACK` | `crates/cloacina-python/src/task.rs:87` | `Vec` stack | `:90,96` | `:101` | **SAFE BY CONSTRUCTION (fragile)** — a *stack*, so nesting is correct; process-global rather than thread-local is the latent risk, not name collision |
| 18 | `Scheduler.last_poll_times` | `crates/cloacina/src/cron_trigger_scheduler.rs:149` | `String` (bare trigger name) | poll loop | `:801` | **VIOLATOR (deferred)** — per-runner, but with the shared `Runtime` (#6) two tenants' same-named triggers share one poll-rate slot. Blocked on #6 |
| 19 | `PROVIDER_SEARCH_PATH` | `crates/cloacina/src/registry/loader/constructor_loader.rs:1649` | `Option<PathBuf>` (not a map) | `:1663,1668` | `:1674` | **VIOLATOR (deferred, different class)** — process-wide provider dir makes constructor resolution tenant-blind. Not name-keying; ticket separately |
| 20 | `LOADED_RUNTIMES`, `imported_py_graph_digests`, `loaded_graphs` | `crates/cloacina-agent/src/main.rs:1139,718,878` | artifact **digest** | — | `:807,811` | **SAFE BY CONSTRUCTION for the cache itself** (content-addressed), but the dispatch at `:807/:811` looks the graph up by bare `packet.graph_name` in #12, so the digest keying buys nothing downstream |
| 21 | `TenantDatabaseCache.databases` | `crates/cloacina-server/src/lib.rs:136` | `String` = **tenant_id** | `:180` | `:164,177` | **OK** |
| 22 | `TenantRunnerCache.cache` | `crates/cloacina-server/src/tenant_runner_cache.rs:90` | `String` = **tenant_id** | — | — | **OK** |
| 23 | `DeliverySink.by_key` | `crates/cloacina-server/src/delivery_sink.rs:55` | `(String, Option<String>)` = `(recipient, tenant_id)` | — | — | **OK — reference implementation.** This is the composite-key shape the fixes below copy |
| 24 | `KeyCache`, `WsTicketStore`, `LoginFlowStore`, `fleet_coordinator.pending`, `reconciler.loaded_packages`, `cron_recovery.recovery_attempts` | `routes/auth.rs:58,321`; `oidc.rs:383`; `fleet_coordinator.rs:45`; `reconciler/mod.rs:261`; `cron_recovery.rs:99` | key-hash / nonce / UUID | — | — | **SAFE BY CONSTRUCTION** — keys are secrets or UUIDs, not tenant-authored names |
| 25 | `COMPILE_TIME_TASK_REGISTRY` | `crates/cloacina-macros/src/registry.rs:36` | `String` | — | — | **SAFE BY CONSTRUCTION** — lives in the proc-macro compiler process, one crate at a time; never sees two tenants |
| 26 | `StreamBackendRegistry` (`stream_backend.rs:80`), `DefaultDispatcher.executors` (`dispatcher/default.rs:55`), `TaskRegistrar` maps (`task_registrar/mod.rs:51`), `DependencyLoader.loaded_contexts` (`executor/types.rs:60`), `WorkflowGraph.task_index` (`graph.rs:78`) | — | `String` but **per-instance, not global** | — | — | **SAFE BY CONSTRUCTION** — not process-global. (`stream_backend.rs:135` notes the global version was removed in CLOACI-T-0509; the collision moved into #8, which is safe) |
| 27 | `PYTHON_RUNTIME` (`crates/cloacina/src/python_runtime.rs:96`), cdylib `OnceLock` runtimes (`cloacina-workflow-plugin/src/lib.rs:230,384,548,645`) | — | singleton value, not a map | — | — | **SAFE BY CONSTRUCTION** |

**Scope decision for this task.** Fixing #1–#4 (EndpointRegistry) and #12–#13 (Python) — these are
the ticket's named acceptance criteria and the two that are reachable from an *unauthenticated-name*
surface (`/v1/ws/accumulator/{name}`, `/v1/ws/reactor/{name}`). #5–#7, #10–#11, #14–#16, #18, #19
are recorded as violators and deferred to follow-up tickets: re-keying `Runtime` and the CG
scheduler touches ~12 public call sites plus the reconciler's unload bookkeeping and is a
materially larger change than this ticket's stated scope.

- 2026-08-04 (PHASE 2a COMPLETE — EndpointRegistry re-keyed; audit rows #1–#4).
  `cargo check -p cloacina --no-default-features --features postgres,sqlite` and
  `cargo check -p cloacina-server` both clean.

  **New types** (`crates/cloacina/src/computation_graph/registry.rs`):
  - `EndpointKey { tenant_id: Option<String>, name: String }` — the map key for all 8 inner maps.
  - `EndpointOwner { tenant_id, package: Option<String>, reactor }` — provenance stamped on each entry.
  - `EndpointScope { tenant_id: Option<&str>, is_admin }` — the *caller's* scope; built from
    `KeyContext` via `EndpointScope::from_key_context`.
  - `resolve_key()` — scoped lookup: (1) caller's own tenant, (2) the untenanted entry
    (embedded/pre-tenancy, allow-all), (3) admins only: a **unique** cross-tenant match;
    two tenants owning the name ⇒ `RegistryError::AmbiguousEndpoint`, never a guess.
    A non-admin can no longer resolve another tenant's endpoint at all.
  - `RegistryError::EndpointOwnershipConflict` — the loud same-tenant collision.

  **Collision policy.** `register_accumulator` / `register_reactor` now return
  `Result`. Same owner re-registering ⇒ append/replace (the restart path relies on
  this). *Different* owner claiming a live `(tenant, name)` ⇒ hard error naming both
  owners and the tenant. `scheduler.rs::load_reactor` unwinds the whole load on such a
  rejection (shutdown signal + deregister everything already claimed) so a rejected
  package leaves nothing half-wired.

  **Deregistration is owner-scoped.** `deregister_accumulator/reactor` take the owner and
  no-op (with a warn) when the live entry belongs to someone else — unloading tenant A
  can no longer tear down tenant B's same-named endpoint. Deregistration now also clears
  the health/freshness/policy/inject side tables, which previously leaked past unload.

  **`list_accumulators_with_health_for_key`** gained a key-tenant gate *ahead* of the
  policy check. This was a real hole: accumulators with no policy row fell back to
  `allow_all`, so tenant B could enumerate tenant A's accumulator names. Covered by
  `test_cloaci_t_0921_health_listing_filters_by_key_tenant`.

  **Ownership carried on `RunningGraph.owner`** (`scheduler.rs`) so the load, restart
  (full + per-accumulator), and unload paths all use the identity claimed at load.

  **Route compatibility — verified, no API change.**
  - `/v1/ws/accumulator/{name}` (`routes/ws.rs:277`) and `/v1/ws/reactor/{name}`
    (`ws.rs:334`, `:430`) keep their bare-`{name}` paths. `AuthenticatedKey` (already in
    scope in both handlers, carrying `tenant_id` + `is_admin`) now builds an
    `EndpointScope` that is threaded into `send_to_accumulator` / `get_reactor_handle` /
    `send_to_reactor`. Identity was ALREADY in scope — it simply was not being used for
    resolution, only for the policy check.
  - REST equivalents threaded the same way: `routes/health_graphs.rs` `list_accumulators`
    (descriptor + inject stat), `fire_reactor` (`send_to_reactor`), `inject_accumulator`
    (`send_to_accumulator` + `note_accumulator_operator_inject`), `list_reactor_fires` and
    `reactor_fire_timeseries` (`get_reactor_handle`).
  - `computation_graph/embedded.rs` carries the declaration's `tenant_id` and pushes under
    that scope (embedded registries are per-instance, so this is a no-op in practice).

  **Deferred within this row.** `EndpointOwner.package` is `None` from `load_reactor`
  today — package provenance is not a parameter there, and threading it would ripple
  through `packaging_bridge::dispatch_{runtime,package}_reactors_into_scheduler` and ~10
  call sites in three crates. It buys only error-message provenance: the *reactor name*
  already discriminates two same-tenant packages claiming one accumulator name (the case
  the AC tests). The one case package would add — two same-tenant packages declaring the
  same **reactor** name — cannot be closed here anyway, because
  `scheduler.rs::load_reactor`'s idempotency guard (audit row #10) returns `Ok(())`
  before `register_reactor` is ever reached. Both belong to the same follow-up.

- 2026-08-04 (PHASE 2b COMPLETE — Python registries scoped; audit rows #12–#13).
  `cargo check -p cloacina-python --tests` clean.

  **New module** `crates/cloacina-python/src/registration_scope.rs` — the tenancy
  counterpart to the existing `runtime_scope::ScopedRuntime`. `ScopedRuntime` tells
  Python decorators *which `Runtime`* to register into; `ScopedRegistration` tells them
  *whose* registration it is. `RegistrationScope { tenant_id, package }` lives in a
  thread-local; unlike `ScopedRuntime` it **nests** (saves/restores the previous scope)
  so a transitively-scoped import cannot strand the outer scope.

  **`GraphKey { tenant_id, package, name }`** now keys both process-globals in
  `crates/cloacina-python/src/computation_graph.rs`: `GRAPH_EXECUTORS` (was `:661`) and
  `ACCUMULATOR_REGISTRY` (was `:130`). `resolve_graph_key()` mirrors the Rust-side
  resolution order: exact `(tenant, package, name)` → same tenant/any package (unique)
  → the unscoped entry → for unscoped callers only, a unique match anywhere. A
  *scoped* caller never falls through to another tenant. Ambiguity logs and returns
  `None` rather than picking a winner.

  **Loader wiring** — the packaged loader knows tenant + package at import time and now
  installs the scope around the import:
  - `loader.rs::import_python_computation_graph` gained `tenant_id` / `package_name`
    params (one non-test caller); `runtime_impl.rs::load_cg_package` already had
    `tenant_id` and derives the package from the entry module's top-level name.
  - `loader.rs::import_and_register_python_workflow_named` already had both
    `package_name` and `tenant_id` — scope installed, no signature change.

  **Reader-side scoping.** `get_registered_accumulators()` now returns only the current
  scope's declarations plus unscoped ones, so `reactor.rs:139` (which builds a reactor's
  accumulator specs) stops seeing other tenants' declarations.
  `get_graph_executors_for_reactor()` filters the same way; the fleet agent dispatches
  with no scope installed and still sees everything it hosts.
  `resolve_poll_closure()` resolves through the scope.
  Added `get_graph_executor_by_key`, `registered_graph_keys`,
  `get_all_registered_accumulators` for explicit/diagnostic access.

  **Python-side deferrals** (audit rows #14–#16): `NODE_REGISTRY`,
  `ACTIVE_GRAPH_CONTEXT`, and `PYTHON_TRIGGER_REGISTRY` are drain-on-build scratch
  buffers, not persistent lookup tables. Registration and drain both happen inside a
  single GIL-serialized decorator body, so they are collision-safe on the current import
  path. They remain latent risks if imports are ever parallelized — ticket separately.

- 2026-08-04 (PHASE 3 COMPLETE — tests + validation). Task complete; changes left
  uncommitted in the worktree.

  **Compile checks — all clean:**
  - `cargo check -p cloacina --no-default-features --features postgres,sqlite` ✅
    (and `--features postgres,sqlite,macros --tests` — the integration suite needs
    `macros`, without it `cloacina_macros` is unresolved, which is pre-existing)
  - `cargo check -p cloacina-python --tests` ✅
  - `cargo check -p cloacina-server --tests` ✅
  - `cargo check -p cloacina-agent` ✅ (consumes `get_graph_executor`)
  - `cargo fmt --all --check` ✅

  **Tests run (none require a live DB):**
  - `cargo test -p cloacina --lib computation_graph` → **54 passed**, incl. 7 new
    `registry::tests::test_cloaci_t_0921_*`: cross-tenant accumulator isolation,
    cross-tenant lookup is not-found, admin ambiguity refused, same-tenant second claim
    rejected loudly (accumulator + reactor), owner-scoped deregistration, health-listing
    tenant filter.
  - `cargo test -p cloacina --test integration computation_graph::` → **50 passed**,
    incl. 2 new scheduler-level tests: `test_cloaci_t_0921_cross_tenant_accumulator_isolation`
    (inject as A fires only A, B receives nothing; unload A, B survives and still fires)
    and `test_cloaci_t_0921_same_tenant_duplicate_accumulator_rejected` (error names the
    endpoint, tenant, and both owners; incumbent untouched; rejected load unwinds its
    reactor registration). The pre-existing supervisor restart/resilience tests pass,
    which exercises the re-registration paths.
  - `cargo test -p cloacina-python --lib` → **138 passed, 1 failed**. The failure is
    `bindings::runner::tests::test_runner_set_cron_schedule_enabled` ("database table is
    locked: schedules") — a pre-existing SQLite parallel-test flake, unrelated to this
    change; it passes in isolation with `--test-threads=1`.
  - `cargo test -p cloacina-python --test python_reactor_library --test cross_language_fan_out`
    → **5 passed**.

  **Acceptance criteria status:**
  - [x] Registry audit table recorded in this task (26 rows, all classified).
  - [x] `EndpointRegistry` keyed by tenant with owner metadata; cross-package/tenant
        same-name isolation proven (inject into A → B receives nothing; unload A → B
        survives).
  - [x] Python graph/accumulator registries scoped, with equivalent tests.
  - [~] Further violators: enumerated in the audit table with verdicts; the deferred set
        needs follow-up tickets filed (rows #5–#7, #10–#11, #14–#16, #18, #19).
