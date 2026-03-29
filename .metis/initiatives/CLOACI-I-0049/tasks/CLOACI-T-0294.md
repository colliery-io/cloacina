---
id: pak-auth-api-key-crud-auth
level: task
title: "PAK auth — API key CRUD, auth middleware with LRU cache, route_layer"
short_code: "CLOACI-T-0294"
created_at: 2026-03-29T14:03:26.947120+00:00
updated_at: 2026-03-29T14:03:26.947120+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# PAK auth — API key CRUD, auth middleware with LRU cache, route_layer

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

Implement PAK (Pre-shared API Key) authentication: API key table, CRUD endpoints for key management, and axum auth middleware with LRU cache to avoid DB hits per request. Uses `route_layer` (not `layer`) per archive learning.

## Acceptance Criteria

- [ ] `api_keys` table in Postgres: id, key_hash, name, permissions, created_at, revoked_at
- [ ] Keys stored as SHA-256 hashes (never store plaintext)
- [ ] `POST /auth/keys` — create key, returns plaintext key once (never retrievable again)
- [ ] `GET /auth/keys` — list keys (id, name, created_at, revoked status — no hashes)
- [ ] `DELETE /auth/keys/:key_id` — revoke key (soft delete via revoked_at)
- [ ] Auth middleware extracts `Authorization: Bearer <key>` header, validates against DB
- [ ] **LRU cache**: validated keys cached in memory with configurable TTL (default 5 min). Cache evicts on revocation.
- [ ] Middleware applied via `route_layer` (not `layer`) to avoid 404→503 regression
- [ ] Unauthenticated requests to protected endpoints return 401 JSON error
- [ ] Revoked keys return 401
- [ ] Health/ready/metrics endpoints bypass auth

## Implementation Notes

### Files to create/modify
- `crates/cloacinactl/src/server/auth.rs` — middleware, LRU cache, key validation
- `crates/cloacinactl/src/server/routes/auth.rs` — key CRUD endpoints
- Migration for `api_keys` table

### Key design points
- LRU cache: `lru` crate with `Arc<Mutex<LruCache<String, CachedKeyInfo>>>`. TTL per entry — check timestamp on cache hit, evict if expired.
- Key format: `clk_` prefix + 32 random bytes base64 (visually identifiable as cloacina key)
- `route_layer` applies auth only to matched routes — unmatched routes still return 404 (not 503)
- First key can be created via CLI `cloacinactl admin create-key` for bootstrapping (no auth required on first key)

### Depends on
- T-0293 (axum server)

## Status Updates

*To be added during implementation*
