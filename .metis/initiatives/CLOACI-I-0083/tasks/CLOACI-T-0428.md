---
id: auth-integration-tests-single
level: task
title: "Auth integration tests — single-tenant, multi-tenant, CG WebSocket, god mode, deny scenarios"
short_code: "CLOACI-T-0428"
created_at: 2026-04-06T15:18:39.763429+00:00
updated_at: 2026-04-06T19:20:44.054780+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Auth integration tests — single-tenant, multi-tenant, CG WebSocket, god mode, deny scenarios

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Comprehensive integration tests validating the full auth model across both deployment modes. Covers single-tenant flow (everything works with bootstrap key), multi-tenant isolation (cross-tenant denied), CG WebSocket authorization, god mode bypass, and role enforcement. Updates the existing server-soak and WS integration tests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Single-tenant tests**: bootstrap key uploads package, executes workflow, connects to CG WebSocket — all succeed
- [ ] **Multi-tenant isolation**: create two tenants with scoped keys; tenant A's key cannot access tenant B's workflows, executions, or WS endpoints
- [ ] **CG WebSocket**: tenant A's key → tenant A's accumulator = 101; tenant A's key → tenant B's accumulator = 403
- [ ] **God mode**: admin key (is_admin=true) can access any tenant's resources, connect to any WS endpoint
- [ ] **Global key**: key with NULL tenant_id can access global/public resources only, denied for tenant-scoped paths
- [ ] **Role enforcement**: read-only key cannot execute or upload; write key cannot manage keys; admin key can do everything
- [ ] **Deny scenarios**: revoked key → 401; no auth → 401; wrong tenant → 403; wrong role → 403
- [ ] **Server-soak test** updated to exercise auth boundaries (create tenant, scoped key, upload, execute, verify isolation)
- [ ] **WS integration test** updated to verify 101 on valid auth after CG policy wiring
- [ ] All existing tests pass

## Implementation Notes

### Test approach
Extend the existing angreal test infrastructure:
- `angreal cloacina ws-integration` — add cases for valid auth → 101, tenant isolation
- `angreal cloacina server-soak` — add multi-tenant step: create second tenant, verify isolation during soak loop

### Dependencies
- All other tasks in this initiative (T-0422 through T-0427) — this is the validation task

## Status Updates **[REQUIRED]**

**2026-04-06 — Revised: proper integration tests**
- Reverted server-soak changes (soak tests ≠ integration tests)
- Created `angreal cloacina auth-integration` — dedicated auth integration test command
- Test boots full server with Postgres, exercises 6 test groups:
  1. Deny scenarios: no auth → 401, invalid token → 401
  2. Bootstrap (god mode): lists keys, accesses public, key response has is_admin/tenant_id
  3. Global key: accesses public, cannot create tenants (403)
  4. Role enforcement: read key lists but can't execute (403), write key executes but can't revoke (403)
  5. Tenant isolation: tenant-scoped key → own tenant OK, other tenant → 403
  6. Revoked key: works before, 401 after revocation
- Registered in `__init__.py`, shows in `angreal tree`
- CG WebSocket tenant isolation tests deferred until CG packages are loadable via server (T-0404 soak test)
- Unit test coverage: registry `test_accumulator_auth_tenant_scoped` (T-0426)
