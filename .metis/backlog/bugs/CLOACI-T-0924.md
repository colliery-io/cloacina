---
id: runtime-and-cg-scheduler
level: task
title: "Runtime and CG scheduler registries are still bare-name keyed across tenants"
short_code: "CLOACI-T-0924"
created_at: 2026-08-06T02:07:44.431110+00:00
updated_at: 2026-08-07T22:49:49.182761+00:00
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

# Runtime and CG scheduler registries are still bare-name keyed across tenants

## Objective

Finish the tenant-keying work T-0921 started. That ticket re-keyed EndpointRegistry (8 maps) and the Python CG registries, but explicitly deferred two larger violators whose fix ripples through public APIs. Same doctrine breach — tenant is THE isolation boundary — just bigger.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Findings (from the T-0921 audit table — see that ticket for the full 26-registry classification)

1. RuntimeInner.{workflows, triggers, computation_graphs, triggerless_graphs, reactors} (crates/cloacina/src/runtime.rs:78-82) are keyed by bare name. Critically, ONE Arc<Runtime> is shared by every tenant's runner via crates/cloacina-server/src/tenant_runner_cache.rs:116 — so two tenants registering the same workflow/trigger/graph name collide in-process. Note RuntimeInner.tasks is ALREADY TaskNamespace-keyed (tenant::package::task) and is the pattern the others should copy.
2. ComputationGraphScheduler.{reactors, graph_to_reactor, graph_topologies} (scheduler.rs:475, 479, 484) — same bare-name keying. list_graphs already carries tenant_id on its output, so the data is available; it just is not in the key.

## Why this was deferred from T-0921

~12 public call sites plus the reconciler's unload bookkeeping, and unload_reactor(&str) is a public API whose signature must change (or gain a tenant-scoped sibling). T-0921 shipped the contained half rather than mixing a large API change into a security fix.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Runtime registries keyed by tenant (mirroring the existing TaskNamespace convention for tasks)
- [x] Scheduler reactor/graph maps keyed by tenant
- [x] unload_reactor and any other name-taking public API resolved: tenant-scoped signature or documented scoping rule
- [x] Same-tenant name collisions across packages are a loud load-time error, matching the policy T-0921 established for EndpointRegistry
- [x] Cross-tenant isolation tests mirroring T-0921's (register same name in two tenants; each resolves its own; unload one, the other survives)

## Status Updates

- 2026-08-06: Filed from CLOACI-T-0921's recorded deferrals (audit rows #5-#7, #10-#11). See T-0921 for the full registry audit table and the EndpointKey/EndpointOwner pattern to follow.

- 2026-08-06 (PHASE 1 — keying convention decided). Work on branch
  `fix/t-0924-runtime-tenant-keying` (worktree `.claude/worktrees/t-0924`), stacked on
  `fix/t-0921-registry-tenant-keying`. NOTE: this ticket file postdates the branch point,
  so it does not exist inside the worktree — status updates are written here.

  **Convention: `(Option<tenant>, name)`, NOT `TaskNamespace`.** New shared module
  `crates/cloacina/src/tenant_scope.rs` lifts T-0921's `EndpointKey`/`EndpointScope`
  shape out of `computation_graph/registry.rs` so Runtime + CG scheduler reuse ONE
  convention rather than inventing a third: `TenantKey { tenant_id, name }`,
  `TenantScope { tenant_id, is_admin }`, `resolve_tenant_key()` (own tenant →
  untenanted → admin-unique-else-Ambiguous), `visible_keys()`.

  Why not `TaskNamespace` (tenant::package::workflow::task): every read site of these
  five registries addresses them by **bare name** and genuinely does not have a package
  — `workflow_executions.workflow_name` (execution_planner/state_manager/context_manager),
  a trigger name off a cron schedule row, a reactor name off a WS path,
  `packaging_bridge` walking `reactor_names()`. `RuntimeInner.tasks` keeps
  `TaskNamespace` precisely because a task's full 4-part namespace IS persisted on
  `task_executions` rows, so its readers have all four components. Package therefore
  belongs in **owner metadata** (used for loud collision detection), not in the key.

  **No `"public"` folding.** Tenant is keyed verbatim, matching T-0921's `EndpointKey`.
  The embedded `DefaultRunner` (`config.tenant_id()` defaults to `"public"`) registers
  reconciler-loaded packages under `Some("public")` and reads with scope
  `Some("public")` → own-tenant hit; inventory/macro entries register untenanted
  (`None`) and are reached via the untenanted fallback. Embedded resolution is
  therefore byte-for-byte what it is today.

  **Scope lives on the `Runtime` handle.** `Runtime` is already `Clone`-shares-`Arc`,
  so it gains a `scope` field and `scoped_to_tenant()` / `untenanted_view()` /
  `admin_view()` views over one shared `RuntimeInner`. That keeps every reader's
  signature (`get_workflow(&str)`) intact while making the tenant structurally
  unavoidable, and mirrors cloacina-python's existing `ScopedRuntime` /
  `RegistrationScope` (T-0921). The two runner construction sites
  (`default_runner/mod.rs`, `default_runner/config.rs`) bind the handle to
  `config.tenant_id()`; everything downstream (TaskScheduler, ThreadTaskExecutor,
  RegistryReconciler, cron `Scheduler`, packaging_bridge) inherits it.
  `seed_from_inventory()` always registers **untenanted** — inventory entries are
  host-binary/compile-time, never tenant-authored, and the reconciler re-calls it
  after every dlopen (so scoping it would cross-stamp one tenant's cdylib entries
  onto the next tenant to load).

  The `ComputationGraphScheduler` is genuinely shared (one `Arc` for all tenants via
  `tenant_runner_cache.rs`), so it cannot carry a scope — its name-taking methods take
  an explicit tenant/scope argument instead.

