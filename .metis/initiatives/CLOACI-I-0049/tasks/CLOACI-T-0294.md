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

Add the `api_keys` table, DAL operations in cloacina core, key generation utilities, and server bootstrap — on first startup if no admin keys exist, auto-create one and write the plaintext to a file (never logged).

## Acceptance Criteria

- [ ] `api_keys` table migration for Postgres + SQLite: id, key_hash, name, permissions, created_at, revoked_at
- [ ] `api_keys` table added to Diesel `schema.rs`
- [ ] DAL in cloacina core: `create_key(hash, name)`, `validate_hash(hash)` → returns key info or None, `has_any_keys()` → bool, `list_keys()`, `revoke_key(id)`
- [ ] Key generation utility: `clk_` prefix + 32 random bytes base64, SHA-256 hash for storage
- [ ] Server startup bootstrap: calls `has_any_keys()`, if false auto-creates admin key, writes plaintext to `~/.cloacina/bootstrap-key` file with `0600` permissions
- [ ] **Never log the key** — not to stdout, stderr, or file logs
- [ ] Bootstrap key file deleted by the server after configurable timeout (or left for admin to delete manually)
- [ ] Existing tests pass

## Implementation Notes

### Files to create/modify
- `crates/cloacina/src/database/migrations/` — Postgres + SQLite migration for `api_keys`
- `crates/cloacina/src/database/schema.rs` — add `api_keys` table
- `crates/cloacina/src/dal/unified/api_keys/` — new DAL module (create, validate, list, revoke, has_any)
- `crates/cloacinactl/src/commands/serve.rs` — bootstrap logic on startup

### Key design points
- DAL follows existing pattern (dispatch_backend macro, Postgres + SQLite impls)
- Key generation uses `rand` + `base64` + `sha2` — these deps go in cloacina core (not cloacinactl)
- Bootstrap file: `~/.cloacina/bootstrap-key` with mode `0600` (owner read/write only)
- No CLI `admin create-key` command — all key management through the API (T-0300)

### Depends on
- T-0293 (axum server)

## Cherry-pick from `feat/api-server-i0049`

- `crates/cloacina/src/dal/unified/api_keys/` (342 lines) — API key DAL, needs adaptation for current DAL patterns
- `crates/cloacina/src/security/api_keys.rs` (72 lines) — key hashing/generation, likely clean
- `crates/cloacina/src/database/migrations/postgres/014_create_api_keys/` — migration SQL, renumber to 016
- `crates/cloacinactl/src/server/auth.rs` (174 lines) — LRU cache middleware, needs `lru` dep
- `crates/cloacinactl/src/server/keys.rs` (146 lines) — key CRUD endpoints

**Adaptation:** DAL uses old dispatch_backend patterns. Migration numbering needs update (015 is taken). Auth references AppState from serve.rs.

## Status Updates

*To be added during implementation*
