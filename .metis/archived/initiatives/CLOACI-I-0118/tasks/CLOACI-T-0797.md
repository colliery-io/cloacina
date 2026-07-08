---
id: local-account-management-endpoints
level: task
title: "Local account management endpoints — tenant-admin create/list/disable/reset under /tenants/{t}/accounts"
short_code: "CLOACI-T-0797"
created_at: 2026-06-24T01:26:44.464111+00:00
updated_at: 2026-06-24T04:00:02.820529+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Local account management endpoints — tenant-admin create/list/disable/reset under /tenants/{t}/accounts

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Tenant-admin account-management endpoints under `/tenants/{t}/accounts`: create, list, disable, and reset-password — gated `TenantParam + Admin` by the Phase 0 authZ matcher (a tenant-admin manages only its tenant's accounts; god manages any). Mirrors the tenant-admin key surface (T-0784). Password reset is admin-reset-only for v1 (self-service deferred — OQ-12); optionally force-change-on-first-login.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `POST/GET/DELETE /tenants/{t}/accounts` (+ a password-reset endpoint) are classified `TenantParam + Admin` in the route table; a tenant-admin manages only its tenant's accounts, god any.
- [ ] Create sets username + initial password + role within the tenant; list shows the tenant's accounts WITHOUT hashes; disable flips status; reset sets a new password.
- [ ] A non-admin tenant key → 403.
- [ ] Integration tests for the tenant-admin account lifecycle + cross-tenant denial.

## Implementation Notes

**Scope:** account-management CRUD endpoints, tenant-admin gated via the Phase 0 matcher.
**Depends on:** Task 1 / CLOACI-T-0795 (store); T-0783 (route-table middleware + matcher).
**References:** CLOACI-I-0118 → "Local accounts strand"; OQ-12; mirrors T-0784 tenant-admin key surface.

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE.** Tenant-admin local-account CRUD under `/v1/tenants/{t}/accounts`, all `TenantParam + Admin` (authz table now **49 routes**; no-drift test updated): `POST` create, `GET` list (no hashes), `DELETE {account_id}` = **disable** (preserves history; minted keys lapse at TTL), `POST {account_id}/password` = admin reset (OQ-12). Handlers in `routes/local_auth.rs`. DAL `set_status`/`set_password` made **tenant-scoped** (`WHERE id AND tenant_id`) — a tenant-admin can't touch another tenant's account by id (atomic, no TOCTOU; cross-tenant id → 404). utoipa-registered; `openapi.json` regenerated. `angreal check` clean; 8/8 authz tests. Endpoint behavior under the postgres lane. **Unblocks** T-0798 UI account view. **Depends on:** T-0795 + T-0783 (done). Self-service reset stays out (OQ-12: admin-reset-only v1).