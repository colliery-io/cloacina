---
id: diesel-schema-declarations-and
level: task
title: "Diesel schema declarations and model structs for auth tables"
short_code: "CLOACI-T-0185"
created_at: 2026-03-16T20:00:59.838724+00:00
updated_at: 2026-03-16T20:24:48.601705+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Diesel schema declarations and model structs for auth tables

## Objective

Add Diesel schema declarations for the three auth tables and define Rust model structs for reading/writing rows. These are the Rust-side type definitions that map to the database tables created in CLOACI-T-0184.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `tenants`, `api_keys`, `api_key_workflow_patterns` table! macros added to schema.rs in all three backend sections (postgres, sqlite, common)
- [ ] `allow_tables_to_appear_in_same_query!` updated to include the new tables
- [ ] `TenantRow` struct with Queryable + Selectable derives (id, name, schema_name, status, created_at, updated_at)
- [ ] `NewTenant` struct with Insertable derive (name, schema_name)
- [ ] `ApiKeyRow` struct with Queryable + Selectable (id, tenant_id Option, key_hash, key_prefix, name Option, can_read, can_write, can_execute, can_admin, created_at, expires_at Option, revoked_at Option)
- [ ] `NewApiKey` struct with Insertable (tenant_id, key_hash, key_prefix, name, can_read, can_write, can_execute, can_admin, expires_at)
- [ ] `WorkflowPatternRow` struct with Queryable + Selectable (id, api_key_id, pattern)
- [ ] `NewWorkflowPattern` struct with Insertable (api_key_id, pattern)
- [ ] All structs compile against both Postgres and SQLite backends

## Implementation Notes

### Schema Declarations
- Follow the existing pattern in schema.rs where each table is declared in postgres, sqlite, and common cfg blocks
- Add joinable! for api_keys -> tenants and api_key_workflow_patterns -> api_keys

### Model Structs
- Place in `dal/unified/models.rs` following the existing `AccumulatorStateRow` / `NewAccumulatorState` pattern
- Use `#[diesel(table_name = ...)]` and `#[diesel(check_for_backend(...))]` attributes
- UUID fields use `uuid::Uuid`, timestamps use `chrono::NaiveDateTime` (or `DateTime<Utc>` depending on backend wrapper)

### Dependencies
- Requires CLOACI-T-0184 (migrations must exist for schema to reference)

## Status Updates

### 2026-03-16 — Completed
- Added tenants, api_keys, api_key_workflow_patterns to all 3 schema sections + all allow_tables_to_appear_in_same_query blocks
- Added joinable! macros for FK relationships
- Added 6 model structs: TenantRow/NewTenant, ApiKeyRow/NewApiKey, WorkflowPatternRow/NewWorkflowPattern
- Used UniversalBool for boolean fields to match unified schema DbBool type
