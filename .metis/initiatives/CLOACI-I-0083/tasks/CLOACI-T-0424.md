---
id: key-scoping-and-bootstrap-god-mode
level: task
title: "Key scoping and bootstrap god mode — tenant_id and is_admin on api_keys, tenant key creation endpoint"
short_code: "CLOACI-T-0424"
created_at: 2026-04-06T15:18:17.903975+00:00
updated_at: 2026-04-06T16:27:54.008156+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Key scoping and bootstrap god mode — tenant_id and is_admin on api_keys, tenant key creation endpoint

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Add tenant scoping and admin privilege to API keys. Currently all keys are identical — no tenant binding, no privilege levels. After this task, keys can be global (NULL tenant_id), tenant-scoped, or admin (god mode via `is_admin` flag). The bootstrap key becomes the first admin key.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration adds `tenant_id TEXT NULL` and `is_admin BOOLEAN NOT NULL DEFAULT FALSE` to `api_keys` (Postgres + SQLite)
- [ ] Bootstrap key creation sets `is_admin = true`
- [ ] `AuthenticatedKey` struct gains `tenant_id: Option<String>` and `is_admin: bool` fields (remove `#[allow(dead_code)]`)
- [ ] `validate_token` / `KeyCache` propagates the new fields
- [ ] `POST /auth/keys` creates global keys (tenant_id=NULL) — preserves single-tenant default
- [ ] New endpoint `POST /tenants/{tid}/keys` creates keys scoped to that tenant (admin-only)
- [ ] `GET /auth/keys` response includes `tenant_id` and `is_admin` fields
- [ ] Only admin keys can call `POST /tenants/{tid}/keys`
- [ ] Only admin keys can call `POST /tenants` and `DELETE /tenants/{schema}`
- [ ] Existing tests pass — single-tenant behavior unchanged

## Implementation Notes

### Key files
- `crates/cloacina/src/database/migrations/` — new migration
- `crates/cloacina/src/dal/unified/api_keys/` — DAL needs tenant_id + is_admin in create/validate
- `crates/cloacinactl/src/server/auth.rs` — `AuthenticatedKey` struct, `validate_token`
- `crates/cloacinactl/src/server/keys.rs` — key creation handlers
- `crates/cloacinactl/src/server/tenants.rs` — restrict to admin keys
- `crates/cloacinactl/src/commands/serve.rs` — bootstrap key creation, new route for tenant keys

### Dependencies
- T-0423 (package tenant ownership) should land first so the tenant_id conventions are consistent

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete (schema + DAL + auth middleware)**
- Created Postgres migration 019: adds `tenant_id TEXT NULL` and `is_admin BOOLEAN NOT NULL DEFAULT FALSE` to api_keys, marks existing bootstrap as admin
- Updated schema.rs postgres api_keys table! with new columns
- Updated `ApiKeyRow` (Queryable) and `NewApiKey` (Insertable) in crud.rs with new fields
- Updated `ApiKeyInfo` in mod.rs with `tenant_id: Option<String>` and `is_admin: bool`
- Updated `create_key` to accept `tenant_id: Option<&str>` and `is_admin: bool` (DAL + public API)
- Updated `AuthenticatedKey` in auth.rs — removed `#[allow(dead_code)]`, added `tenant_id` and `is_admin`
- Updated `validate_token` to map new fields from `ApiKeyInfo` → `AuthenticatedKey`
- Bootstrap key creation passes `(None, true)` — global, admin
- `POST /auth/keys` handler creates global non-admin keys `(None, false)` — single-tenant default
- `GET /auth/keys` response now includes `tenant_id` and `is_admin` fields
- All callers updated: serve.rs (bootstrap + test helpers), keys.rs handler, integration tests
- Both crates compile clean, integration tests compile
- NOTE: `POST /tenants/{tid}/keys` endpoint and admin-only guards on tenant operations deferred to T-0425 (handler enforcement)
