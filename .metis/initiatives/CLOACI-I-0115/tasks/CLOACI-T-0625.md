---
id: outbox-foundation-table-migration
level: task
title: "Outbox foundation — table, migration, transactional enqueue API, state machine"
short_code: "CLOACI-T-0625"
created_at: 2026-05-27T17:36:17.606298+00:00
updated_at: 2026-05-27T19:34:50.319917+00:00
parent: CLOACI-I-0115
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0115
---

# Outbox foundation — table, migration, transactional enqueue API, state machine

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0115]] — implements [[CLOACI-S-0012]] / [[CLOACI-A-0006]].

## Objective **[REQUIRED]**

Land the durable foundation of the substrate: an outbox table, its diesel migration, and a transactional enqueue API that lets producing code insert an outbox row **inside its own transaction**, with a typed delivery-state machine (`pending → delivered → acked`). This is the system of record; nothing else in the substrate is durable, so this task comes first.

## Acceptance Criteria **[REQUIRED]**

- [x] Outbox table + diesel migration (Postgres + SQLite for schema parity) with columns: id, recipient, kind, tenant_id, payload, delivery_state, delivery_attempts, created_at, delivered_at, acked_at. (OQ-A → single table + `kind` discriminator.)
- [~] `enqueue` API — async standalone form landed + tested; the in-existing-transaction helper (REQ-1.1.1) deferred to the first producer (T-0626/T-0629), mirroring `task_outbox`'s atomic insert in `mark_ready()`.
- [x] Typed state machine with guarded transitions `pending → delivered → acked` (+ `delivered → pending` redelivery); invalid transitions rejected via atomic compare-and-set.
- [x] Query helpers: `list_open_for_recipient`, `list_stuck` (sweeper), `count_open` (metrics).
- [~] Unit tests for every state transition ✅ (incl. invalid-transition rejection); rollback-atomicity test deferred with the in-txn enqueue helper.
- [x] Migration uses CREATE TABLE for a new table (no DROP+CREATE of an existing one) — consistent with [[feedback_sqlite_migration_recreate]].

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
New diesel migration + schema entry; an `outbox` DAL module alongside the existing DAL. Payload stored as bytes/jsonb; NOTIFY (T-0626) will reference rows by id only (8KB NOTIFY cap — OQ-F), so the table is the only place the full payload lives. Postgres-only; no SQLite path (see [[CLOACI-A-0006]] scoping).

### Dependencies
None — this is the unblocked starting point of the substrate. Foundation for T-0626/0627/0628.

### Risk Considerations
- Outbox-row write is now on the hot path of every producing transaction — keep the insert cheap and indexed.
- Granularity decision (OQ-A) affects schema; pick single-table-with-`kind` to avoid premature table sprawl, revisit only if a consumer needs isolation.

## Status Updates **[REQUIRED]**

### 2026-05-27 — Discovery: an outbox already exists (reshapes this task)

Pre-implementation exploration found the codebase **already has** outbox + events machinery, so this is not greenfield:

