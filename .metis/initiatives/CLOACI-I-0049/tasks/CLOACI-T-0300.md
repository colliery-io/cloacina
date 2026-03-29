---
id: auth-middleware-lru-cache-route
level: task
title: "Auth middleware — LRU cache, route_layer, Bearer token validation"
short_code: "CLOACI-T-0300"
created_at: 2026-03-29T15:15:23.228688+00:00
updated_at: 2026-03-29T15:15:23.228688+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# Auth middleware — LRU cache, route_layer, Bearer token validation

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

Add axum auth middleware that extracts `Authorization: Bearer <key>` tokens, validates against the API key DAL (from T-0294), and caches validated keys in an LRU cache with TTL to avoid DB hits per request. Applied via `route_layer` (not `layer`) per archive learning.

## Acceptance Criteria

- [ ] axum middleware extracts `Authorization: Bearer <key>` header
- [ ] Hashes token with SHA-256, calls DAL `validate_hash()` to check against DB
- [ ] **LRU cache**: validated keys cached in memory with configurable TTL (default 5 min), checked before DB
- [ ] Cache entry evicted on TTL expiry (lazy eviction on lookup)
- [ ] Applied via `route_layer` (not `layer`) — unmatched routes still return 404, not 503
- [ ] Missing/invalid header returns 401 `{"error": "..."}`
- [ ] Revoked key returns 401 (not in cache or DB returns None)
- [ ] `AuthenticatedKey` struct inserted into request extensions for downstream handlers
- [ ] Health/ready/metrics endpoints bypass auth (public routes not behind `route_layer`)
- [ ] `lru` crate added to cloacinactl

## Implementation Notes

### Files to create/modify
- `crates/cloacinactl/src/server/auth.rs` — middleware function, `KeyCache` struct, `AuthenticatedKey` struct
- `crates/cloacinactl/src/commands/serve.rs` — wire middleware into router via `route_layer`, add `KeyCache` to `AppState`

### Key design points
- `KeyCache`: `Arc<Mutex<LruCache<String, CachedKeyInfo>>>` with TTL per entry
- Middleware calls `state.dal.api_keys().validate_hash()` on cache miss
- `route_layer` applies auth only to the authenticated sub-router, not the public routes

### Depends on
- T-0293 (axum server)
- T-0294 (api_keys DAL + bootstrap)

## Status Updates

*To be added during implementation*
