---
id: t-02-tenant-scoped-routes-triggers
level: task
title: "T-02: Tenant-scoped routes — triggers list/get + health endpoints"
short_code: "CLOACI-T-0579"
created_at: 2026-05-13T19:38:42.038123+00:00
updated_at: 2026-05-13T19:38:42.038123+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-02: Tenant-scoped routes — triggers list/get + health endpoints

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Three handlers leak cross-tenant data today by querying admin-schema state instead of the caller's tenant: `list_triggers`, `get_trigger`, and `/v1/health/{accumulators,graphs}`. Scope each to the caller's authorized tenant set (admin bypass for `is_admin=true`). Closes SEC-02 and SEC-05.

## Acceptance Criteria

- [ ] `list_triggers` queries the caller's tenant schema only (admin → all tenants).
- [ ] `get_trigger` 404s when the requested trigger belongs to a different tenant (no info-disclosure via "not in your tenant" vs "doesn't exist" distinction).
- [ ] `/v1/health/accumulators` filters by caller's tenant; admin sees all.
- [ ] `/v1/health/graphs` filters by caller's tenant; admin sees all.
- [ ] Regression test: trigger created by tenant A is invisible to tenant B via `list_triggers`.
- [ ] Regression test: tenant B's `get_trigger` with A's trigger id returns 404, not 200 with A's payload.
- [ ] Regression test: `/v1/health/graphs` for tenant A returns only A's graphs.
- [ ] Regression test: admin key sees all tenants' triggers and graphs.
- [ ] **Test harness updated as we go**: existing tests that asserted admin-schema-bound list/get/health behavior reframed for the per-tenant model. Run `angreal test integration` (and the relevant `angreal test auth` if it covers these paths) after each handler is fixed — don't batch to the end of the task.

## Test Cases

- **TC-1 (list isolation):** create trigger in tenant A; list as tenant B returns `[]`; list as tenant A returns `[trigger]`.
- **TC-2 (get cross-tenant 404):** create trigger in tenant A; tenant B's `GET /v1/triggers/<A's id>` → 404.
- **TC-3 (admin bypass):** admin key on either route sees both tenants' rows.
- **TC-4 (health graphs):** load CG packages in two tenants; `/v1/health/graphs` per tenant returns only its own.
- **TC-5 (health accumulators):** same shape as TC-4 but on the accumulator endpoint.

## Implementation Notes

### Technical Approach

- Routes live in `crates/cloacina-server/src/routes/triggers.rs` and `crates/cloacina-server/src/routes/health.rs` (verify paths during implementation).
- Pattern: extract `AuthenticatedKey` (already in scope via the auth middleware extractor), branch on `key.is_admin`, scope the DAL call via `TenantDatabaseCache::with_tenant_database(tenant_id, ...)`.
- For `get_trigger`, the existing 404 path on "row not found in current schema" naturally collapses with cross-tenant access (the query returns no rows when scoped to the wrong tenant). No special handling needed; just don't fall back to admin schema.
- Health endpoints currently iterate `ReactiveScheduler::list_graphs()` etc.; extend to filter by tenant (likely a new method on the scheduler that takes a tenant filter, with admin → no filter).

### Dependencies

- None blocking. Can land before T-0580 — these routes use the existing tenant-scoped DAL surface, not a per-tenant runner.
- T-0578's enriched spans help debugging during this task but aren't a hard dep.

### Risk Considerations

- **Orphan triggers** (tenant deleted but trigger row survives — pre-T-0581 behavior) will be invisible to `list_triggers` after this fix. That's correct; T-0581 will sweep them as part of `remove_tenant` teardown.
- **Health endpoint perf:** scoping by tenant filter on the scheduler's in-memory state is O(graphs) per call; cheap. No perf concern.
- **Existing tests likely failing:** any test that asserts `list_triggers` returns a global list will break. Triage as we go — most are likely admin-context tests that just need updating to assert the tenant-scoped result.

## Status Updates

*To be added during implementation.*
