---
id: server-phase-5-tenant-management
level: initiative
title: "Server Phase 5: Tenant Management API"
short_code: "CLOACI-I-0033"
created_at: 2026-03-16T01:32:36.540330+00:00
updated_at: 2026-03-17T00:46:32.795899+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: server-phase-5-tenant-management
---

# Server Phase 5: Tenant Management API Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0031 (Auth — tenant creation generates initial API key)
**Blocks**: None

## Context

Multi-tenancy via schema isolation already exists, but tenants are hardcoded at startup. This phase makes tenant provisioning dynamic via API — create, list, configure, and deprovision tenants at runtime without server restarts.

## Goals

- `POST /tenants` — create tenant (creates Postgres schema, runs migrations, returns initial API key)
- `GET /tenants` — list tenants
- `GET /tenants/{tenant}` — tenant details + stats (active pipelines, task counts)
- `DELETE /tenants/{tenant}` — deprovision (soft delete/archive)
- `POST /tenants/{tenant}/api-keys` — create key (returns secret once)
- `GET /tenants/{tenant}/api-keys` — list keys (metadata only)
- `DELETE /tenants/{tenant}/api-keys/{id}` — revoke key
- Optional scheduler tenant affinity: `--tenants=a,b` flag for QoS isolation

## Detailed Design

### Existing Infrastructure (from I-0031)

- `TenantDAL`: create, list, get, get_by_name, deactivate — already implemented
- `ApiKeyDAL`: create, create_patterns, load_by_prefix, list_by_tenant, list_all, revoke — already implemented
- `generate_api_key()`, `hash_key()` — already implemented in security/api_keys.rs
- `AuthContext` with `can_admin` bit — already checked by PermissionGuard
- `require_admin` middleware — already implemented

### Endpoint Design

All tenant management endpoints require `can_admin` permission.

| Endpoint | Method | Request | Response | Notes |
|---|---|---|---|---|
| `/tenants` | POST | `{"name": "acme", "schema_name": "tenant_acme"}` | 201 `{"id": uuid, "name": ..., "initial_api_key": secret}` | Creates Postgres schema + runs migrations + generates admin key |
| `/tenants` | GET | — | 200 `[{"id", "name", "schema_name", "status", "created_at"}]` | Admin only |
| `/tenants/{id}` | GET | — | 200 `{"id", "name", "schema_name", "status", ...}` | Admin or own tenant |
| `/tenants/{id}` | DELETE | — | 200 `{"status": "deactivated"}` | Soft delete via deactivate |
| `/tenants/{id}/api-keys` | POST | `{"name": "ci-key", "read": true, "execute": true, ...}` | 201 `{"id": uuid, "secret": key}` | Secret shown once |
| `/tenants/{id}/api-keys` | GET | — | 200 `[{"id", "name", "permissions", "created_at", ...}]` | No secrets |
| `/tenants/{id}/api-keys/{key_id}` | DELETE | — | 200 `{"status": "revoked"}` | Invalidates auth cache |

### Schema Provisioning

`POST /tenants` must:
1. Insert into `tenants` table
2. Create Postgres schema: `CREATE SCHEMA IF NOT EXISTS tenant_<name>`
3. Run migrations within the new schema (same migrations as main schema)
4. Generate an initial admin API key for the tenant
5. Return the tenant info + the initial key secret

This uses the existing `Database::try_new_with_schema()` pattern for schema creation.

### Auth Cache Invalidation

When a key is created or revoked via the API, call `auth_cache.invalidate(prefix)` so the change takes effect immediately on the same server instance. Other instances pick it up within TTL.

## Implementation Plan

- [ ] Create `routes/tenants.rs` with all 7 endpoint handlers
- [ ] Tenant CRUD handlers: create (with schema provisioning), list, get, deactivate
- [ ] Tenant API key handlers: create (with auth cache invalidation), list, revoke (with cache invalidation)
- [ ] Wire tenant routes into protected Router with require_admin guard
- [ ] utoipa annotations on all tenant endpoints
- [ ] Integration tests: create tenant, create key for tenant, list, revoke, deactivate
