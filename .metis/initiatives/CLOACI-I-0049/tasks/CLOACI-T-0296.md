---
id: workflow-package-api-upload-list
level: task
title: "Workflow package API — upload/list/get/delete .cloacina packages per tenant"
short_code: "CLOACI-T-0296"
created_at: 2026-03-29T14:03:30.287497+00:00
updated_at: 2026-03-29T14:03:30.287497+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# Workflow package API — upload/list/get/delete .cloacina packages per tenant

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

REST API for uploading, listing, inspecting, and removing `.cloacina` workflow packages within a tenant's scope. Upload uses **multipart form data** (packages can be several MB). Packages stored in Postgres via the existing `WorkflowRegistryImpl` DAL and registered via the reconciler.

## Acceptance Criteria

- [ ] `POST /tenants/:tenant_id/workflows` — **multipart upload** of `.cloacina` file. Streams the file (not buffered entirely in memory). Validates manifest, stores in DB via `WorkflowRegistry::register_workflow`, triggers reconciler.
- [ ] `GET /tenants/:tenant_id/workflows` — list registered workflows (name, version, task count, triggers)
- [ ] `GET /tenants/:tenant_id/workflows/:name` — workflow details (manifest, tasks, triggers, upload time)
- [ ] `DELETE /tenants/:tenant_id/workflows/:name/:version` — unregister workflow, remove binary
- [ ] Duplicate upload (same name+version) returns 409 Conflict
- [ ] Invalid/corrupt package returns 400 with validation errors
- [ ] Configurable max upload size (default 50MB)
- [ ] All operations scoped to tenant schema

## Implementation Notes

### Multipart upload
- Use `axum::extract::Multipart` (requires `multipart` feature on axum, added in T-0293)
- Stream the file field to a `Vec<u8>`, then pass to `WorkflowRegistry::register_workflow()`
- Set `tower_http::limit::RequestBodyLimitLayer` for max upload size
- Content-Type: `multipart/form-data` with a `file` field

### Example upload
```bash
curl -X POST http://localhost:8080/tenants/acme/workflows \
  -H "Authorization: Bearer clk_..." \
  -F "file=@my-workflow.cloacina"
```

### Depends on
- T-0293 (axum server with multipart)
- T-0294 (auth)
- T-0295 (tenant scoping)

## Cherry-pick from `feat/api-server-i0049`

- `crates/cloacinactl/src/server/workflows.rs` (345 lines) — upload/list/get/delete endpoints

**Adaptation:** Uses `UnifiedRegistryStorage` and `WorkflowRegistryImpl` which exist on main. Package format changed to fidius source packages — upload handler needs to validate bzip2 tar + package.toml instead of gzip tar + manifest.json. The reconciler compilation step happens automatically on load.

## Status Updates

*To be added during implementation*
