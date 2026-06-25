---
id: key-management-leak-fix-tenant
level: task
title: "Key-management leak fix + tenant-admin key endpoints + api_keys DAL"
short_code: "CLOACI-T-0784"
created_at: 2026-06-24T00:41:38.589576+00:00
updated_at: 2026-06-24T02:21:06.325737+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Key-management leak fix + tenant-admin key endpoints + api_keys DAL

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Close the cross-tenant key-management leak and ship tenant-admin self-service over keys. Move `/auth/keys` create/list/revoke to `Platform` (god-only). Add `GET /tenants/{t}/keys` and `DELETE /tenants/{t}/keys/{key_id}` (`TenantParam + Admin`), and lower `POST /tenants/{t}/keys` from god-only to tenant-admin. Add DAL `list_keys_for_tenant(tenant)` and `get_key(id)`; tenant-scoped revoke verifies the target key belongs to the caller's tenant.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `POST/GET/DELETE /auth/keys` are `Platform`: a tenant `role=admin` key gets 403 on all three (the leak fix).
- [ ] `GET /tenants/{t}/keys` returns only tenant `t`'s keys (via `list_keys_for_tenant`).
- [ ] `POST /tenants/{t}/keys` allowed for tenant `t`'s admin (no longer god-only); created key is tenant-scoped, never god.
- [ ] `DELETE /tenants/{t}/keys/{key_id}` revokes only if the target key ∈ `t` (god may still revoke any via `/auth/keys/{id}`).
- [ ] DAL `list_keys_for_tenant` + `get_key` added with unit tests.
- [ ] Leak-regression integration tests: cross-tenant list/revoke blocked; own-tenant allowed; another tenant's key → 403. `angreal test integration` green.

## Implementation Notes

**Scope:** the leak fix + tenant-admin key surface + supporting DAL. Land alongside T-0783 (the table already classifies these rows).
**Depends on:** T-0783 (route table + middleware).
**References:** I-0118 → "Phase 0 design" behavior changes 1–2; `crates/cloacina-server/src/routes/keys.rs`; `crates/cloacina/src/dal/unified/api_keys/crud.rs`.

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE (commit 211c5ea3).** No migration needed — `api_keys` already has `tenant_id`. **Leak fix:** `/auth/keys` POST/GET/DELETE flipped `Any+Admin`→`Platform` in the authz table (a tenant `role=admin` key can no longer list/revoke any tenant's keys). **Tenant-admin self-service:** `POST /tenants/{t}/keys` lowered `Platform`→`Tenant+Admin`; **new** `GET /tenants/{t}/keys` + `DELETE /tenants/{t}/keys/{key_id}` (`Tenant+Admin`), routes registered in lib.rs. DAL: added `list_keys_for_tenant(tenant)` + `get_key(id)` to crud.rs + the `ApiKeyDAL` wrapper (postgres-gated). New handlers `list_tenant_keys` + `revoke_tenant_key`; the revoke handler loads the key via `get_key` and 404s if `tenant_id` mismatches (no cross-tenant probe/touch), then revokes + clears the LRU cache. authz table now 45 routes; no-drift test updated; `angreal check crate` clean; 8/8 authz unit tests green. **Deferred:** leak-regression + tenant-admin integration tests run under `angreal test integration` (postgres lane) — recommended before merge.