---
id: authcache-in-memory-cache-with-ttl
level: task
title: "AuthCache: in-memory cache with TTL, prefix lookup, negative caching"
short_code: "CLOACI-T-0188"
created_at: 2026-03-16T20:01:02.512412+00:00
updated_at: 2026-03-16T20:30:34.337495+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# AuthCache: in-memory cache with TTL, prefix lookup, negative caching

## Objective

Implement an in-memory auth cache with TTL-based expiry, prefix-based lookup, and negative caching. This cache sits between the AuthExtract middleware and the database, preventing a DB query on every request.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AuthCache` struct using `Arc<parking_lot::RwLock<HashMap<String, CacheEntry>>>` keyed by prefix
- [ ] `CacheEntry` enum: `Found { keys: Vec<CachedKey>, cached_at: Instant }` or `NotFound { cached_at: Instant }`
- [ ] `CachedKey` contains: key_hash, key_id, tenant_id, can_read, can_write, can_execute, can_admin, expires_at, revoked_at, workflow_patterns (pre-loaded)
- [ ] `lookup(prefix) -> Option<CacheEntry>` returns None on miss, the entry on hit (caller checks staleness)
- [ ] `insert(prefix, keys: Vec<CachedKey>)` caches found keys
- [ ] `insert_not_found(prefix)` caches a negative result to prevent DB hammering
- [ ] `invalidate(prefix)` removes entry (used on key create/revoke)
- [ ] `is_stale(entry, ttl) -> bool` checks if cached_at + ttl < now
- [ ] TTL configurable via constructor, default 60 seconds
- [ ] Unit tests: insert then lookup returns hit, TTL expiry returns stale, negative cache works, invalidate clears entry, concurrent read/write access is safe

## Implementation Notes

### Structure
- `AuthCache::new(ttl: Duration)` constructor
- Inner map is `parking_lot::RwLock` for low-contention concurrent access (reads don't block reads)
- `Arc`-wrapped so it can be shared across Tower middleware instances and axum State

### Negative Caching
- When a prefix lookup hits the DB and finds no matching keys, cache `NotFound` for TTL duration
- Prevents attackers from hammering the DB with invalid prefixes
- `lookup` returns `Some(NotFound)` — caller treats this as "definitely no keys" without querying DB

### Cache Lifecycle
- On key create: `invalidate(prefix)` so next request re-fetches
- On key revoke: `invalidate(prefix)` for immediate same-instance effect
- Cross-instance revocations propagate naturally within TTL (acceptable for API keys)

### Dependencies
- No DAL dependency — pure in-memory data structure
- Used by CLOACI-T-0189 (AuthExtract middleware)

## Status Updates

### 2026-03-16 — Completed
- Created auth/cache.rs: AuthCache with parking_lot::RwLock HashMap, CachedKey struct, TTL-based expiry, negative caching, invalidation
- 5 unit tests: insert/lookup, TTL expiry, negative cache, invalidation, miss
