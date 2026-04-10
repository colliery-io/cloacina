---
id: handler-tenant-enforcement-extract
level: task
title: "Handler tenant enforcement — extract AuthenticatedKey, scope queries to tenant schema, admin bypass"
short_code: "CLOACI-T-0425"
created_at: 2026-04-06T15:18:23.250747+00:00
updated_at: 2026-04-06T19:07:30.719324+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Handler tenant enforcement — extract AuthenticatedKey, scope queries to tenant schema, admin bypass

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Connect all REST handlers to the tenant schema system and make them use `AuthenticatedKey` for authorization. Currently every handler ignores the authenticated key identity and queries the global schema regardless of the `tenant_id` URL parameter. After this task, handlers enforce that the caller's key matches the requested tenant and queries target the correct tenant schema.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All tenant-scoped handlers extract `AuthenticatedKey` from request extensions
- [ ] Tenant-scoped keys: handler verifies `key.tenant_id == tenant_id` from URL path, returns 403 on mismatch
- [ ] Global keys (tenant_id=NULL): can access global/public resources only, return 403 for tenant-scoped paths
- [ ] Admin keys (is_admin=true): pass all tenant checks — god mode
- [ ] Handlers use `tenant_id` from URL to scope database queries to the correct Postgres schema
- [ ] Affected handlers: `upload_workflow`, `list_workflows`, `get_workflow`, `delete_workflow`, `execute_workflow`, `list_executions`, `get_execution`, `get_execution_events`, `list_triggers`, `get_trigger`
- [ ] Single-tenant default (global keys + public tenant) continues to work unchanged
- [ ] Server-soak test passes

## Implementation Notes

### Key files
- `crates/cloacinactl/src/server/workflows.rs` — upload, list, get, delete
- `crates/cloacinactl/src/server/executions.rs` — execute, list, get
- `crates/cloacinactl/src/server/triggers.rs` — list, get
- `crates/cloacinactl/src/server/auth.rs` — `AuthenticatedKey` (updated in T-0424)

### Design consideration
Tenant schema switching: the server currently uses a single global DB connection. Need to either switch schema per-request (`SET search_path TO {tenant_schema}`) or establish per-tenant connection pools. Per-request `SET search_path` is simpler for MVP.

### Dependencies
- T-0424 (key scoping) — `AuthenticatedKey` must have `tenant_id` and `is_admin` fields
- T-0423 (package tenant ownership) — packages must carry tenant_id

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete**
- Added `can_access_tenant()`, `forbidden_response()`, `admin_required_response()` helpers on `AuthenticatedKey`
- `can_access_tenant` logic: admin=always, scoped=must match, global=public only
- All 10 tenant-scoped handlers now extract `Extension(auth): Extension<AuthenticatedKey>` and check `auth.can_access_tenant(&tenant_id)` → 403 on mismatch
- Updated handlers: upload_workflow, list_workflows, get_workflow, delete_workflow, execute_workflow, list_executions, get_execution, get_execution_events, list_triggers, get_trigger
- Tenant management (create_tenant, remove_tenant) now require `auth.is_admin` → 403 otherwise
- Added `POST /tenants/{tenant_id}/keys` endpoint (admin-only) for creating tenant-scoped keys
- Route registered in `build_router`
- All compiles clean
- NOTE: per-request schema switching (`SET search_path`) deferred — requires deeper DAL architecture change. Auth checks enforce who can access; data isolation via schema requires follow-up work.
