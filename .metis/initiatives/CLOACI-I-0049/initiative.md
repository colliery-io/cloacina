---
id: server-daemon-deployment
level: initiative
title: "Server & Daemon — Deployment Infrastructure"
short_code: "CLOACI-I-0049"
created_at: 2026-03-26T05:34:56.254874+00:00
updated_at: 2026-03-26T05:34:56.254874+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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

- `cloacinactl serve` starts, serves health endpoint, accepts workflow uploads
- `cloacinactl daemon` watches directory, loads `.cloacina` packages, runs cron schedules
- Auth system (PAK + ABAC) protects server endpoints
- Tenant isolation via Postgres schemas
- `angreal soak --mode server` and `angreal soak --mode daemon` pass
- Docker compose for Postgres + server works end-to-end
- All unit and integration tests pass

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

## Implementation Notes

Key fixes to apply from archive learnings:
- Use `route_layer` for auth middleware from the start
- Default bind `0.0.0.0`
- Postgres outbox uses DB `NOW()`
- Cron poll 50ms default, catchup `run_all`
- Pin Dockerfile to specific Rust version

## Alternatives Considered

- **Single mode only (server):** Rejected because many users need a lightweight local daemon without Postgres overhead.
- **gRPC instead of REST:** Rejected for now; REST is simpler for initial adoption. gRPC can be added later as an alternative transport.
- **Embedded HTTP in daemon mode:** Rejected to keep daemon minimal and avoid auth complexity for local-only use.