- 2026-08-06 (ATTEMPT 3 — survey re-confirmed on top of merged main). Worktree
  `.claude/worktrees/t-0924` @ 8af90162 (T-0921 merged as PR #235). The only artefact
  surviving attempts 1-2 is the untracked `crates/cloacina/src/tenant_scope.rs`
  (343 lines, `TenantKey`/`TenantOwner`/`TenantScope`/`resolve_tenant_key`/
  `visible_keys` + 6 unit tests) — NOT yet wired into `lib.rs`. Design above is
  re-affirmed against the merged T-0921 code (`registry.rs:233-400` `EndpointKey`/
  `EndpointOwner`/`EndpointScope`/`resolve_key`, errors at `registry.rs:64,77`).

  CALL-SITE CENSUS (grep, whole workspace):
  * `Runtime` name-taking methods — ~120 hits, but ALL are `runtime.get_x(name)` /
    `register_x(name, ctor)` / `x_names()`. Because the scope rides on the handle,
    NONE of these signatures change. Only the handle-construction sites do.
    Production readers: `execution_planner/{mod.rs:348,context_manager.rs:57,
    state_manager.rs:110}`, `cron_trigger_scheduler.rs:814`,
    `computation_graph/packaging_bridge.rs:950-951`,
    `registry/loader/constructor_loader.rs:937`,
    `registry/reconciler/loading.rs` (~20 sites: 379,846,905,914,927,996,1008,1265,
    1415,1481-83,1515-19,1567-71,1589-93,1871,1894,1905,2010,2030,2050,2182),
    `cloacinactl/src/commands/daemon.rs:479,532`, `cloacina-python/src/{reactor.rs:131,
    workflow.rs:211,420,loader.rs:340,377,395,406,bindings/trigger.rs:278}`,
    `examples/features/workflows/event-triggers/src/main.rs:442`.
  * `ComputationGraphScheduler` name-taking methods DO change (shared Arc, no handle
    scope): `load_reactor`(already takes tenant_id), `bind_graph_to_reactor`,
    `unbind_graph_from_reactor`, `unload_reactor`, `unload_graph`,
    `reactor_accumulator_names`, `load_graph_split`(already takes tenant_id),
    `load_graph`(decl carries tenant_id). Non-test callers:
    `cloacina-server/src/lib.rs:2870`, `computation_graph/embedded.rs:61`,
    `computation_graph/packaging_bridge.rs:991,1063`,
    `registry/reconciler/loading.rs:751,783,931,956,2134,2192`.
    Test callers: `tests/integration/computation_graph.rs` (~30),
    `tests/constructor_reactor_scheduler_wasm.rs:223,309`,
    `tests/integration/dal/reconciler_e2e_load.rs:201,241,273,296,413`,
    `cloacina-python/tests/{python_reactor_library.rs,cross_language_fan_out.rs}`.

- 2026-08-06 (STEP 1 DONE — `Runtime` re-keyed; `cargo check -p cloacina` clean).
  * `tenant_scope.rs` wired into `lib.rs` (`pub mod tenant_scope` + re-export of
    `TenantKey`/`TenantOwner`/`TenantResolveMiss`/`TenantScope`).
  * `runtime.rs`: new private `ScopedRegistry<V>` (kind label + `RwLock<HashMap<
    TenantKey, ScopedEntry<V>>>` where `ScopedEntry` = owner + constructor) replaces
    the five bare `HashMap<String, _>` maps. `tasks` (TaskNamespace) and
    `stream_backends` (backend KIND) deliberately unchanged. New public error
    `RuntimeRegistrationError::OwnershipConflict` worded like T-0921's
    `EndpointOwnershipConflict`.
  * `Runtime` = `Arc<RuntimeInner>` + owned `RuntimeScope{tenant_id,is_admin}`.
    New: `scoped_to_tenant()`, `untenanted_view()`, `admin_view()`, `tenant_id()`,
    `is_admin()`, `shares_registries_with()` (replaces `Arc::ptr_eq` on
    `Arc<Runtime>`), `{workflow,trigger,computation_graph,triggerless_graph,
    reactor}_keys()`, `try_register_{workflow,trigger,computation_graph,
    triggerless_graph,reactor}(&TenantOwner, ...) -> Result`, and
    `may_claim_{trigger,triggerless_graph}(&TenantOwner, name) -> Result<bool>`.
    EVERY pre-existing signature is untouched (`get_workflow(&str)` etc.) — the
    scope rides on the handle.
  * `seed_from_inventory` registers via `self.untenanted_view()` regardless of the
    handle's scope (unit-tested).
  * Handle binding: `default_runner/mod.rs::with_database_secrets` and
    `default_runner/config.rs` build path both wrap the incoming/shared runtime in
    `.scoped_to_tenant(config.tenant_id())`. This is THE choke point that fixes the
    shared-`Arc` collision; everything downstream clones the handle.
  * DECISION (differs from the phase-1 note, and matters for the embedded
    constraint): the RECONCILER does NOT re-scope to `config.default_tenant_id`.
    It writes through whatever scope its `Arc<Runtime>` carries. Rationale: an
    embedded user who hands their own untenanted `Runtime` to the builder and then
    reads `rt.get_workflow(name)` off THAT handle must still see reconciler-loaded
    packages. Self-scoping the reconciler to `"public"` would have broken exactly
    that read. In the server the runner has already bound the handle to the
    tenant, so isolation still holds; the two are consistent because
    `services.rs:203` sets `default_tenant_id = config.tenant_id()`.
  * Loud collisions wired in `registry/reconciler/loading.rs`: workflow
    (`try_register_workflow`), computation graph (`try_register_computation_graph`),
    custom triggers and trigger-less graphs (`may_claim_*` replacing the bare
    "already registered? reuse it" guard, then `try_register_*`). All map to
    `RegistryError::RegistrationFailed`. The `may_claim_*` change also closes a
    latent teardown bug: package B used to silently adopt package A's same-named
    trigger and then unregister it on B's unload.
  * 11 new unit tests in `runtime.rs` (two-tenant isolation, scoped unregister,
    invisibility, loud cross-package conflict + same-owner replace, cross-tenant
    same name OK, untenanted path unchanged, untenanted visible from tenant view,
    own-tenant shadowing + dedup, admin unique/ambiguous, reactors+graphs, always-
    untenanted inventory seeding). ALL 17 `runtime::tests::*` PASS.

- 2026-08-06 (STEP 2 DONE — `ComputationGraphScheduler` re-keyed; all 9
  `computation_graph::scheduler::tests::*` PASS).
  * `reactors: HashMap<TenantKey, RunningGraph>`,
    `graph_to_reactor: HashMap<TenantKey, TenantKey>` (VALUE is a full key, so a
    tenant graph bound to an untenanted upstream still points at the exact entry),
    `graph_topologies: HashMap<TenantKey, String>`.
  * KEYING RULE, documented at each site: a LOAD/BIND is a CLAIM → addresses
    `scope.own_key(name)` exactly, never the fallback. A LOOKUP/UNLOAD is a
    RESOLUTION → `resolve_tenant_key` (own tenant → untenanted → admin-unique).
  * Public signature changes (scheduler holds no scope — it is a single shared
    `Arc`): `bind_graph_to_reactor(graph, reactor, TenantScope, graph_fn)`,
    `unbind_graph_from_reactor(name, TenantScope) -> TenantKey` (was `String`),
    `unload_reactor(name, TenantScope)`, `unload_graph(name, TenantScope)`,
    `reactor_accumulator_names(name, TenantScope)`. UNCHANGED: `load_reactor`
    (already took `tenant_id`), `load_graph`/`load_graph_split` (decl carries it),
    `list_graphs`/`list_reactors` (now source `tenant_id` from the KEY),
    `set_graph_executor`, `check_and_restart_failed`, `shutdown_all`.
  * `PlannedRestart::{Reactor,Accumulator}` carry `reactor_key: TenantKey`;
    `restart_{reactor,accumulator}_after_backoff` take `&TenantKey`. Metric and
    log labels still use the BARE name, so the `cloacina_component_health` /
    `cloacina_supervisor_restarts_total` label vocabulary is unchanged.
  * `shutdown_all` now unloads each graph in ITS OWN scope (was implicitly
    untenanted, which post-keying would have skipped every tenant's graphs).
  * New guard in `bind_graph_to_reactor`: the per-reactor subscriber map is still
    keyed by bare graph name (the dispatcher labels results with it), so two
    tenants binding same-named graphs to one UNTENANTED reactor now errors
    loudly instead of silently replacing. Re-keying that map is out of scope
    (see residuals).
  * `RegistryReconciler::tenant_scope()` (`mod.rs`) returns
    `TenantScope::tenant(&config.default_tenant_id)` and is threaded into the 4
    scheduler call sites in `loading.rs` (751, 931, 956, 2173-ish).
  * 6 new scheduler tests: two tenants coexist under one name, tenant-scoped
    unload, other tenant unreachable (4 methods), untenanted lifecycle unchanged
    + reachable from a tenant view, tenant graph binding an untenanted upstream
    (and unload following the key back), same-tenant duplicate rejected.
  * `cloacina-server/src/lib.rs` shared-runtime test rewritten: `Arc::ptr_eq` on
    `Arc<Runtime>` no longer holds (each runner has its own scoped handle) →
    now asserts `shares_registries_with` plus DIFFERENT `tenant_id()` per runner.

- 2026-08-06 (VALIDATION COMPLETE — all acceptance criteria checked; changes left
  UNCOMMITTED in `.claude/worktrees/t-0924` per instruction). 12 files changed,
  +1478/-247, plus new `crates/cloacina/src/tenant_scope.rs`.

  COMPILES CLEAN: `cargo check -p cloacina --no-default-features --features
  postgres,sqlite`; `-p cloacina-server`; `-p cloacina-python`; `-p cloacinactl`;
  `-p cloacina-agent`; `cargo check --workspace --tests`; and the suite that has
  twice caught stale callers, `cargo check -p cloacina --features
  constructors-wasm --tests`. `cargo fmt --all` applied, `--check` clean.

  TESTS: `cargo test -p cloacina --lib` = 792 pass (includes the 17
  `runtime::tests` and 9 `computation_graph::scheduler::tests`). Postgres
  integration lane = 334 pass / 4 fail; sqlite lane = 7 pass / 0 fail;
  `cargo test -p cloacina-server --lib` = 207 pass; constructors-wasm targets
  (`constructor_trigger_wasm`, `constructor_reactor_scheduler_wasm`) = 6 pass;
  cloacina-python (`python_reactor_library`, `cross_language_fan_out`,
  `trigger_packaging`) = 9 pass. `computation_graph` = 50/50;
  `dal::reconciler_e2e_load` = 2/2 (needed `cargo build` of the
  reactor-only-rust / reactor-subscriber-rust / mixed-rust fixtures first).

  THE 4 POSTGRES FAILURES ARE PRE-EXISTING, PROVEN NOT MINE: `scheduler::
  cron_basic::{test_cron_schedule_with_recovery_config,
  test_default_runner_cron_integration, test_workflow_instance_register_roundtrip}`
  fail IDENTICALLY on the stashed (unmodified merged-main) tree — verified by
  `git stash push -u` + rerun. They assert `stats.total_executions == 0` against
  a shared, never-reset dev Postgres; the count climbs run over run (7 on the
  modified tree, 14 on the stashed one). `signing::reconciler_did_check::
  postgres_tests::test_find_signature_present_and_absent` and
  `registry_simple_functional_test::{test_registry_api_simplification,
  test_registry_with_simple_binary_data}` are order-dependent on the same shared
  schema and PASS in isolation.

  DELIBERATELY OUT OF SCOPE (residuals for a follow-up, not regressions):
  1. `RunningGraph.subscribers` is still keyed by BARE graph name — the reactor
     dispatcher labels per-subscriber results with it, so re-keying reaches into
     `GraphResult` plumbing. Only reachable when two tenants bind same-named
     graphs to one UNTENANTED reactor, which cannot happen server-side (every
     packaged load stamps `default_tenant_id`). Mitigated: that bind now errors
     loudly instead of silently replacing.
  2. `endpoint_registry_keys` on `RunningGraph` stays `Vec<String>`; it is
     already paired with an owner-stamped `EndpointOwner` from T-0921, so
     deregistration is owner-scoped and correct.
  3. Reactor registrations still carry NO package provenance into
     `EndpointOwner.package` (`load_reactor` never receives it) — T-0921's own
     recorded residual, unchanged here.
  4. `may_claim_*` was added only for triggers and trigger-less graphs (the two
     loader paths with a "reuse what is already registered" fast path). Workflows
     and CGs register unconditionally and use `try_register_*` directly; reactors
     are never registered by the reconciler (Rust = inventory/FFI, Python = the
     Python loader), so they got neither.
  5. `RuntimeInner.tasks` (TaskNamespace) and `stream_backends` (backend KIND)
     are untouched — T-0921's audit classified both as already-safe.

- 2026-08-07: COMPLETED — PR #245 merged (squash). Closes the tenant-keying class
  that T-0921 opened; every registry in that ticket's 26-row audit is now either
  tenant-keyed or classified as already-safe.

  The two design calls that departed from this ticket's own filed premise, both
  upheld through review and CI:

  1. NOT `TaskNamespace`, despite finding #1 naming it as "the pattern the others
     should copy". Every read site of these five registries addresses them by BARE
     NAME with no package in hand — `workflow_executions.workflow_name`, a trigger
     name off a cron row, a reactor name off a WS path, `packaging_bridge` walking
     `reactor_names()`. `tasks` can keep `TaskNamespace` only because the full
     4-part namespace is PERSISTED on `task_executions` rows, so its readers have
     all four components. Package therefore lives in owner metadata for loud
     collision detection, not in the key.

  2. The reconciler does NOT re-scope to `default_tenant_id` (contradicting the
     phase-1 note). It writes through whatever scope its `Arc<Runtime>` carries,
     because an embedded user who hands their own untenanted Runtime to the builder
     and then reads `rt.get_workflow(name)` off that same handle must still see
     reconciler-loaded packages. Server isolation still holds — the runner binds the
     tenant first, and `services.rs:203` keeps the two consistent.

  Scope-on-the-handle is why this landed without churn: `Runtime` already
  Clone-shares its `Arc`, so ZERO pre-existing signatures changed and all ~120
  call sites were untouched. The `ComputationGraphScheduler` is one genuinely
  shared `Arc` and could not carry a scope, so its five name-taking methods took
  an explicit `TenantScope` instead.

  MERGE-ORDER NOTE: this was the last of three PRs contending for the same files.
  Merging #244 (T-0925) put it in conflict on
  `crates/cloacina/src/computation_graph/scheduler.rs`, and unlike the earlier
  mechanical `loader.rs` conflicts this one was SEMANTIC: T-0925 had converted
  `load_reactor` into a wrapper delegating to a new `load_reactor_in` carrying
  `provider_root`, while this branch was inserting its `reactor_key` claim into
  the same function. Dropping either side would have silently disabled one
  ticket's isolation guarantee with no compile error. Both were kept and the
  result was PROVEN rather than assumed — the shared reactor-scheduler suite (2),
  T-0925's cross-tenant provider isolation (3), and this ticket's
  `computation_graph` suite (60) all pass on the merged code.

  Residuals 1-5 above stand as recorded; #1 (`RunningGraph.subscribers` bare-name
  keyed) is the only one with a live, if server-unreachable, failure mode, and it
  is mitigated with a loud error rather than silent replacement.
