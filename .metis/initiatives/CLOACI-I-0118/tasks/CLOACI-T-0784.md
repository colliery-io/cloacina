---
id: key-management-leak-fix-tenant
level: task
title: "Key-management leak fix + tenant-admin key endpoints + api_keys DAL"
short_code: "CLOACI-T-0784"
created_at: 2026-06-24T00:41:38.589576+00:00
updated_at: 2026-06-24T00:41:38.589576+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Key-management leak fix + tenant-admin key endpoints + api_keys DAL

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Close the cross-tenant key-management leak and ship tenant-admin self-service over keys. Move `/auth/keys` create/list/revoke to `Platform` (god-only). Add `GET /tenants/{t}/keys` and `DELETE /tenants/{t}/keys/{key_id}` (`TenantParam + Admin`), and lower `POST /tenants/{t}/keys` from god-only to tenant-admin. Add DAL `list_keys_for_tenant(tenant)` and `get_key(id)`; tenant-scoped revoke verifies the target key belongs to the caller's tenant.

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

*To be added during implementation*
