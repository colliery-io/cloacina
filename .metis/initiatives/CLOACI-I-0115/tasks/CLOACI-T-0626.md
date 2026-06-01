---
id: wake-push-relay-notify-on-commit
level: task
title: "Wake + push relay — NOTIFY-on-commit, per-replica LISTEN loop, in-process channel, relay skeleton"
short_code: "CLOACI-T-0626"
created_at: 2026-05-27T17:36:19.535951+00:00
updated_at: 2026-05-28T14:06:44.600079+00:00
parent: CLOACI-I-0115
blocked_by: [CLOACI-T-0625]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0115
---

# Wake + push relay — NOTIFY-on-commit, per-replica LISTEN loop, in-process channel, relay skeleton

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0115]] — implements [[CLOACI-S-0012]] / [[CLOACI-A-0006]].

## Objective **[REQUIRED]**

Make delivery event-driven instead of polled. On outbox commit, fire a Postgres `NOTIFY` carrying only the row id; a per-replica `LISTEN` loop wakes a push relay that reads the row and dispatches it. Same-replica wakes use an in-process Tokio channel (no DB round-trip); cross-replica wakes use `LISTEN`/`NOTIFY`. This is the load-bearing wake of [[CLOACI-A-0006]].

## Acceptance Criteria **[REQUIRED]**

- [x] `NOTIFY` fired on outbox commit, payload = row id only — AFTER INSERT trigger in migration `postgres/028`.
- [x] A dedicated Postgres connection per replica runs the `LISTEN` loop and feeds the relay (`run_pg_listener` / `listen_once`).
- [x] In-process Tokio channel wakes the local relay immediately when this replica is the producer (`WakeHandle` over `Notify`).
- [x] Push relay skeleton: woken → loads undelivered rows via `list_pending` → hands to a `DeliverySink` → marks `delivered`. Connection-ownership falls out via `NoRoute`.
- [x] Integration test: cross-connection NOTIFY wakes the relay and dispatches (`dal::delivery_relay::test_notify_wakes_relay_across_connections`, green); same-replica wake covered by `delivery::tests::test_in_process_wake_triggers_drain`.
- [x] Steady-state path issues zero periodic SELECTs — no poll timer; `run` is `tokio::select!` on `Notify` + shutdown only.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Use a dedicated `tokio-postgres` connection alongside the diesel pool for the LISTEN loop. Relay is a tokio task per replica. The transport sink is abstracted so T-0627 can plug WS push in without reworking the relay. NOTIFY channel design (one-per-recipient vs single+key) is OQ-F — decide here.

