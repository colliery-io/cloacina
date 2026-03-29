---
id: server-daemon-deployment
level: initiative
title: "Server & Daemon — Deployment Infrastructure"
short_code: "CLOACI-I-0049"
created_at: 2026-03-26T05:34:56.254874+00:00
updated_at: 2026-03-29T14:03:16.743695+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: L
initiative_id: server-daemon-deployment
---

# API Server — HTTP Service with Postgres

## Context

Split from original I-0049 (Server & Daemon). Daemon is now I-0057.

`cloacinactl serve` is the production deployment mode — HTTP API backed by Postgres, multi-tenant, with PAK+ABAC auth. Endpoints for workflow upload, execution, scheduling, tenant management, and metrics.

Implemented in the archive branches (`archive/main-pre-reset`, `archive/cloacina-server-week1`). Key learnings from that work:

- Use `route_layer` not `layer` for axum auth middleware (prevents 404-to-503 regression)
- Server default bind should be `0.0.0.0` not `127.0.0.1`
- Postgres `mark_ready` must use DB `NOW()` not Rust chrono (clock skew in Docker)
- Daemon needs proper cron poll interval (50ms default, not 30s)
- Catchup policy should default to `run_all`
- Soak tests (both daemon and server modes) are essential for validation
- Containerized soak test via docker-compose proved effective

Prior work: CLOACI-I-0029 (serve foundation), CLOACI-I-0031 (PAK+ABAC auth), CLOACI-I-0032 (REST API), CLOACI-I-0033 (tenant management), CLOACI-I-0035 (Prometheus metrics).

## Goals & Non-Goals

**Goals:**
- `cloacinactl serve` with Postgres backend, PAK+ABAC auth, REST API, multi-tenancy
- Cron schedule management via REST API (create/list/delete)
- Workflow upload and execution pipeline via REST API
- Prometheus metrics endpoint (`/metrics`)
- Tenant management API (create/remove schemas)
- Docker compose for local dev and soak testing
- Server soak test via angreal

**Non-Goals:**
- Daemon mode (I-0057)
- Continuous scheduling (I-0053)
- Performance benchmarking (I-0054)
- Trigger management API (future — triggers work via packages for now)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `cloacinactl serve` starts HTTP server on configurable bind address
- Health/ready/metrics endpoints respond without auth
- PAK auth protects all tenant-scoped endpoints (LRU cache for validated keys)
- Tenant CRUD creates/removes Postgres schemas
- Workflow package upload/list/get/delete scoped to tenant
- Workflow execution with status + event log queries
- Trigger schedule read-only listing
- Docker compose for Postgres + server works end-to-end
- Server soak test via angreal passes
- All unit and integration tests pass

## API Endpoints

```
GET  /health                                         — liveness (no auth)
GET  /ready                                          — readiness (no auth)
GET  /metrics                                        — Prometheus (no auth)

POST   /auth/keys                                    — create API key
GET    /auth/keys                                    — list API keys
DELETE /auth/keys/:key_id                            — revoke API key

POST   /tenants                                      — create tenant
GET    /tenants                                      — list tenants
DELETE /tenants/:tenant_id                           — remove tenant

POST   /tenants/:tenant_id/workflows                 — upload .cloacina package
GET    /tenants/:tenant_id/workflows                 — list workflows
GET    /tenants/:tenant_id/workflows/:name           — get workflow details
DELETE /tenants/:tenant_id/workflows/:name/:version  — unregister workflow

POST   /tenants/:tenant_id/workflows/:name/execute   — execute workflow
GET    /tenants/:tenant_id/executions                — list executions
GET    /tenants/:tenant_id/executions/:id            — get execution status
GET    /tenants/:tenant_id/executions/:id/events     — execution event log

GET    /tenants/:tenant_id/triggers                  — list trigger schedules
GET    /tenants/:tenant_id/triggers/:name            — trigger details + history
```

## Prior Art

Reference implementations on archive branches:
- `archive/main-pre-reset`: cloacinactl serve (`2ceb940`), PAK+ABAC auth (`963033b`), REST API (`01a42db`), tenant management (`05d003a`), pipeline claiming (`ee32916`), Prometheus metrics (`60b1f4a`)
- `archive/cloacina-server-week1`: daemon (`43968dd`), cron API (`6fb6616`), workflow upload/execute (`18b4f35`), soak tests, Docker compose

Key bugs found in archive that must be fixed during re-implementation:
- `route_layer` vs `layer` for axum auth middleware (404→503 regression)
- Postgres `mark_ready` must use DB `NOW()` not Rust `chrono::Utc::now()` (Docker clock skew)
- Server default bind `0.0.0.0` not `127.0.0.1`
- Cron poll interval 50ms default (was 30s), catchup policy `run_all` (was `skip`)
- Diesel schema must include `claimed_by` and `heartbeat_at` columns after migration

## Implementation Plan

1. **axum server + health** — `cloacinactl serve` command, axum HTTP server, `/health`, `/ready`, `/metrics`, Postgres init, configurable bind address
2. **PAK auth** — API key table, key creation/list/revoke endpoints, auth middleware with LRU cache (TTL-based), `route_layer` not `layer`
3. **Tenant management** — create/list/delete tenants via Postgres schema isolation, ABAC scoping on tenant_id
4. **Workflow package API** — upload/list/get/delete `.cloacina` packages scoped to tenant, reconciler integration
5. **Execution API** — execute workflow, list/get pipeline executions, task status, event log
6. **Trigger schedule API** — read-only trigger listing + recent execution history
7. **Docker compose + soak test** — Postgres + server containerized, `angreal soak --mode server`

## Key Design Decisions

- **Auth middleware**: LRU cache for validated PAK keys (avoids DB hit per request). Cache entries have TTL. Use `route_layer` not `layer` (archive learning: prevents 404→503 regression).
- **Tenant isolation**: Each tenant gets its own Postgres schema. All queries scoped to tenant schema via `SET search_path`.
- **Bind address**: Default `0.0.0.0` not `127.0.0.1` (archive learning).
- **Postgres timestamps**: Use DB `NOW()` not Rust `chrono::Utc::now()` for `mark_ready` (archive learning: Docker clock skew).

## Archive Learnings

Key bugs found in prior implementation that must be fixed:
- `route_layer` vs `layer` for axum auth middleware (404→503 regression)
- Postgres `mark_ready` must use DB `NOW()` not Rust `chrono::Utc::now()` (Docker clock skew)
- Server default bind `0.0.0.0` not `127.0.0.1`
- Diesel schema must include `claimed_by` and `heartbeat_at` columns (done in I-0055)
- Pin Dockerfile to specific Rust version

## Alternatives Considered

- **Single mode only (server):** Rejected because many users need a lightweight local daemon without Postgres overhead.
- **gRPC instead of REST:** Rejected for now; REST is simpler for initial adoption. gRPC can be added later as an alternative transport.
- **Embedded HTTP in daemon mode:** Rejected to keep daemon minimal and avoid auth complexity for local-only use.
