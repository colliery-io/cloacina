---
id: api-hardening-tls-rate-limiting
level: initiative
title: "API Hardening — TLS, Rate Limiting, Error Format, and Versioning"
short_code: "CLOACI-I-0087"
created_at: 2026-04-08T10:46:49.259470+00:00
updated_at: 2026-04-08T10:46:49.259470+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: api-hardening-tls-rate-limiting
---

# API Hardening — TLS, Rate Limiting, Error Format, and Versioning Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 3: API Hardening*

## Context

The REST API lacks several production-readiness features: no TLS (all traffic cleartext including API keys), no rate limiting (brute-force and upload abuse unmitigated), no consistent error response format (ad-hoc JSON strings), and core routes lack API versioning (newer CG routes correctly use `/v1/` but tenant/workflow/auth routes don't). Tenant credentials are returned in plaintext in HTTP responses.

## Goals & Non-Goals

**Goals:**
- Add TLS support or document reverse proxy requirement (SEC-06)
- Add rate limiting to HTTP endpoints (SEC-07, SEC-13)
- Standardize REST API error responses with machine-readable codes (API-02)
- Add `/v1/` version prefix to all core routes (API-03)
- Protect tenant credentials in API responses (SEC-08, SEC-05)

**Non-Goals:**
- Full API redesign
- GraphQL or gRPC support

## Detailed Design

### REC-07: TLS Support (SEC-06) — 1-2 days (docs) or 3-5 days (native)
Option A (minimum): Document reverse proxy requirement with example configs.
Option B (preferred): Add `axum-server` + `rustls` with `--tls-cert`/`--tls-key` CLI options.

### REC-08: Rate Limiting (SEC-07, SEC-13) — 1-2 days
Add `tower_governor` for per-IP rate limiting. Auth endpoints: 10 req/s. Upload: 2 req/s. Read: 100 req/s. Add `DefaultBodyLimit::max(100MB)` to router. WebSocket connection limits via `Semaphore`.

### REC-09: Error Format and Versioning (API-02, API-03) — 2-3 days
Define `ApiError` struct with `error`, `code`, `status`, `request_id` fields. Implement `IntoResponse`. Add `/v1/` prefix to all authenticated routes. Standardize status casing (lowercase).

### REC-10: Protect Tenant Credentials (SEC-08, SEC-05) — 3-4 hours
Remove password from `create_tenant` response connection string. For WebSocket auth, implement short-lived ticket exchange (`POST /auth/ws-ticket` returns single-use, time-limited ticket).

## Implementation Plan

REC-09 (error format) first as it touches all handlers. REC-08 and REC-10 in parallel. REC-07 last. Target: 2-3 weeks.
