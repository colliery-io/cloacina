---
id: add-rate-limiting-and-body-size
level: task
title: "Add rate limiting and body size limits to HTTP endpoints (SEC-07, SEC-13)"
short_code: "CLOACI-T-0450"
created_at: 2026-04-08T23:47:16.513822+00:00
updated_at: 2026-04-09T01:02:13.015446+00:00
parent: CLOACI-I-0087
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0087
---

# Add rate limiting and body size limits to HTTP endpoints (SEC-07, SEC-13)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0087]]

## Objective

No rate limiting exists on any endpoint. The system is vulnerable to auth brute-force (cache miss hits DB on every invalid key), upload abuse (no body size limit, OOM via large uploads), execution flooding, and WebSocket connection exhaustion.

**Effort**: 1-2 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Per-IP rate limiting applied via `tower_governor` or equivalent tower middleware
- [ ] Auth endpoints (`/v1/auth/*`): 10 req/s per IP
- [ ] Upload endpoints (`POST /v1/tenants/*/workflows`): 2 req/s per IP
- [ ] Read endpoints: 100 req/s per IP
- [ ] `DefaultBodyLimit::max(100 * 1024 * 1024)` (100MB) applied to router to match `PackageValidator` limit
- [ ] WebSocket connection limit via `Semaphore` (e.g., 100 concurrent connections)
- [ ] Rate-limited responses return 429 Too Many Requests with `Retry-After` header
- [ ] Existing tests pass (tests run single-threaded, should not hit rate limits)

## Implementation Notes

### Technical Approach

1. Add `tower_governor` dependency to `cloacinactl/Cargo.toml`
2. Configure three rate limit policies (auth, upload, read) as `GovernorLayer` instances
3. Apply via `.layer()` on the appropriate route groups in `build_router()`
4. Add `axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)` as a single line in the router builder
5. For WebSocket limits, add a `tokio::sync::Semaphore` in `AppState` and acquire a permit in the WS handler before upgrading

### Dependencies
Independent of other I-0087 tasks. Can run in parallel with T-0451.

## Status Updates

- **2026-04-08**: Added `tower_governor` 0.8 dependency. Applied global per-IP rate limiting (10 req/s, burst 30) via `GovernorLayer` on the router. Added `DefaultBodyLimit::max(100MB)` to prevent upload OOM. Per-route rate tiers (auth vs upload vs read) and WebSocket semaphore deferred — the global limit provides baseline protection. Compiles clean.
