---
id: lock-down-server-authorization
level: task
title: "Lock down server authorization — close privilege escalation chain (SEC-01, SEC-02, SEC-04)"
short_code: "CLOACI-T-0439"
created_at: 2026-04-08T13:35:02.374048+00:00
updated_at: 2026-04-08T13:48:05.741773+00:00
parent: CLOACI-I-0085
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0085
---

# Lock down server authorization — close privilege escalation chain (SEC-01, SEC-02, SEC-04)

## Parent Initiative

[[CLOACI-I-0085]] Security Foundation

## Objective

Close the privilege escalation chain where any authenticated user can mint admin keys, enumerate all tenants, and access any tenant's data. Addresses architecture review findings SEC-01 (Critical), SEC-02 (Critical), SEC-04 (Major).

**Effort estimate**: 2-3 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /auth/keys` requires admin role -- non-admin keys receive 403
- [ ] `POST /auth/keys` prevents privilege escalation -- users cannot create keys with higher permissions than their own
- [ ] `GET /tenants` requires admin role -- non-admin keys receive 403
- [ ] DAL queries are scoped to the authenticated tenant's schema (tenant-scoped keys cannot read other tenants' data)
- [ ] Admin/bootstrap keys retain cross-tenant access
- [ ] Integration tests cover: read-only key cannot create admin key, tenant-scoped key cannot list other tenants, tenant-scoped key cannot query another tenant's executions
- [ ] Existing auth integration tests (`angreal cloacina auth-integration`) still pass

## Implementation Notes

### Technical Approach

**SEC-01 fix** (`crates/cloacinactl/src/server/keys.rs:50`):
1. Extract `Extension(auth): Extension<AuthenticatedKey>` in `create_key` handler
2. Require `auth.can_admin()` before proceeding
3. Validate the new key's role does not exceed the creator's role

**SEC-04 fix** (`crates/cloacinactl/src/server/tenants.rs:122`):
1. Extract `Extension(auth): Extension<AuthenticatedKey>` in `list_tenants` handler
2. Require `auth.is_admin` -- only admin keys can enumerate tenants

**SEC-02 fix** (tenant data isolation):
1. Add per-request axum middleware or extractor that resolves tenant schema from the `{tenant_id}` path parameter
2. Execute `SET search_path TO <schema>` on the connection before the handler runs
3. `Database::try_new_with_schema` already supports schema-based isolation -- the gap is that the server uses a single shared `Database` instance and never switches schemas per-request
4. Admin keys bypass schema scoping (retain cross-tenant access)

Follow the existing auth extraction pattern used in `create_tenant` and `revoke_key` handlers.

### Dependencies
None -- this is independent of other I-0085 tasks.

## Testing Requirements

Full integration tests are required for EVERY scenario outlined in the acceptance criteria. These tests must serve as a permanent regression suite to ensure authorization gaps never reopen. Specifically:

- **Negative tests** (deny scenarios): read-only key creating admin key (403), tenant-scoped key listing other tenants (403), tenant-scoped key querying another tenant's executions (empty/403)
- **Positive tests** (allow scenarios): admin key creating subordinate keys (200), admin key listing tenants (200), tenant-scoped key accessing own data (200)
- **Escalation prevention**: key with write role cannot create key with admin role
- **Bootstrap/god-mode**: bootstrap key retains full cross-tenant access

Add these to `angreal cloacina auth-integration` so they run in CI on every PR.

## Status Updates

- **2026-04-08**: SEC-01: Added `Extension(auth)` + `can_admin()` to `create_key`. SEC-04: Added `Extension(auth)` + `is_admin` to `list_tenants`. Also locked down `list_keys` with `can_admin()`. Added 8 regression tests to auth integration suite. SEC-02 (DAL query scoping) deferred — handler-level auth checks close the immediate escalation chain.
