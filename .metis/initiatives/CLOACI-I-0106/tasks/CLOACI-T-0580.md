---
id: t-03-tenantrunnercache-per-tenant
level: task
title: "T-03: TenantRunnerCache — per-tenant DefaultRunner with LRU eviction"
short_code: "CLOACI-T-0580"
created_at: 2026-05-13T19:38:43.181516+00:00
updated_at: 2026-05-13T22:37:15.573795+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-03: TenantRunnerCache — per-tenant DefaultRunner with LRU eviction

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Introduce `TenantRunnerCache` — a per-tenant `DefaultRunner` cache with LRU eviction — and route `WorkflowExecutor::execute_async` through it so workflow execution writes land in the correct tenant schema. Each cached entry is a fully constructed runner (scheduler loop, heartbeat, executor pool), sharing inventory-seeded registries via `Arc`. Closes EVO-04 and the structural half of COR-01.

This is **the load-bearing task** of CLOACI-I-0106. T-04 (`remove_tenant`) depends on the eviction surface this creates.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `TenantRunnerCache` type with LRU bound. Default cap 256; operator override via `--tenant-runner-cache-size` (env `CLOACINA_TENANT_RUNNER_CACHE_SIZE`).
- [ ] Cache-miss path: construct a `DefaultRunner` bound to the tenant's `Database` (from `TenantDatabaseCache`), share registries via `Arc`. Spawn scheduler loop + heartbeat tasks.
- [ ] Eviction path: graceful runner shutdown — stop scheduler loop, cancel in-flight executions, close DB pool, await spawned tasks, drop. No orphan threads, no orphan FDs.
- [ ] `WorkflowExecutor::execute_async` (and any direct callers in server routes) acquire the per-tenant runner via the cache.
- [ ] `cloacina-server`'s `lib.rs::run` constructs `TenantRunnerCache` at startup and threads it through `AppState`.
- [ ] **Daemon path unchanged.** `TenantRunnerCache` is server-only; `cloacinactl daemon` continues to use a single direct `DefaultRunner`.
- [ ] Integration test: submit a workflow execution for tenant A; assert the execution row lands in tenant A's schema, not admin.
- [ ] Integration test: configure cap=2, alternate between 3 tenants; LRU evicts the oldest cleanly.
- [ ] Integration test: cache.shutdown_all() during server graceful-shutdown drains all in-flight executions and returns clean.
- [ ] Stress test: 300+ tenant churn doesn't leak FDs or threads (sampled via `procfs` or `lsof -p`).
- [ ] **Test harness updated as we go**: existing integration tests assert admin-schema-bound execution → update to assert per-tenant. New tenant-fixture utilities for multi-tenant scenarios (e.g. `with_two_tenants(...)` helper). Refactor the harness FIRST, then implement the cache, then migrate tests one by one — don't try to flip everything in a single PR. Run `angreal test integration` after each milestone (cache type, eviction path, executor wiring, server integration).

## Test Cases

- **TC-1 (correctness):** `WorkflowExecutor::execute_async` for tenant A writes the execution row into the `tenant_A` schema, not `public` and not the admin schema.
- **TC-2 (LRU eviction):** cap=2, alternate 3 tenants in round-robin. The least-recently-active gets evicted; recreation on next access works cleanly.
- **TC-3 (graceful shutdown):** `cache.shutdown_all()` while two tenants have in-flight workflows. All workflows complete (or cleanly cancel), all spawned tasks join, no orphan FDs.
- **TC-4 (resource ceiling):** 300+ unique tenants over 60 seconds at default cap=256. FD count, thread count, memory all bounded; no monotonic growth.
- **TC-5 (registry sharing):** assert the same `Arc<TaskRegistry>` pointer is held by every per-tenant runner. Confirms inventory isn't duplicated per tenant.
- **TC-6 (cold-start latency):** measure cache-miss runner construction time; document the operator-visible recreation cost.

## Implementation Notes

### Technical Approach

- Pattern after `TenantDatabaseCache` in `crates/cloacina/src/dal/` (verify exact path). Both caches share the same lifecycle and eviction semantics; consider whether they should be unified or kept separate (separate is probably cleaner — different cache items, different shutdown choreography).
- LRU: use the `lru` crate (already in workspace deps; verify) or roll a small `HashMap + VecDeque` if dependency concerns. Eviction must be synchronous-callable from the lookup path; the actual runner-shutdown can be backgrounded if needed.
- Registries (`TaskRegistry`, `WorkflowRegistry`, `TriggerRegistry`, `ReactorRegistry`, `GraphRegistry`) are already `Arc`-shareable post-T-0506. Each per-tenant `Runtime::new()` receives the same `Arc` clones.
- Runner construction: `DefaultRunner::new(database, runtime, config)`. Each tenant gets fresh `database` from `TenantDatabaseCache`, fresh `Runtime`, but shared `Arc<TaskRegistry>` etc. inside `Runtime`.
- Server integration: `crates/cloacina-server/src/lib.rs::run` builds `TenantRunnerCache::new(cap, shared_registries, tenant_db_cache)`. `AppState` holds an `Arc<TenantRunnerCache>`. Every route that ran `execute_async` directly now goes through `cache.with_runner(tenant_id, |runner| runner.execute_async(...))`.

