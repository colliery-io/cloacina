---
id: tower-authextract-middleware
level: task
title: "Tower AuthExtract middleware: Bearer extraction, cache lookup, hash verify"
short_code: "CLOACI-T-0189"
created_at: 2026-03-16T20:01:03.838517+00:00
updated_at: 2026-03-16T20:30:50.494836+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Tower AuthExtract middleware: Bearer extraction, cache lookup, hash verify

## Objective

Implement the Tower AuthExtract middleware — the first layer in the auth middleware stack. It intercepts every request on protected routes, extracts the Bearer token, resolves it via cache + DB, verifies the argon2 hash, and injects an `AuthContext` into request extensions for downstream middleware and handlers.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AuthExtractLayer` and `AuthExtractService` implement Tower Layer + Service traits
- [ ] Extracts token from `Authorization: Bearer <token>` header
- [ ] Derives prefix from token, checks AuthCache first
- [ ] On cache miss or stale entry: queries ApiKeyDAL.load_by_prefix(), caches result, then verifies
- [ ] On cache hit with negative entry: returns 401 immediately (no DB query)
- [ ] Verifies argon2 hash of full token against each cached key for the prefix
- [ ] Checks expiry (expires_at) and revocation (revoked_at) — rejects if expired or revoked
- [ ] On success: injects `AuthContext` into request extensions and calls inner service
- [ ] `AuthContext` struct: key_id (Uuid), tenant_id (Option<Uuid>), can_read, can_write, can_execute, can_admin (booleans), workflow_patterns (Vec<String>)
- [ ] Returns 401 Unauthorized with JSON error body `{"error": "..."}` on missing, invalid, expired, or revoked key
- [ ] Skips auth for configurable public paths (health, metrics, api-docs)

## Implementation Notes

### Tower Pattern
- `AuthExtractLayer` holds `Arc<AuthCache>` and DAL access (via `Arc<dyn ...>` or concrete type)
- `AuthExtractService<S>` wraps inner service `S`, implements `Service<Request<Body>>`
- The `call` method is async — extract header, do cache/DB lookup, verify, then call inner

### Request Flow
1. Extract `Authorization` header, parse `Bearer <token>`
2. `extract_prefix(token)` to get cache key
3. `cache.lookup(prefix)` — if hit and not stale, use cached keys; if miss/stale, query DB
4. If DB returns empty, `cache.insert_not_found(prefix)`, return 401
5. For each cached key matching prefix, `verify_api_key(token, key.key_hash)` — first match wins
6. Check `expires_at` and `revoked_at` on matched key
7. Build `AuthContext`, insert into request extensions

### Public Path Bypass
- Accept a `Vec<String>` of path prefixes to skip (e.g., `/health`, `/metrics`, `/api-docs`)
- Check request URI path before doing any auth work

### Dependencies
- CLOACI-T-0186 (ApiKeyDAL for DB fallback)
- CLOACI-T-0187 (verify_api_key, extract_prefix)
- CLOACI-T-0188 (AuthCache)

## Status Updates

### 2026-03-16 — Completed
- Created auth/context.rs: AuthContext struct with key_id, tenant_id, permissions, patterns, is_global() helper
- Created auth/middleware.rs: auth_middleware function using axum::middleware::from_fn_with_state
- AuthState holds AuthCache + Arc<DAL> for DB fallback
- Bearer extraction → prefix lookup → cache-or-DB → argon2 verify → revocation/expiry check → inject AuthContext
- Returns 401 JSON for invalid/missing/expired/revoked keys, 500 for DB errors
