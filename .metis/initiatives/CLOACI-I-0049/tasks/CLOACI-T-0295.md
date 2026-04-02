---
id: tenant-management-create-list
level: task
title: "Tenant management — create/list/delete tenants via Postgres schema isolation"
short_code: "CLOACI-T-0295"
created_at: 2026-03-29T14:03:28.304370+00:00
updated_at: 2026-03-29T14:03:28.304370+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# Tenant management — create/list/delete tenants via Postgres schema isolation

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

Expose existing tenant DAL operations (`database/admin.rs` — `create_tenant`, `remove_tenant`) as REST endpoints. Add ABAC scoping to the auth middleware so API keys are restricted to specific tenants. Add tenant-scoping middleware that sets `search_path` for all tenant-scoped routes.

**Note**: The DAL for Postgres schema-per-tenant isolation already exists in `database/admin.rs`. This task is the HTTP layer + ABAC, not building the schema isolation from scratch.

## Acceptance Criteria

- [ ] `POST /tenants` — calls existing `create_tenant` DAL, returns tenant_id + metadata
- [ ] `GET /tenants` — lists all tenants with metadata
- [ ] `DELETE /tenants/:tenant_id` — calls existing `remove_tenant` DAL (with confirmation safeguard)
- [ ] Schema naming: `tenant_<slug>` (sanitized, no SQL injection via schema name) — verify existing DAL does this
- [ ] Tenant-scoping middleware: extracts `:tenant_id` from path, sets `search_path` on the connection before downstream handlers
- [ ] Invalid tenant_id returns 404
- [ ] ABAC: extend PAK auth from T-0294 — API keys have `permissions` field with tenant list or `admin` role. Middleware rejects requests to tenants the key doesn't have access to.
- [ ] Tenant metadata stored in `public` schema (not per-tenant)

## Implementation Notes

### Existing code to leverage
- `crates/cloacina/src/database/admin.rs` — `create_tenant()`, `remove_tenant()` already handle schema creation/deletion with migrations
- `crates/cloacina/src/python/bindings/admin.rs` — Python bindings already wrap these (shows the API surface)
- `examples/features/multi-tenant/` — existing multi-tenant example

### Files to create/modify
- `crates/cloacinactl/src/server/routes/tenants.rs` — REST endpoints wrapping existing DAL
- `crates/cloacinactl/src/server/middleware/tenant.rs` — tenant-scoping middleware (search_path)
- `crates/cloacinactl/src/server/auth.rs` — extend with ABAC tenant permission check

### Depends on
- T-0293 (axum server)
- T-0294 (PAK auth — extend with ABAC)

## Cherry-pick from `feat/api-server-i0049`

- `crates/cloacinactl/src/server/tenants.rs` (149 lines) — tenant CRUD endpoints, uses `DatabaseAdmin`/`TenantConfig` which exist on main
- `crates/cloacinactl/src/server/tenant_scope.rs` (163 lines) — tenant-scoping middleware

**Adaptation:** Imports look clean — `DatabaseAdmin`/`TenantConfig` are on main. May need minor type adjustments.

## Status Updates

*To be added during implementation*
