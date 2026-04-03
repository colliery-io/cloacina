---
id: key-management-api-post-get-delete
level: task
title: "Key management API — POST/GET/DELETE /auth/keys endpoints"
short_code: "CLOACI-T-0301"
created_at: 2026-03-29T15:15:24.343526+00:00
updated_at: 2026-03-29T15:15:24.343526+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# Key management API — POST/GET/DELETE /auth/keys endpoints

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

REST API endpoints for managing API keys. All endpoints are behind auth middleware — you need an existing valid key to create/list/revoke keys.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /auth/keys` — create a new key. Request: `{"name": "my-key"}`. Response: `{"id": "...", "name": "...", "key": "clk_..."}`. Plaintext key returned once, never retrievable again.
- [ ] `GET /auth/keys` — list all keys (id, name, created_at, revoked status). No hashes or plaintext.
- [ ] `DELETE /auth/keys/:key_id` — soft-revoke key (sets `revoked_at`). Evicts from LRU cache.
- [ ] All endpoints require valid `Authorization: Bearer <key>` (behind auth middleware from T-0300)
- [ ] Duplicate key names allowed (keys are identified by id, not name)
- [ ] Revoking a non-existent or already-revoked key returns 404
- [ ] Response format: JSON with consistent error structure

## Implementation Notes

### Files to create/modify
- `crates/cloacinactl/src/server/routes/auth.rs` — route handlers calling DAL
- `crates/cloacinactl/src/commands/serve.rs` — merge auth routes into router

### Key design points
- Handlers call DAL: `dal.api_keys().create_key()`, `dal.api_keys().list_keys()`, `dal.api_keys().revoke_key()`
- Key generation (clk_ prefix + SHA-256) uses utility from T-0294
- On revoke: also evict from the `KeyCache` (access via `AppState`)
- All routes behind the auth `route_layer` from T-0300

### Depends on
- T-0294 (api_keys DAL)
- T-0300 (auth middleware)

## Cherry-pick notes

**Merged into T-0294** — key management endpoints are part of the overall auth task. Cherry-pick source: `crates/cloacinactl/src/server/keys.rs` (146 lines) from `feat/api-server-i0049`.

## Status Updates

*Merged into T-0294*
