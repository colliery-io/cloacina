---
id: add-v1-version-prefix-to-all-core
level: task
title: "Add /v1/ version prefix to all core REST routes (API-03)"
short_code: "CLOACI-T-0449"
created_at: 2026-04-08T23:47:05.461246+00:00
updated_at: 2026-04-09T00:03:46.828651+00:00
parent: CLOACI-I-0087
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0087
---

# Add /v1/ version prefix to all core REST routes (API-03)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0087]]

## Objective

Core REST routes (`/auth/keys`, `/tenants`, `/tenants/{id}/workflows`, `/tenants/{id}/executions`, `/tenants/{id}/triggers`) lack a `/v1/` prefix, while newer CG routes correctly use it (`/v1/ws/accumulator/{name}`, `/v1/health/accumulators`). This prevents safe API evolution — any breaking change to core routes has no migration path.

**Effort**: 3-4 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All authenticated routes nested under `/v1/` in the axum router
- [ ] Old routes (without `/v1/`) kept as aliases during deprecation period (optional — discuss with owner)
- [ ] Health/ready/metrics remain at root (`/health`, `/ready`, `/metrics`) — these are infrastructure endpoints
- [ ] CG routes already at `/v1/` remain unchanged
- [ ] All integration tests, soak tests, and auth tests updated to use `/v1/` prefix
- [ ] Server startup log lists the versioned base URL

## Implementation Notes

### Technical Approach

In `build_router()` (`crates/cloacinactl/src/commands/serve.rs`):
1. Nest the authenticated route group under `/v1/` instead of root
2. Current: `.route("/auth/keys", ...)` becomes `.route("/v1/auth/keys", ...)`
3. Current: `.route("/tenants", ...)` becomes `.route("/v1/tenants", ...)`
4. Keep `/health`, `/ready`, `/metrics` at root (no version prefix for infra)
5. Update all test URLs in `auth_integration.py`, `server_soak.py`, `ws_integration.py`

### Dependencies
Should be done alongside or after T-0448 (error format) since both modify route registration.

## Status Updates

- **2026-04-08**: Used axum `nest("/v1", auth_routes)` to prefix all authenticated routes under `/v1/`. Health/ready/metrics remain at root. CG routes (already `/v1/`) and WS routes unchanged. Updated 28 URLs in auth_integration.py and 14 URLs in server_soak.py. Unit tests only hit `/health`/`/ready`/`/metrics` — no changes needed. Compiles clean.