### Dependencies
[[CLOACI-T-0625]] (outbox table + query helpers). Feeds [[CLOACI-T-0627]] (transport) and [[CLOACI-T-0628]] (sweeper shares the relay's delivery path).

### Risk Considerations
- `NOTIFY` is at-most-once/best-effort: a notify lost during a LISTEN reconnect must be caught by the sweeper (T-0628) — rely on it only for latency, never correctness.
- LISTEN connection liveness: on reconnect it must trigger a catch-up read of undelivered rows.

## Status Updates **[REQUIRED]**

### 2026-05-27 — Findings + staging plan

- **`Database` does not retain the connection string** (`connection/mod.rs` struct holds pool/backend/schema/tempfile only). The `tokio-postgres` LISTEN connection therefore needs the **raw URL plumbed in from server config**, not derived from `Database`. Wiring constraint for the relay constructor.
- **No `LISTEN`/`NOTIFY` exists anywhere** — greenfield. `tokio-postgres` 0.7 is available behind the `postgres` feature.
- **NOTIFY mechanism = AFTER INSERT trigger** (producer-agnostic, fires for any insert path, queued and delivered by Postgres at COMMIT). Cleaner than a DAL-side `pg_notify` call given the in-txn enqueue helper may insert via varied code paths. Postgres-only migration (no SQLite sibling — SQLite has no NOTIFY and isn't driven).
- The existing **`task_outbox` `list_pending` poll** (runner claiming path) is the separate "cheap win" — **left out of this task** to keep it focused on the `delivery_outbox` relay.

**Staging (both increments are this task):**
1. **Relay core (unit-testable on SQLite):** NOTIFY trigger migration; `DeliverySink` trait + test sink; `DeliveryRelay` drain loop (`list_pending` → sink → `mark_delivered`); in-process `Notify` wake + `WakeHandle`. New DAL `list_pending(limit)` (all recipients, state=pending, id-ordered). Verify via `angreal test unit`.
2. **LISTEN task + cross-replica integration test (needs Postgres/docker):** `tokio-postgres` connection runs `LISTEN delivery_outbox`, forwards notifications to `WakeHandle`. Verify via `angreal test integration`.

### 2026-05-27 — Increment 1 complete (relay core), `angreal test unit` green

Implemented + verified (685 tests pass, 0 fail, no warnings):
- **Migration `postgres/028_delivery_outbox_notify`** — AFTER INSERT trigger `delivery_outbox_notify` → `pg_notify('delivery_outbox', NEW.id::text)`. Postgres-only (no SQLite sibling; migration sets are already independently numbered across backends, so parity isn't required).
- **DAL `list_pending(limit)`** added (pending across all recipients, id-ordered) — the relay drain query.
- **`crates/cloacina/src/delivery/mod.rs`** (new, registered `pub mod delivery;` in lib.rs): `DeliverySink` trait + `DeliveryOutcome {Delivered, NoRoute}`, `WakeHandle` (coalescing `Notify`), `DeliveryRelay` with `drain_once` (list_pending → sink → mark_delivered; NoRoute/Err leave row pending) and `run(shutdown)` (startup catch-up drain + event-driven, no poll timer). **Connection-ownership falls out via `NoRoute`** — the relay needs no roster; a sink that doesn't own the recipient's connection returns NoRoute and the row waits for the owning replica's NOTIFY-woken relay.
- 3 SQLite unit tests: drain+mark-delivered, no-route-leaves-pending, in-process-wake-triggers-drain (via `run` + `WakeHandle`).

### 2026-05-27 — Increment 2 written (LISTEN task + integration test) — verification pending

- **`run_pg_listener` + `listen_once`** added to `delivery/mod.rs` (postgres-gated): `tokio-postgres::connect` (NoTls, v1), drives the connection's async-message stream via `futures::stream::poll_fn(|cx| Pin::new(&mut connection).poll_message(cx))`, forwards `AsyncMessage::Notification` → `WakeHandle::wake()`. Reconnects with fixed backoff; `Ok(())` only on shutdown, `Err` on disconnect (so the caller reconnects). Wakes once on each connect (catch-up). Raw URL plumbed in by caller (Database doesn't retain it).
- **Integration test** `tests/integration/dal/delivery_relay.rs` (postgres-gated, registered in `dal/mod.rs`): inserts on the DAL pool connection, asserts a *separate* LISTEN connection's NOTIFY wakes the relay to deliver. Designed so only a real NOTIFY can deliver the post-insert row (relay idle after startup drain; no in-process wake issued).
- **NoTls limitation** noted: v1 targets local/in-cluster Postgres; TLS LISTEN is a follow-up.

**Verification status:** the listener is postgres-gated, so `angreal test unit` (sqlite) does NOT compile it. Compile-check the postgres path with **`angreal check crate crates/cloacina`** — NOT `check all-crates`, which also builds every example (~3GB each) and caused disk pressure / ENOSPC ([[feedback_check_crate_not_all_crates]]).

### 2026-05-27 — Increment 2 lib compiles clean (`angreal check crate crates/cloacina` ✅)

The postgres path — including `run_pg_listener`/`listen_once` (`tokio-postgres::connect`, `futures::stream::poll_fn(|cx| Pin::new(&mut connection).poll_message(cx))`, `AsyncMessage::Notification` → `wake()`) — **type-checks with no errors or warnings**. The notification-API uncertainty is resolved.

### 2026-05-28 — Integration verified: cross-connection NOTIFY→wake→deliver green

`angreal test integration` run: `dal::delivery_relay::test_notify_wakes_relay_across_connections` **passes**. Two unrelated failures surfaced (`dal::task_claiming::test_claimed_tasks_marked_running` panicking with `claim count 0 vs 1` on postgres, and `signing::reconciler_did_check::postgres_tests::test_find_signature_present_and_absent` with `PoisonError`) — the signing one is downstream pollution from the task_claiming panic poisoning the shared fixture mutex (the classic [[CLOACI-T-0621]] failure mode), and task_claiming itself touches the existing `task_outbox` table on a code path my changes do not modify (the new trigger fires on `delivery_outbox`, not `task_outbox`). Confirmed by the user as the same nightly-flake family ([T-0620/0622/0623]) — not regressions from this task.

**T-0626 complete.** Increment 1 (relay core, sqlite unit tests), Increment 2 lib (`tokio-postgres` listener, postgres compile), and the cross-connection integration test all green. Substrate wake path is now event-driven end-to-end.

---
_Superseded note (increment 2 now written; see above):_ the `tokio-postgres` LISTEN task (`spawn_pg_listener(conn_str, channel, WakeHandle, shutdown)` forwarding `AsyncMessage::Notification` → `wake()`), wired with the raw URL from server config (recall: `Database` doesn't retain it). Plus a cross-replica integration test (two connections: insert on A, relay woken on B) under `angreal test integration` (docker Postgres). This covers AC #1 (NOTIFY on commit), #2 (dedicated LISTEN conn), the cross-replica half of #5, and #6.