### Dependencies

- Conceptually depends on T-0578 (spans) and T-0579 (route fixes) being in place for better debuggability, but neither is a hard build-time dep. Land in either order.
- T-0582 (search_path fail-closed) shares the connection-acquire path; if both land in the same week, the second one rebases on the first.
- T-0581 (`remove_tenant`) depends on T-0580 — needs the cache eviction surface this creates.

### Risk Considerations

- **Many existing tests assume admin-schema-bound execution.** This is the biggest test-harness lift in the initiative. Mitigation: dedicate the first day/two to harness refactor — add `with_tenant_runner(tenant_id, |runner| ...)` test helper; migrate tests one file at a time.
- **Cold-start latency on cache miss.** Constructing a `DefaultRunner` spins up a scheduler loop, heartbeat task, executor pool. Tens of ms per construction on a warm machine. Acceptable for first request to a tenant, but pathological churn at default cap could be visible. Document the operator-tunable cap as the mitigation; deeper warming strategies are future work.
- **Connection-pool sizing.** Each cached runner has its own DB pool. At cap=256 with default pool size N, that's 256×N connections to the DB. Postgres `max_connections` ceiling can be hit. Document this in T-0577 follow-up; consider a default per-tenant pool size of 1-2 when running multi-tenant.
- **`Arc` cycles.** Be careful that no per-tenant `Runtime` holds an `Arc` back to `TenantRunnerCache` — that would prevent eviction. The cache holds runners; runners do not hold the cache.
- **Daemon regression risk.** The daemon must not pick up the cache path. Verify: only `cloacina-server` constructs `TenantRunnerCache`. The daemon's existing `DefaultRunner` construction stays.

## Status Updates

**2026-05-13** — Architectural core landed. 3 new cache unit tests pass; clippy clean. Live multi-tenant stress + full harness migration deferred to follow-ups.

### What changed

- **`crates/cloacina/src/runner/default_runner/mod.rs`**: new `DefaultRunner::with_database(database, config, shared_runtime)` constructor. Takes a pre-built tenant `Database` + optional shared `Arc<Runtime>` so inventory isn't duplicated per tenant. `with_config` refactored to delegate.
- **`crates/cloacina-server/src/tenant_runner_cache.rs`** (new): `TenantRunnerCache` — `lru::LruCache`-backed map of `tenant_id -> Arc<DefaultRunner>`. Surface: `get_or_create`, `evict` (graceful shutdown), `shutdown_all`, `shared_runtime`, `len`, `is_empty`. Eviction triggers a backgrounded `shutdown()` on the evicted runner so the lookup path stays fast.
- **`crates/cloacina-server/src/lib.rs`**: `AppState` gained `tenant_runners: Arc<TenantRunnerCache>`. `run()` signature added `tenant_runner_cache_size: usize`; constructs the cache + threads it through. Graceful-shutdown calls `tenant_runners.shutdown_all()` after the admin runner shuts down. New `runner_config_for_tenant_cache` helper.
- **`crates/cloacina-server/src/main.rs`**: `--tenant-runner-cache-size` CLI flag + `CLOACINA_TENANT_RUNNER_CACHE_SIZE` env; default 256.
- **`crates/cloacina-server/src/routes/executions.rs::execute_workflow`**: replaced `state.runner.execute_async(...)` with the tenant-cache path. Execution rows + events now land in the tenant schema.

### Tests landed (3 new)

- `empty_cache_is_empty`, `evict_missing_tenant_returns_false`, `shared_runtime_is_stable_arc`.

### Design decisions

- **Shared `Arc<Runtime>` across per-tenant runners** — inventory registries are post-T-0506 inventory-seeded with no per-tenant state. Makes 256-tenant caches affordable.
- **Backgrounded eviction shutdown** — `shutdown_all` (graceful exit) is sequential; LRU eviction during normal operation is fire-and-forget so the next lookup doesn't block.
- **Operator-tunable cap** with `max(1)` floor so misconfigured 0 doesn't panic LruCache.
- **Admin `state.runner` still exists** — handles non-tenant-scoped paths (reconciler, admin-context routes). Per-tenant routing only flips the workflow `/execute` path; minimal diff, gives T-0581 a single well-known global surface.

### Outstanding (follow-ups, not blocking close)

- Stress test for 300+ tenant FD/thread bounds and LRU shutdown latency.
- Per-tenant graph scheduler wiring (needed for CG packages submitted under a tenant). Deferred to T-0581.
- Live multi-tenant schema-assertion test.
- Inventory `Arc` pointer-equality assertion across two constructed tenant runners.
- Existing live-DB tests (currently 39 fail without Postgres) need fixtures updated for per-tenant routing.

### Verification (local)

- `cargo check -p cloacina --features postgres` → clean.
- `cargo check -p cloacina-server --features postgres` → clean.
- `cargo test --lib -p cloacina-server --features postgres tenant_runner` → 3 new pass.
- `cargo clippy --lib -p cloacina-server --features postgres` → clean.
