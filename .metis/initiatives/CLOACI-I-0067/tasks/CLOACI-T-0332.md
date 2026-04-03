---
id: t1-http-api-handler-tests
level: task
title: "T1: HTTP API handler tests"
short_code: "CLOACI-T-0332"
created_at: 2026-04-03T02:36:29.489732+00:00
updated_at: 2026-04-03T10:15:23.551642+00:00
parent: CLOACI-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0067
---

# T1: HTTP API handler tests

## Parent Initiative
[[CLOACI-I-0067]] — Tier 1 (highest impact)

## Objective
Add handler-level tests for every HTTP API endpoint in crates/cloacinactl/src/server/. Currently zero test coverage — only validated by soak test.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Auth middleware tests: no token → 401, invalid token → 401, valid token → passes through
- [ ] POST /auth/keys: valid request → 201 + key returned, missing name → 400
- [ ] GET /auth/keys: returns list, respects auth
- [ ] DELETE /auth/keys/:id: valid → 200, nonexistent → 404
- [ ] POST /tenants: valid → 201, duplicate schema → error, invalid chars → 400
- [ ] GET /tenants: returns list
- [ ] DELETE /tenants/:name: valid → 200, nonexistent → 404
- [ ] POST /tenants/:id/workflows: valid multipart → 201, corrupt package → 400, wrong content-type → 400
- [ ] GET /tenants/:id/workflows: returns list (empty and populated)
- [ ] POST /tenants/:id/workflows/:name/execute: valid → 202, nonexistent workflow → 404
- [ ] GET /tenants/:id/executions: returns list
- [ ] GET /tenants/:id/executions/:id: valid → 200, nonexistent → 404
- [ ] GET /tenants/:id/executions/:id/events: returns event list
- [ ] GET /tenants/:id/triggers: returns list (empty and populated)
- [ ] GET /health → 200, GET /ready → 200 (with DB), GET /metrics → 200
- [ ] All tests run against real Postgres (use docker-compose)
- [ ] Tests added to `angreal cloacina integration` or new `angreal cloacina server-tests` task

## Source Files
- crates/cloacinactl/src/server/auth.rs
- crates/cloacinactl/src/server/keys.rs
- crates/cloacinactl/src/server/tenants.rs
- crates/cloacinactl/src/server/workflows.rs
- crates/cloacinactl/src/server/executions.rs
- crates/cloacinactl/src/server/triggers.rs
- crates/cloacinactl/src/commands/serve.rs (build_router)

## Technical Approach
Use axum's built-in test utilities (tower::ServiceExt, oneshot requests) or axum-test crate. Create a test harness that spins up AppState with a real Postgres connection, creates a bootstrap key, then exercises each handler.

## Status Updates

### 2026-04-02 — Implementation complete (27 tests, all passing)

**Changes:**
- `crates/cloacinactl/Cargo.toml`: added `http-body-util`, `serial_test` dev-deps; added `util` feature to `tower`
- `crates/cloacinactl/src/commands/serve.rs`: added `#[cfg(test)] mod tests` with 27 handler tests

**Coverage:** Health/Ready/Metrics (3), Auth middleware (4), Key management (6), Tenants (3), Workflows (4), Executions (5), Triggers (2), Fallback 404 (1). All run against real Postgres with `serial_test`.

**Finding:** `remove_tenant` is idempotent (DROP SCHEMA IF EXISTS) — returns 200 even for nonexistent schemas.
