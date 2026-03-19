---
id: wire-auth-middleware-into-axum
level: task
title: "Wire auth middleware into axum Router with public route bypass"
short_code: "CLOACI-T-0192"
created_at: 2026-03-16T20:01:06.972997+00:00
updated_at: 2026-03-16T20:37:35.985279+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Wire auth middleware into axum Router with public route bypass

## Objective

Wire the auth middleware stack (AuthExtract, TenantScope, PermissionGuard) into the axum Router in `serve.rs`, splitting routes into public (no auth) and protected (auth required) groups.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Router split into public and protected route groups
- [ ] Public routes: `/health`, `/api-docs/*`, `/metrics` — no auth middleware applied
- [ ] Protected routes: all other endpoints — AuthExtract + TenantScope applied as Tower layers
- [ ] PermissionGuard layers applied per route group (placeholders for subsequent API phases)
- [ ] `AuthCache` created at server startup and shared via `Arc`
- [ ] DAL access available to auth middleware (via axum State or Extension)
- [ ] Public routes remain accessible without any Bearer token
- [ ] Protected routes return 401 when accessed without valid token
- [ ] Server compiles and starts with the layered router

## Implementation Notes

### Router Composition
- Use axum's `Router::merge` to combine public and protected routers
- Public router: `Router::new().route("/health", get(health_handler)).route("/metrics", get(metrics_handler))...`
- Protected router: `Router::new().route(...)....layer(TenantScopeLayer::new()).layer(AuthExtractLayer::new(cache, dal))`
- Layers apply outermost-first: AuthExtract runs before TenantScope

### Shared State
- `AuthCache` instantiated in `serve.rs` app construction, wrapped in `Arc`
- DAL pool/connection available via existing axum State pattern
- Both passed to `AuthExtractLayer` constructor

### PermissionGuard Wiring
- For this task, add PermissionGuard to route groups as infrastructure — actual route definitions come in CLOACI-I-0032 (Core API)
- Example structure: `execution_routes.layer(require_permission(Permission::Execute))`

### Dependencies
- CLOACI-T-0189 (AuthExtract), CLOACI-T-0190 (TenantScope, PermissionGuard)
- Existing serve.rs app() function from CLOACI-I-0029 (Foundation)

## Status Updates

### 2026-03-16 — Completed
- Split Router into public (health, swagger) and protected (empty, ready for API endpoints) route groups
- Auth middleware applied to protected routes only when AuthState is available
- AuthState created from DB config in run() — AuthCache(60s TTL) + Arc<DAL>
- auth_state: None when no DB configured (api-only mode without DB)
- Updated all test AppState constructions with auth_state: None
