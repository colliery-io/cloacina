---
id: tower-tenantscope-middleware-and
level: task
title: "Tower TenantScope middleware and PermissionGuard route layer"
short_code: "CLOACI-T-0190"
created_at: 2026-03-16T20:01:04.775056+00:00
updated_at: 2026-03-16T20:30:55.177427+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Tower TenantScope middleware and PermissionGuard route layer

## Objective

Implement two composable middleware layers that sit after AuthExtract: TenantScope (enforces tenant isolation) and PermissionGuard (checks permission bits per route group). Together with AuthExtract, these form the complete auth middleware stack.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TenantScope` middleware reads `AuthContext` from extensions
- [ ] If `AuthContext.tenant_id` is Some, verifies it matches the tenant ID in the URL path — returns 403 on mismatch
- [ ] If `AuthContext.tenant_id` is None (global/super-admin), allows access to any tenant
- [ ] Injects resolved `tenant_id` into request extensions for downstream handlers
- [ ] `PermissionGuard` is a configurable axum layer — factory: `require_permission(Permission::Execute)` returns a layer
- [ ] `Permission` enum with variants: Read, Write, Execute, Admin
- [ ] PermissionGuard reads `AuthContext` from extensions, checks the corresponding bit, returns 403 Forbidden with JSON body if not set
- [ ] Both middlewares are composable — applied per route group in the Router via `.layer()`
- [ ] 403 responses include JSON error body `{"error": "..."}`

## Implementation Notes

### TenantScope
- Reads `AuthContext` from request extensions (inserted by AuthExtract upstream)
- Extracts tenant identifier from URL path (e.g., `/tenants/:tenant_id/...` path parameter)
- Global keys (tenant_id = None) bypass tenant check — they can access any tenant's resources
- Tenant-scoped keys must match exactly

### PermissionGuard
- Generic over the required permission — `require_permission(Permission::Write)` creates a layer that checks `can_write`
- Simple bit check on `AuthContext` — no DB or cache access needed
- Apply per route group: e.g., execution routes get `require_permission(Permission::Execute)`, admin routes get `require_permission(Permission::Admin)`

### Composability
- These are standard Tower layers, applied via axum's `.layer()` on Router or MethodRouter
- Order: AuthExtract (outermost) -> TenantScope -> PermissionGuard (innermost, per route group)

### Dependencies
- CLOACI-T-0189 (AuthExtract must run first to populate AuthContext in extensions)

## Status Updates

### 2026-03-16 — Completed
- Permission enum (Read, Write, Execute, Admin) in middleware.rs
- require_read/write/execute/admin middleware functions for route-level guards
- check_permission helper reads AuthContext from extensions, returns 403 if bit not set, 401 if not authenticated
- Composable — apply per route group via axum::middleware::from_fn
