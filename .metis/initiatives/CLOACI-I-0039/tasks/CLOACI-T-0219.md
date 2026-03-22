---
id: fix-auth-bypass-reject-protected
level: task
title: "Fix auth bypass — reject protected endpoints when no database configured"
short_code: "CLOACI-T-0219"
created_at: 2026-03-22T00:34:21.080530+00:00
updated_at: 2026-03-22T00:44:14.704703+00:00
parent: CLOACI-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0039
---

# Fix auth bypass — reject protected endpoints when no database configured

## Objective

**Severity: CRITICAL.** When `auth_state` is `None` (no database URL configured), the auth middleware is simply not applied to protected routes. All endpoints (`/executions`, `/workflows`, `/tenants`) become fully unauthenticated. Combined with the default 0.0.0.0 bind, a freshly started server is wide open to the network.

**Location:** `crates/cloacinactl/src/commands/serve.rs:164-172`

Also fix: `/auth-test` endpoint is exposed unconditionally (should be debug-only).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] When no database is configured, protected routes return 503 Service Unavailable (not silently unauthenticated)
- [ ] Health endpoint (`/health`) remains accessible without auth
- [ ] Swagger UI remains accessible without auth
- [ ] `/auth-test` endpoint gated behind `#[cfg(debug_assertions)]` or removed from production builds
- [ ] Integration test: start server without DB URL, verify POST /executions returns 503
- [ ] Integration test: start server with DB URL, verify auth middleware is applied

## Implementation Notes

Replace the `if let Some(ref auth)` branch with a rejection layer when auth is None:
```rust
let protected_routes = if let Some(ref auth) = state.auth_state {
    protected_routes.layer(auth_middleware)
} else {
    // Apply a layer that rejects all requests with 503
    protected_routes.layer(rejection_middleware)
};
```

## Status Updates

### 2026-03-21 — Complete

- Added `reject_no_auth` middleware that returns 503 when no DB configured
- Protected routes now get rejection layer instead of no middleware
- Warning logged at startup when auth is disabled
- `/auth-test` gated behind `#[cfg(debug_assertions)]`
- 488 tests pass
