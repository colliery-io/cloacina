---
id: tenant-scoped-cg-policies-upgrade
level: task
title: "Tenant-scoped CG policies — upgrade auth policies from allow-all to tenant-scoped on package load"
short_code: "CLOACI-T-0426"
created_at: 2026-04-06T15:18:27.829543+00:00
updated_at: 2026-04-06T19:16:08.384538+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Tenant-scoped CG policies — upgrade auth policies from allow-all to tenant-scoped on package load

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Upgrade T-0422's "allow any authenticated key" CG policies to tenant-scoped policies. When a CG package belongs to a tenant, only that tenant's keys (plus admin keys) should be able to access its WebSocket endpoints. Global packages (NULL tenant_id) keep the allow-all behavior.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] When `load_graph()` loads a package with a `tenant_id`, accumulator/reactor policies restrict access to keys belonging to that tenant
- [ ] Admin keys (is_admin=true) are always included in all policies
- [ ] Global packages (tenant_id=NULL) retain allow-all-authenticated behavior from T-0422
- [ ] Tenant A's key connecting to tenant B's accumulator → 403
- [ ] Tenant A's key connecting to tenant A's accumulator → 101
- [ ] Admin key connecting to any accumulator → 101
- [ ] Policy lookup needs access to the API keys DAL to resolve which keys belong to a tenant (or policies use tenant_id matching instead of UUID lists)

## Implementation Notes

### Design consideration
The current `AccumulatorAuthPolicy` uses `allowed_producers: Vec<uuid::Uuid>` — explicit key UUID lists. For tenant-scoped policies, two approaches:
1. **UUID list**: query DAL for all keys with matching tenant_id, populate the list. Downside: must refresh when keys are created/revoked.
2. **Tenant-based check**: add `allowed_tenants: Vec<String>` to the policy. The WS handler already has the `AuthenticatedKey` with `tenant_id` — check if the key's tenant matches. Simpler, no refresh needed.

Option 2 is cleaner. The policy struct evolves from UUID-only to supporting tenant-based access.

### Dependencies
- T-0422 (CG policy wiring) — the `allow_all_authenticated` flag
- T-0423 (package tenant ownership) — packages carry tenant_id
- T-0424 (key scoping) — keys carry tenant_id

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete**
- Added `KeyContext` struct with `key_id`, `tenant_id`, `is_admin` — passed through all auth check chains
- Added `allowed_tenants: Vec<String>` to both `AccumulatorAuthPolicy` and `ReactorAuthPolicy`
- Added `for_tenant(tid)` constructors on both policy types
- Updated `is_authorized()` and `is_operation_permitted()` to accept `KeyContext` — checks: allow_all → admin → explicit UUID → tenant match → deny
- Updated `check_accumulator_auth`, `check_reactor_auth`, `check_reactor_op_auth` on EndpointRegistry to accept `&KeyContext`
- Updated WS handlers to construct `KeyContext` from `AuthenticatedKey` and pass through
- Updated `process_reactor_command` to accept and forward tenant/admin context
- Added `tenant_id: Option<String>` to `ComputationGraphDeclaration`
- `load_graph()` now sets `for_tenant(tid)` policies when tenant_id is Some, `allow_all()` when None
- Both restart paths (full reactor, individual accumulator) use declaration's tenant_id for policy
- `build_declaration_from_ffi` sets `tenant_id: None` (reconciler will fill from package metadata)
- New test `test_accumulator_auth_tenant_scoped`: acme key→acme=pass, other→deny, admin→pass, global→deny
- All 11 registry tests pass, all crates compile clean
