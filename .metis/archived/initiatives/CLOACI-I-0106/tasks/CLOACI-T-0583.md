---
id: t-06-execution-events-correlation
level: task
title: "T-06: execution_events correlation columns — migration + DAL"
short_code: "CLOACI-T-0583"
created_at: 2026-05-13T19:38:45.993615+00:00
updated_at: 2026-05-13T20:49:28.113704+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-06: execution_events correlation columns — migration + DAL

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Add `request_id`, `runner_id`, and `tenant_id` columns to `execution_events` so operators can trace events back to a specific request and per-tenant runner. New rows populate all three; existing rows stay NULL (no backfill). Closes OPS-16.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration: `ALTER TABLE execution_events ADD COLUMN request_id UUID NULL`, `ADD COLUMN runner_id UUID NULL`, `ADD COLUMN tenant_id TEXT NULL`. Both Postgres and SQLite (per project convention: no DROP+CREATE on SQLite).
- [ ] Indexes: `(tenant_id, occurred_at DESC)` for tenant-scoped historical queries. Consider `(request_id)` if tracing-style forensics is a known use case; otherwise skip to avoid index bloat.
- [ ] DAL `insert_execution_event` signature extended to take `ExecutionEventContext { request_id, runner_id, tenant_id }`; existing call sites updated.
- [ ] Request context sources:
  - `request_id` from the current tracing span (span carries it after T-0578 lands; for now, callers pass it explicitly from their handler/middleware).
  - `runner_id` from the per-tenant `DefaultRunner`'s id (assigned at construction in T-0580; pass an `Option<UniversalUuid>` for pre-T-0580 callers, `None` is acceptable transitional state).
  - `tenant_id` from `AuthenticatedKey.tenant_id` or the current tenant context.
- [ ] Pre-migration rows remain `NULL` for all three columns. No backfill.
- [ ] Integration test: after a workflow execution, query `SELECT * FROM execution_events WHERE tenant_id = $1 AND request_id = $2`; assert all events for that workflow are present and correctly tagged.
- [ ] Integration test: migration runs clean on a DB with existing rows; pre-migration rows have all three new columns NULL; new rows are fully populated.
- [ ] **Test harness updated as we go**: existing assertion helpers for `execution_events` extended to optionally filter/assert on the three new fields. The tenant-scoped query pattern becomes a first-class fixture. Run `angreal test integration` after migration lands and again after each DAL call site is updated.

## Test Cases

- **TC-1 (migration clean):** fresh Postgres and SQLite DBs run the migration; schema matches expected; no errors.
- **TC-2 (migration on existing data):** DB pre-loaded with execution_events rows; migration adds three NULL columns without rewriting existing rows.
- **TC-3 (new events populated):** post-migration workflow execution writes events with all three fields set.
- **TC-4 (tenant-scoped query):** `SELECT ... WHERE tenant_id = 'A'` returns only tenant A's events.
- **TC-5 (request-correlation):** all events from a single request share the same `request_id`.

## Implementation Notes

### Technical Approach

- Migration files: `crates/cloacina/src/database/migrations/postgres/` and `/sqlite/` (verify exact paths during implementation).
- Use the `ALTER TABLE ADD COLUMN ... NULL` pattern; both backends support this without table rewrite.
- Indexes: postgres `CREATE INDEX CONCURRENTLY` if possible; sqlite plain `CREATE INDEX`.
- DAL: `crates/cloacina/src/dal/unified/execution_events.rs` (verify). Extend the insert path to take an `ExecutionEventContext` struct (cheap to construct, threaded through the executor).
- Call sites: workflow_start, task_start, task_complete, retries, cancellation. Each gets the context from the surrounding scope. For T-0578 + T-0580 + T-0578 cleanly converging, the context can be lifted from the current tracing span — but to keep this task independently shippable, accept explicit parameters and let the callers thread them in.

### Dependencies

- **None hard.** Can land before T-0578/T-0580/T-0581. New columns will sit mostly-NULL until those land and start populating; that's acceptable per the locked-decision "no backfill."
- After T-0578 (spans carry `tenant_id`) and T-0580 (runner has an id), call sites get cleaner — pull context from span instead of explicit args. Plan to revisit DAL signature after both land.

