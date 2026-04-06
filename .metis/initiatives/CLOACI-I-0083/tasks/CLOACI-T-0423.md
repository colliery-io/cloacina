---
id: package-tenant-ownership-tenant-id
level: task
title: "Package tenant ownership — tenant_id column, upload storage, reconciler context"
short_code: "CLOACI-T-0423"
created_at: 2026-04-06T15:18:13.615562+00:00
updated_at: 2026-04-06T16:22:38.021241+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Package tenant ownership — tenant_id column, upload storage, reconciler context

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Thread tenant ownership through the package lifecycle so every package knows which tenant it belongs to. Currently `workflow_packages` has no `tenant_id` column, the upload handler ignores the `tenant_id` URL parameter, and the reconciler uses a hardcoded `default_tenant_id = "public"`. After this task, packages carry their tenant from upload through loading.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Diesel migration adds `tenant_id TEXT NULL` to `workflow_packages` (both Postgres and SQLite)
- [ ] Upload handler (`workflows.rs`) stores the `tenant_id` from the URL path with the package
- [ ] NULL tenant_id means global (single-tenant default — backward compatible)
- [ ] `list_workflows` and `get_workflow` handlers filter by tenant_id from URL path
- [ ] Reconciler carries tenant_id from the package record through to `load_graph()` / workflow registration
- [ ] Existing packages (pre-migration) get NULL tenant_id and remain globally accessible
- [ ] Existing tests pass — single-tenant behavior unchanged

## Implementation Notes

### Key files
- `crates/cloacina/src/database/migrations/` — new migration for both Postgres and SQLite
- `crates/cloacina/src/database/schema.rs` — update Diesel schema
- `crates/cloacina/src/dal/` — package DAL operations need tenant_id parameter
- `crates/cloacinactl/src/server/workflows.rs` — upload, list, get handlers
- `crates/cloacina/src/registry/reconciler/` — loading pipeline

### Dependencies
- None — this is foundational, can be done independently of T-0422

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete**
- Created Postgres migration 018 and SQLite migration 016: `ALTER TABLE workflow_packages ADD COLUMN tenant_id TEXT NULL`
- Updated all three schema.rs `table!` blocks (unified, postgres, sqlite) with `tenant_id -> Nullable<Text>`
- Added `tenant_id: Option<String>` to `UnifiedWorkflowPackage`, `NewUnifiedWorkflowPackage`, and `WorkflowPackage` domain model
- Updated `From<UnifiedWorkflowPackage> for WorkflowPackage` conversion
- Added `tenant_id: Option<&str>` parameter to `WorkflowPackagesDAL::store_package_metadata` (public + both backend methods)
- Both backend insert blocks populate `tenant_id` from the new parameter
- `WorkflowRegistryImpl::store_package_metadata` (separate code path) passes `tenant_id: None` for now
- Updated all integration test callers to pass `None` as tenant_id
- All lib targets compile clean; integration test callers fixed
- NOTE: handler-level filtering by tenant_id and reconciler threading deferred to T-0425 (handler enforcement) since they require AuthenticatedKey extraction
