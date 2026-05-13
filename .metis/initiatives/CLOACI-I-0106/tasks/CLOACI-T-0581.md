---
id: t-04-remove-tenant-orchestrated
level: task
title: "T-04: remove_tenant orchestrated teardown"
short_code: "CLOACI-T-0581"
created_at: 2026-05-13T19:38:43.941745+00:00
updated_at: 2026-05-13T19:38:43.941745+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-04: remove_tenant orchestrated teardown

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Implement orchestrated teardown for `DELETE /v1/tenants/{name}` in five top-down steps: stop reactors → cancel executions → revoke keys → evict caches (DB + runner) → drop schema. Each step idempotent and observable; partial failures leave the system in a state a retry can resume. Closes SEC-14 and SEC-17.

## Acceptance Criteria

- [ ] `remove_tenant(tenant_id)` runs the five-step sequence in fixed order. Each step is idempotent.
- [ ] **Step 1 — reactors:** `scheduler.stop_all_reactors_for_tenant(tenant_id)`. Reactors stop accepting new firings before any other teardown begins.
- [ ] **Step 2 — executions:** `executor.cancel_running_for_tenant(tenant_id)` triggers cooperative cancellation (via T-0487's mechanism) on every running workflow in the tenant. Bounded wait (`--tenant-deletion-drain-timeout-s`, default 30s); past that, hard-cancel and proceed.
- [ ] **Step 3 — keys:** all `api_keys` with `tenant_id=X` set to `status='revoked'` in a single transaction.
- [ ] **Step 4 — caches:** `TenantRunnerCache.evict(tenant_id)` and `TenantDatabaseCache.evict(tenant_id)`. The runner cache's graceful-shutdown path (T-0580) handles in-flight cleanup.
- [ ] **Step 5 — schema:** `DROP SCHEMA tenant_X CASCADE` (Postgres); SQLite equivalent — drop the tenant's tables in dep order, or treat schema-per-tenant differently on SQLite (verify the existing convention during implementation).
- [ ] Idempotent retry: re-running `remove_tenant` on an already-deleted tenant returns `Ok(())` cleanly (not "not found" error).
- [ ] Audit emit per step: five new event kinds — `tenant.teardown.reactors_stopped`, `tenant.teardown.executions_cancelled`, `tenant.teardown.keys_revoked`, `tenant.teardown.caches_evicted`, `tenant.teardown.schema_dropped`. Each carries `tenant_id` and `step_duration_ms`.
- [ ] Integration test: full teardown of a tenant with active workflows, running reactors, and live API keys. Assert (a) no orphan reactors, (b) workflows transitioned to `Cancelled`, (c) all keys revoked, (d) caches no longer hold the tenant, (e) schema is gone.
- [ ] Integration test: partial-failure on step 3 (simulated DB blip), retry recovers cleanly without double-effecting earlier steps.
- [ ] **Test harness updated as we go**: existing tenant-deletion test (if any) updated for the new orchestration; new fixtures for "tenant with in-flight workflows" and "tenant with many reactors" so we exercise step 1+2 with realistic state. Run `angreal test integration` after each step is wired (not at the end).

## Test Cases

- **TC-1 (clean teardown):** empty tenant (no reactors, no executions). All five steps succeed, schema dropped, return Ok.
- **TC-2 (with in-flight work):** tenant has a running workflow + 2 active reactors + 5 API keys. Teardown drains in <30s; assertions on all five outcomes.
- **TC-3 (idempotent retry):** invoke `remove_tenant` twice. Second invocation returns Ok cleanly; no spurious events emitted.
- **TC-4 (drain timeout):** workflow ignores its cancellation token (simulated). Teardown waits `drain_timeout_s`, then hard-cancels and proceeds.
- **TC-5 (partial failure recovery):** simulated DB error during step 3. Audit log shows step 1 and 2 succeeded; step 3 failed. Retry from step 3 onward; final state is clean.

## Implementation Notes

### Technical Approach

- Sits in `crates/cloacina/src/security/tenant_manager.rs` (verify path) or the existing remove-tenant route handler in `crates/cloacina-server/src/routes/tenants.rs`.
- Cancellation: cooperative cancellation already lands per T-0487. Step 2 just iterates running workflows in the tenant schema and triggers their cancellation tokens. Track which workflows we cancelled so the drain wait knows what to wait on.
- Drain wait: simple `tokio::time::timeout(drain_timeout, futures::future::join_all(cancel_handles))`. On timeout, log a warning naming the workflows that didn't drain and proceed; their schema rows get dropped in step 5 anyway.
- Audit events: extend `crates/cloacina/src/security/audit.rs` with 5 new `pub fn log_tenant_teardown_*` + const event kinds. Same `with_captured_logs` test pattern as T-0576.
- Schema drop on SQLite: confirm convention. If SQLite uses table-name prefixes rather than schemas, this becomes a per-table drop loop in dep order.

### Dependencies

- **T-0580** is a hard dep — uses `TenantRunnerCache::evict` in step 4 and `cancel_running_for_tenant` (implemented on the per-tenant runner). Must land first.
- **T-0578** (spans) helpful for debugging teardown failures but not a build-time dep.

### Risk Considerations

- **Step 2 hang:** a task that ignores its cancellation token blocks the drain. Mitigation: the drain has a configurable hard timeout. Past that, we move on — the workflow row will get its schema dropped from underneath it in step 5, causing it to error on its next DB write. Document this in T-0577 follow-up.
- **Race with new requests:** between step 1 (stop reactors) and step 3 (revoke keys), a request with a still-valid key could try to use the tenant. The route handlers must check that the tenant still exists (or the cache miss-path will fail fast on a dropped schema). Plan: the auth middleware already lookups tenant in `TenantDatabaseCache`; if the cache returns "evicted," return 410 Gone to the caller.
- **Idempotent isn't trivial:** step 5 (DROP SCHEMA) on an already-dropped schema errors on Postgres without `IF EXISTS`. Use the `IF EXISTS` clause everywhere; treat "not found" at any step as success.
- **SQLite test environment:** if SQLite uses a different tenant model (one DB file vs per-tenant schemas), step 5 needs a different code path. Confirm before scoping.

## Status Updates

*To be added during implementation.*
