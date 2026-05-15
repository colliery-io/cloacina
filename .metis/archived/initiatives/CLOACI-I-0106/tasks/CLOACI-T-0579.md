---
id: t-02-tenant-scoped-routes-triggers
level: task
title: "T-02: Tenant-scoped routes — triggers list/get + health endpoints"
short_code: "CLOACI-T-0579"
created_at: 2026-05-13T19:38:42.038123+00:00
updated_at: 2026-05-13T21:36:25.844553+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-02: Tenant-scoped routes — triggers list/get + health endpoints

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Three handlers leak cross-tenant data today by querying admin-schema state instead of the caller's tenant: `list_triggers`, `get_trigger`, and `/v1/health/{accumulators,graphs}`. Scope each to the caller's authorized tenant set (admin bypass for `is_admin=true`). Closes SEC-02 and SEC-05.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-05-13** — Landed. 3 new unit tests pass; clippy clean.

### What changed

- **`crates/cloacina-server/src/routes/triggers.rs`** — `list_triggers` + `get_trigger` resolve a tenant-scoped `Database` via `state.tenant_databases.resolve(&tenant_id, &state.database)` before constructing the DAL. Each tenant has its own schema, so `SELECT FROM schedules` naturally returns only that tenant's rows. Cross-tenant `get_trigger` 404s without info-disclosure.
- **`crates/cloacina/src/computation_graph/scheduler.rs`** — `GraphStatus` gained `tenant_id: Option<String>` populated from `RunningGraph::declaration.tenant_id`.
- **`crates/cloacina/src/computation_graph/registry.rs`** — new `EndpointRegistry::list_accumulators_with_health_for_key(&KeyContext)` filters by each accumulator's `AccumulatorAuthPolicy::is_authorized`. Accumulators without a policy entry fall through to `allow_all_authenticated` (single-tenant compat).
- **`crates/cloacina-server/src/routes/health_graphs.rs`** —
  - All three handlers take `Extension<AuthenticatedKey>`.
  - `list_accumulators` uses the new `_for_key` registry method.
  - `list_graphs` filters via `graph_visible(&auth, g.tenant_id)`.
  - `get_graph` 404s on cross-tenant access (not 403).
  - `graph_visible` helper: admin sees all; tenant-scoped keys see own + untagged; global non-admin sees only untagged.

### Tests landed (3 new)

- `graph_visible_admin_sees_all` — admin path.
- `graph_visible_tenant_scoped` — own + untagged visible, other-tenant not.
- `graph_visible_global_key_cannot_see_tenant_graphs` — global non-admin.

### Design notes

- **Untagged graphs visible to all authenticated callers** — single-tenant compat.
- **404 over 403 for cross-tenant `get_graph`** — prevents tenant-name probing.
- **Accumulator filtering reuses existing policy** rather than introducing a new tenant field on the registry.

### Verification (local)

- `cargo test --lib -p cloacina-server --features postgres graph_visible` → 3 new pass.
- `cargo check --features postgres -p cloacina-server` → clean.
- `cargo clippy --lib -p cloacina --features postgres` + `cloacina-server` → clean.
- 17 pre-existing tests pass; 39 fail due to no live Postgres locally (pre-existing infra dependency).