- **`task_outbox` table** (migration `011_create_execution_events_and_outbox`, DAL `crates/cloacina/src/dal/unified/task_outbox.rs`): a **transient** work-distribution outbox. Row inserted in `mark_ready()` in the **same transaction** as the `task_executions.status` update (transactional-outbox property already present), then **deleted on claim**. Columns are minimal: `id`, `task_execution_id`, `created_at`. It is a competing-consumer "a task is Ready, go claim it" queue — durable truth lives in `task_executions.status`.
- **`execution_events` table**: append-only audit trail (`event_type`, `event_data`, `worker_id`, `sequence_num`). This is what the CLI polls via `/executions/:id/events` (T-0629's target).
- **`LISTEN`/`NOTIFY` is NOT implemented anywhere** — `grep` for `pg_notify`/`LISTEN`/`NOTIFY` finds nothing. The `task_outbox` DAL doc-comment *says* it "enables push notifications (Postgres LISTEN/NOTIFY)" but it is currently **polled** via `list_pending()`. So the poll-vs-push gap [[CLOACI-A-0006]] targets is real and unstarted.

**Implication / fork (needs a call before writing the table):** the existing `task_outbox` is transient, task-only, claim-based, and competing-consumer. The substrate ([[CLOACI-S-0012]]) needs a **durable, ack-tracked, addressed (recipient-targeted), multi-kind** delivery outbox with payloads. These are different shapes and arguably different concerns (scheduler→executor claim queue vs. addressed push-with-ack). Options:
- **(A) New general delivery-outbox table** alongside `task_outbox`; later optionally fold `task_outbox` in. Keeps the claim-queue and the addressed-delivery concerns separate.
- **(B) Generalize `task_outbox`** into the substrate outbox (add state machine / payload / kind / recipient / ack columns), making task-ready one `kind`.

Also a cheap early win surfaced: adding `NOTIFY` to the existing `task_outbox` would kill the scheduler→executor `list_pending` poll independent of the rest of the substrate — candidate to pull into T-0626.

Paused implementation to get the fork decided (see conversation).

### 2026-05-27 — Implemented: new `delivery_outbox` table (fork → Option A)

Decision: **new general table**, leaving `task_outbox` untouched. Implemented across:

- **Migrations**: `postgres/027_create_delivery_outbox`, `sqlite/024_create_delivery_outbox` (up/down). Columns: `id` (BIGSERIAL/AUTOINCREMENT), `recipient`, `kind`, `tenant_id` (nullable), `payload` (BYTEA/BLOB), `delivery_state` (default `pending`), `delivery_attempts` (default 0), `created_at`, `delivered_at?`, `acked_at?`. Two partial indexes (recipient+open for replay; open+age for the sweeper). Added to **both** backends for unified-schema parity even though substrate is Postgres-only at runtime (mirrors how `task_outbox` is declared in both).
- **schema.rs**: `delivery_outbox` table! block added to `unified_schema` (Db* aliases), `postgres_schema` (Bytea/Int8/Int4/Timestamp), `sqlite_schema` (Binary/BigInt/Integer/Text).
- **models/delivery_outbox.rs** (new, registered in `models/mod.rs`): domain `DeliveryOutbox` + `NewDeliveryOutbox` + typed `DeliveryState` enum with `can_transition_to` (pending→delivered→acked, delivered→pending redelivery; acked terminal).
- **dal/unified/models.rs**: `UnifiedDeliveryOutbox` / `NewUnifiedDeliveryOutbox` (payload is `UniversalBinary` — `Vec<u8>` does **not** satisfy `AsExpression<DbBinary>`, confirmed via diagnostic).
- **dal/unified/delivery_outbox.rs** (new, registered + accessor `dal.delivery_outbox()`): `enqueue`, `mark_delivered`/`mark_acked`/`reset_to_pending` (atomic **compare-and-set** UPDATEs filtered on expected state — 0 rows ⇒ `InvalidStateTransition`), `list_open_for_recipient`, `list_stuck` (sweeper), `count_open` (metrics). Per-backend arms via `dispatch_backend!`, mirroring `task_outbox.rs`. 6 sqlite-feature unit tests (lifecycle, attempts increment, redelivery, invalid-transition rejection, recipient isolation+ordering).
- **error.rs**: new `ValidationError::InvalidStateTransition { id, from, to }`.

**Done vs deferred against acceptance criteria:**
- ✅ table + migration; ✅ state machine + guarded transitions (compare-and-set rejects invalid); ✅ query helpers; ✅ transition unit tests; ✅ migration uses CREATE TABLE (new table, not DROP+CREATE of existing).
- ⏳ **In-transaction enqueue + atomicity test (rollback drops row)**: the async `enqueue` opens its own connection. The true "insert in the producing txn" helper + its atomicity test land with the **first producer** (T-0626 work-push / T-0629 events), exactly as `task_outbox`'s atomic insert lives in `mark_ready()`, not in the outbox DAL. Noted so it isn't lost.

**Verified — `angreal test unit` green (exit 0).** Clean compile, no warnings. All 6 `delivery_outbox` DAL tests pass (enqueue→pending, full lifecycle, attempts increment, invalid-transition rejection, redelivery, recipient isolation+ordering). Whole suite: 682 + 45 passed, 0 failed — no regressions. The rust-analyzer "second test attribute" flag was a false positive (real build accepts the `#[cfg]`+`#[tokio::test]` idiom).

**Task complete** for its defined scope. Carry-forward to T-0626/T-0629: wire the in-transaction enqueue helper into the first real producer and add the rollback-atomicity test there.
