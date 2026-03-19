---
id: tenantdal-and-apikeydal-with-crud
level: task
title: "TenantDAL and ApiKeyDAL with CRUD operations"
short_code: "CLOACI-T-0186"
created_at: 2026-03-16T20:01:00.746328+00:00
updated_at: 2026-03-16T20:24:58.151231+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# TenantDAL and ApiKeyDAL with CRUD operations

## Objective

Implement TenantDAL and ApiKeyDAL providing CRUD operations for tenants and API keys. These are the data access functions used by the auth middleware, CLI, and integration tests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TenantDAL::create(name, schema_name) -> TenantRow` inserts and returns new tenant
- [ ] `TenantDAL::list() -> Vec<TenantRow>` returns all tenants
- [ ] `TenantDAL::get(id) -> Option<TenantRow>` by UUID
- [ ] `TenantDAL::get_by_name(name) -> Option<TenantRow>` by unique name
- [ ] `TenantDAL::deactivate(id)` sets status to 'inactive'
- [ ] `ApiKeyDAL::create(NewApiKey) -> Uuid` inserts key and returns id
- [ ] `ApiKeyDAL::load_by_prefix(prefix) -> Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>` with LEFT JOIN on workflow_patterns in one query
- [ ] `ApiKeyDAL::list_by_tenant(tenant_id) -> Vec<ApiKeyRow>` for listing (no patterns needed)
- [ ] `ApiKeyDAL::revoke(key_id)` sets revoked_at to now
- [ ] `ApiKeyDAL::delete(key_id)` hard-deletes key (cascade removes patterns)
- [ ] All operations work on both Postgres and SQLite backends

## Implementation Notes

### Pattern
- Follow the `dispatch_backend!` macro pattern from `accumulator_state.rs` for multi-backend dispatch
- Place in `dal/unified/` — either new files (e.g., `tenant.rs`, `api_key.rs`) or extend existing modules

### load_by_prefix (Hot Path)
- This is called on every authenticated request (after cache miss). Must return the key row plus all associated workflow patterns in a single query.
- Use LEFT JOIN on `api_key_workflow_patterns` keyed by `api_key_id`, grouped in Rust after query
- Filter by `key_prefix` column (indexed)

### Dependencies
- Requires CLOACI-T-0184 (migrations) and CLOACI-T-0185 (schema + models)

## Status Updates

### 2026-03-16 — Completed
- Created tenant_dal.rs: create, list (active only), get, get_by_name, deactivate
- Created api_key_dal.rs: create, create_patterns, load_by_prefix (with pattern loading), list_by_tenant, revoke
- Both use dispatch_backend! pattern with postgres/sqlite split implementations
- Added module declarations and re-exports in mod.rs
- Compiles clean
