---
id: database-migrations-tenants-api
level: task
title: "Database migrations: tenants, api_keys, api_key_workflow_patterns tables"
short_code: "CLOACI-T-0184"
created_at: 2026-03-16T20:00:58.602528+00:00
updated_at: 2026-03-16T20:15:46.753835+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Database migrations: tenants, api_keys, api_key_workflow_patterns tables

## Objective

Create Postgres and SQLite database migrations for the three auth tables: `tenants`, `api_keys`, and `api_key_workflow_patterns`. These tables form the persistence layer for the PAK + ABAC authentication system.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Postgres migration `015_create_auth_tables` with up.sql and down.sql
- [ ] SQLite migration `014_create_auth_tables` with up.sql and down.sql
- [ ] `tenants` table: id (UUID PK), name (UNIQUE NOT NULL), schema_name (UNIQUE NOT NULL), status (default 'active'), created_at, updated_at
- [ ] `api_keys` table: id (UUID PK), tenant_id (FK to tenants, nullable for global keys), key_hash (TEXT NOT NULL), key_prefix (VARCHAR(32) NOT NULL), name (VARCHAR(255)), can_read/can_write/can_execute/can_admin (BOOLEAN), created_at, expires_at, revoked_at
- [ ] `api_key_workflow_patterns` table: id (UUID PK), api_key_id (FK to api_keys ON DELETE CASCADE), pattern (TEXT NOT NULL)
- [ ] Indexes on api_keys(key_prefix) and api_keys(tenant_id)
- [ ] down.sql drops all three tables in reverse dependency order
- [ ] Migrations run cleanly on both backends

## Implementation Notes

### Postgres (015_create_auth_tables)
- Use `gen_random_uuid()` for PK defaults, `TIMESTAMPTZ` for timestamps, `NOW()` for defaults
- `api_keys.tenant_id` is nullable — NULL means global/super-admin scope
- Permission booleans default: can_read=true, others=false

### SQLite (014_create_auth_tables)
- Use TEXT for timestamps and VARCHAR for UUIDs (no native UUID/TIMESTAMPTZ in SQLite)
- Foreign key constraints require `PRAGMA foreign_keys = ON` at connection time (already handled by DAL)
- Same logical schema, adapted for SQLite type system

### Dependencies
- No code dependencies — this is the foundation task for the auth initiative

## Status Updates

### 2026-03-16 — Completed
- Postgres 015_create_auth_tables: tenants (UUID PK, name UNIQUE, schema_name UNIQUE, status, timestamps), api_keys (UUID PK, tenant_id FK nullable, key_hash, key_prefix, name, 4 permission booleans, timestamps, expires_at, revoked_at), api_key_workflow_patterns (UUID PK, api_key_id FK CASCADE, pattern)
- SQLite 014_create_auth_tables: same structure with VARCHAR(36) for UUIDs, TEXT for timestamps, BOOLEAN as integer
- Indexes on key_prefix, tenant_id, api_key_id
- down.sql drops all three tables in dependency order
