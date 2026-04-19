---
id: t1-schema-dal-for-build-queue
level: task
title: "T1: Schema + DAL for build queue (compiled_data/build_status/heartbeat)"
short_code: "CLOACI-T-0519"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-19T00:22:42.817886+00:00
parent: CLOACI-I-0097
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T1: Schema + DAL for build queue (compiled_data/build_status/heartbeat)

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Land the schema changes, Diesel model fields, and DAL helpers that the compiler service (T3), sweeper (T4), upload handler (T5), and reconciler (T6) will build on. Purely additive — no behavior change; packages uploaded before this task still work because the reconciler still does inline `cargo build` until T6 retires it.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migrations `postgres/022_add_build_queue` and `sqlite/019_add_build_queue` add:
  - `compiled_data` (BYTEA / BLOB, NULL)
  - `build_status` (TEXT NOT NULL, default `'pending'`)
  - `build_error` (TEXT NULL)
  - `build_claimed_at` (TIMESTAMP NULL)
  - `compiled_at` (TIMESTAMP NULL)
- [ ] Partial index `idx_pending_builds` on `(build_status, build_claimed_at) WHERE build_status IN ('pending', 'building') AND NOT superseded`.
- [ ] `schema.rs` unified + per-backend `workflow_packages` blocks updated.
- [ ] `UnifiedWorkflowPackage` + `NewUnifiedWorkflowPackage` models pick up the five new fields.
- [ ] `WorkflowPackage` domain type picks up the same fields.
- [ ] DAL additions in `workflow_registry/database.rs`:
  - `claim_next_build()` — atomic `UPDATE ... RETURNING` that transitions one eligible row from `pending` → `building`, sets `build_claimed_at`, returns `(id, registry_id, package_metadata)` or `None`.
  - `mark_build_success(id, compiled_bytes)` — transactional UPDATE setting `build_status='success'`, `compiled_data`, `compiled_at`, clearing `build_error`.
  - `mark_build_failed(id, error_message)` — UPDATE setting `build_status='failed'` + truncated `build_error` (max 64KB).
  - `heartbeat_build(id)` — UPDATE `build_claimed_at = NOW()` if row is still in `building` state.
  - `sweep_stale_builds(stale_threshold: Duration)` — UPDATE batch that resets rows whose `build_claimed_at` is older than threshold back to `pending`.
- [ ] All existing construction sites of `NewUnifiedWorkflowPackage` default the new fields (`compiled_data: None`, `build_status: "pending"`, etc.) so existing insert paths keep working.
- [ ] Unit tests in `workflow_registry/database.rs` for claim, mark_success, mark_failed, heartbeat no-op on wrong-state rows, and sweep behavior.

## Implementation Notes

### Claim query shape

Postgres:
```sql
UPDATE workflow_packages
SET build_status = 'building',
    build_claimed_at = NOW(),
    build_error = NULL
WHERE id = (
    SELECT id
    FROM workflow_packages
    WHERE build_status = 'pending' AND NOT superseded
    ORDER BY created_at
    LIMIT 1
    FOR UPDATE SKIP LOCKED
)
RETURNING id, registry_id, metadata, ...;
```

SQLite: same shape but without `FOR UPDATE SKIP LOCKED` (relies on WAL + busy_timeout).

### Diesel

Use `diesel::update(...).returning(...)` for the claim; Diesel 2.1 supports `RETURNING` on both backends.

### Back-compat for inserts

The `store_package_metadata` and `supersede_and_insert` paths from T-0497 both build `NewUnifiedWorkflowPackage`. Add the five new fields with sensible defaults at those construction sites.

### Test fixtures

Existing `create_test_registry` helper is fine. Add new tests under the same `mod tests` block. No schema wiring for partial index is needed in tests — the migration embeds it.

## Status Updates

*To be added during implementation*