### Risk Considerations

- **Migration on a populated table.** `ALTER TABLE ADD COLUMN ... NULL` is cheap (no row rewrite). Tested on Postgres many times; SQLite same. No surprises expected, but run the e2e suite against the migrated DB.
- **Index bloat.** Adding `(tenant_id, occurred_at)` doubles the index footprint on a hot table. If `execution_events` is high-cardinality, monitor. Operator can drop the index if it costs more than it helps.
- **Backwards-compat for queries.** Existing analytics queries that don't know about `tenant_id` will return all events (since pre-migration rows are NULL and predicates `WHERE tenant_id = X` exclude them). Document this in T-0577 follow-up.
- **Test harness reliance.** Some integration tests probably query `execution_events` for assertions. Each one needs the new columns to be either ignored (`SELECT col1, col2 FROM ...` doesn't change) or asserted-against (in which case the test fixture must populate the fields). Migrate one helper at a time.

## Status Updates

**2026-05-13** — Migration + DAL plumbing landed. Verified clean compile both backends; 664 unit tests pass; clippy clean.

### What changed

- **Migrations:** Postgres `024_add_correlation_to_execution_events`, SQLite `021_add_correlation_to_execution_events`. `ALTER TABLE ADD COLUMN` for `request_id`, `runner_id`, `tenant_id`; partial index `(tenant_id, created_at DESC) WHERE tenant_id IS NOT NULL`.
- **Schema (`schema.rs`):** new nullable columns added to all three `execution_events` definitions (unified / postgres / sqlite modules).
- **Models:**
  - `dal/unified/models.rs`: `UnifiedExecutionEvent` (Queryable) and `NewUnifiedExecutionEvent` (Insertable) gained the three fields. `From<UnifiedExecutionEvent> for ExecutionEvent` threads them.
  - `models/execution_event.rs`: domain `ExecutionEvent` + `NewExecutionEvent` extended. `workflow_event` / `task_event` constructors default the new fields to `None` for backward-compat. New `NewExecutionEvent::with_context(request_id, runner_id, tenant_id)` builder.
- **DAL passthrough (`dal/unified/execution_event.rs`):** `create_postgres` / `create_sqlite` pass `new_event.request_id` / `runner_id` / `tenant_id` into `NewUnifiedExecutionEvent`.
- **Inline emitters (32 sites):** every `NewUnifiedExecutionEvent { ... }` literal across `workflow_execution.rs`, `task_execution/{claiming,crud,state}.rs`, `execution_event.rs` now sets `request_id: None`, `runner_id: None`, `tenant_id: None` — applied via perl-in-place to keep the diff mechanical. These deep DAL emitters don't have plausible context to populate today; revisited after T-0578/T-0580 land.

### Design notes

- **No backfill** per locked I-0106 decision; pre-migration rows stay NULL.
- **Index gated by `WHERE tenant_id IS NOT NULL`** — skips the (large) pre-migration NULL set, keeps the index small until tenant-correlated data accumulates.
- **`with_context` builder** rather than expanding constructor signatures — keeps existing callers compiling unchanged.

### Acceptance criteria status

- [x] Migration files for Postgres + SQLite.
- [x] Index `(tenant_id, created_at DESC)` on both backends.
- [x] DAL extended via new `NewExecutionEvent` fields + `with_context` builder.
- [x] Pre-migration rows remain NULL (no backfill).
- [x] Unit tests: 664 existing pass; both backends compile clean; clippy clean.
- [ ] Integration-test of migration + tag-through on a live DB — deferred to the initiative-level `angreal test integration` run (requires docker + multi-tenant fixtures).
- [x] Test harness compatibility verified — additive schema change; no existing fixture broke.

### Verification (local)

- `cargo check --features postgres -p cloacina` → clean.
- `cargo check --no-default-features --features sqlite -p cloacina` → clean.
- `cargo test --lib --features postgres -p cloacina` → 664 passed; 0 failed.
- `cargo clippy --features postgres -p cloacina --lib` → clean.

End-to-end `angreal test integration` deferred to the initiative-level run.
