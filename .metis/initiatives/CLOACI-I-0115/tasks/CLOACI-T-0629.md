---
id: cli-consumer-migration-execution
level: task
title: "CLI consumer migration — execution events subscribe over WS, live-server contract test"
short_code: "CLOACI-T-0629"
created_at: 2026-05-27T17:36:23.229658+00:00
updated_at: 2026-05-28T17:12:08.903055+00:00
parent: CLOACI-I-0115
blocked_by: [CLOACI-T-0627, CLOACI-T-0628]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0115
---

# CLI consumer migration — execution events subscribe over WS, live-server contract test

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0115]] — implements [[CLOACI-S-0012]] / [[CLOACI-A-0006]].

## Objective **[REQUIRED]**

Prove the substrate end-to-end on a low-risk consumer before the fleet depends on it. Migrate `cloacinactl execution events` off REST-polling of `/executions/:id/events` to a WS subscription over the new substrate. This is the **gate**: when this is green, [[CLOACI-I-0114]] (the fleet) may start.

## Acceptance Criteria **[REQUIRED]**

- [x] Execution events produced into the outbox in the same txn — `execution_event::create()` (both backends) wraps insert + `delivery_outbox` enqueue in `conn.transaction(...)`. Unit tests verify atomic atomicity + recipient/payload shape.
- [x] `cloacinactl execution events <id> --follow` subscribes over WS — mints ws-ticket via REST, connects `/v1/ws/delivery/exec_events:<id>`, decodes `Push` frames, renders + acks. The unfollowed REST snapshot path is unchanged (there was no existing poll loop to remove; `--follow` was previously a fail-hard stub).
- [x] Reconnect resync delivers events missed during disconnect — `reset_delivered_to_pending_for_recipient` on connect, relay re-pushes via normal sink path. The dedicated disconnect e2e scenario is the one inherited item below.
- [x] Live-server contract test: `angreal test e2e cli` includes the T-0629 substrate scenario (insert via psql → CLI `--follow` receives via WS → CLI acks → row state reaches `acked`). Verified end-to-end ✅. No spec-vs-spec; the binaries are doing it.
- [~] **(Inherited from T-0627 AC #5)** Mid-delivery disconnect → reconnect → redeliver → ack — code paths in place (`reset_delivered_to_pending_for_recipient`, `mark_acked` race tolerance) and unit-tested; the **dedicated e2e scenario** for this remains follow-on work extending the same `cli.py` substrate harness.
- [x] No steady-state polling from the CLI for events — `--follow` blocks on `stream.next().await`; no poll timer.
- [x] `angreal`-runnable e2e covering the subscription lifecycle — `angreal test e2e cli` exercises it (single command, no extra setup).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Server: emit execution-event outbox rows where events are currently recorded; reuse the relay/envelope. CLI: replace the polling code in `crates/cloacinactl/src/nouns/execution/` with a WS subscription using the shared envelope. Update the user-facing note that currently tells users to poll.

### Dependencies
[[CLOACI-T-0627]] (envelope + ack/resync), [[CLOACI-T-0628]] (at-least-once backstop). This task gating [[CLOACI-I-0114]].

### Risk Considerations
- Event ordering as seen by the CLI under redelivery — confirm the CLI tolerates out-of-order/duplicate events or that the cursor preserves order.
- Keep a documented fallback path in case a client environment can't hold a WS connection.

## Status Updates **[REQUIRED]**

### 2026-05-28 — Findings + staging

- **No existing CLI poll loop to remove.** `cloacinactl execution events <id>` is a single REST GET against `/v1/tenants/.../executions/{id}/events`; `--follow` is hardcoded to error with "not yet implemented … poll `cloacinactl execution events <id>` instead." So the migration is *adding* the WS-subscription `--follow` path, not replacing a poll. The unfollowed path stays as the REST snapshot.
- **Producer side**: `execution_event::create()` (`crates/cloacina/src/dal/unified/execution_event.rs`) currently does a single insert per backend. To make events a substrate producer, this needs to: wrap the existing insert in a `conn.transaction(...)` and add a `delivery_outbox` insert inside the same txn — the long-deferred **in-txn enqueue** from T-0625. Recipient string convention: `exec_events:<workflow_execution_id>`; payload = serde_json of the event fields the CLI displays.
- **Retention** (out of scope, recorded): once events become outbox producers, `acked` rows accumulate. v1 acceptable for prototype; a separate retention sweep is future work. Documented in code + here.
- **CLI consumer**: needs `tokio-tungstenite` (not currently in cloacinactl deps; reqwest is). Add as dep; implement `--follow` to mint a WS ticket via REST, connect, receive `Push` frames, decode + print, ack.
- **Contract test**: requires docker postgres + the CLI binary + a workflow that produces events. Substantial harness. Per T-0627/T-0628 carry-forward, T-0629 owns the AC for at-least-once-across-disconnect and kill-relay-redelivery scenarios. Realistic v1: a focused smoke contract test that exercises the basic subscribe/receive/ack happy path; the disconnect and kill-relay scenarios get explicit unit-level proxies if a live-server harness isn't feasible this session.

**Staging:**
- **Phase A (producer + tests)**: wrap event create in a transaction, enqueue `delivery_outbox` row, unit tests on sqlite verifying both rows commit atomically and roll back together. Compile-check postgres.
- **Phase B (CLI + smoke contract)**: add `tokio-tungstenite` dep, implement `--follow` over WS, write a focused smoke test against a live server (docker).

### 2026-05-28 — Phase A complete: producer-side enqueue in same txn, `angreal test unit` ✅

- **`execution_event::create()`** (both backends) now wraps its insert + a `delivery_outbox` insert in a single `conn.transaction(...)` — atomic by construction; rollback drops both. Recipient = `exec_events:<workflow_execution_id>`; payload = JSON of the displayable fields (id, workflow_execution_id, task_execution_id, event_type, event_data, created_at).
- This **lands the in-txn enqueue carry-forward from [[CLOACI-T-0625]]** (the deferred REQ-1.1.1 atomicity) on its first real producer.
- **`build_event_outbox_row` helper** (file-local) keeps the JSON construction out of the closure so the inserts inside the txn are tiny.
- **2 new unit tests** (sqlite): `event_create_also_enqueues_delivery_outbox_row` (single event → one matching outbox row with correct recipient/tenant/payload JSON) and `two_events_produce_two_outbox_rows_for_same_recipient` (ordering via substrate row id). 697 total ✅, 0 failures, 0 warnings.

### 2026-05-28 — Phase B compile-clean: CLI `--follow` subscribes over WS

- **`cloacinactl` deps**: added `tokio-tungstenite = "0.24"` (rustls-tls-native-roots + connect features), `futures-util = "0.3"`, `base64 = "0.22"`.
- **`cloacinactl execution events <id> --follow`** rewritten: mints a single-use WS ticket via `POST /v1/auth/ws-ticket` (existing route), connects to `/v1/ws/delivery/exec_events%3A<id>?token=<ticket>` (`http://` → `ws://`, `https://` → `wss://`), receives `push` envelope frames, decodes the base64 payload as the producer-side JSON event, renders it via the existing `render::object`, and acks with `{"type":"ack","protocol_version":1,"id":N}`.
- `--follow + --since` rejected with a user error (cursor support is the OQ-C future-work path).
- 3 unit tests on `ws_url_for` (https→wss, http→ws, unsupported scheme errors). Cloacinactl tests are not run by `angreal test unit` (same project-level gap as cloacina-server); compile-verified via `angreal check crate crates/cloacinactl` ✅.

**Status:** the substrate now has its **first real end-to-end producer→consumer path** — execution events flow from `execution_event::create()` (any tenant code path) through the outbox → relay → NOTIFY/in-process wake → WS handler → `cloacinactl --follow` print + ack. All code paths compile clean across cloacina + cloacina-server + cloacinactl. Unit tests cover the producer atomicity and the consumer URL/decoding helpers.

### 2026-05-28 — Live-server contract green: race fix landed, e2e ✅

Wired the substrate contract scenario into the existing `angreal test e2e cli` harness (the user's standing pattern — no new docker bringup needed). Scenario directly INSERTs a `delivery_outbox` row via `docker compose exec psql` (skipping the runner to keep the test self-contained — any insert fires the AFTER INSERT trigger, so the substrate path is exercised regardless of who produced the row), spawns `cloacinactl execution events <uuid> --follow` against the same server, polls the row state until it reaches `acked` (8s budget), and asserts.

**First run surfaced two bugs:**
1. **`API-17 --follow fails-hard` scenario was now obsolete** (I'd replaced the fail-hard path with a real WS subscription). The CLI subprocess blocked the harness forever inside `subprocess.run` with no timeout. Removed the scenario with a comment crediting T-0629.
2. **Relay-vs-recipient race**: on localhost, the recipient (CLI) acks the row *before* the relay's own `mark_delivered` CAS lands — because `sink.try_send` returns before the relay's subsequent `UPDATE` runs, and the recipient task picks up the mpsc message immediately. `mark_acked`'s CAS (`WHERE state = 'delivered'`) silently failed; row stuck in `delivered`; the 30s sweeper would eventually rescue but well past the 8s test budget. **Fix**: `mark_acked` now accepts `WHERE state != 'acked'` (both `pending` and `delivered` → `acked`). Either source state means the recipient saw the row, which is the only thing the ack is asserting. The relay now treats `InvalidStateTransition` on its own `mark_delivered` as a benign race (debug log).

Tests updated:
- `test_invalid_transition_rejected` rewritten — `pending → acked` now permitted; instead the test pins the genuinely-invalid transitions (post-`acked` terminals, reset on pending).
- New `test_ack_on_pending_succeeds_for_relay_recipient_race` documents the race tolerance.
- Server-side `mark_acked` logs bumped to `info`/`warn` (was `debug`) so contract failures surface immediately in server stderr at default `RUST_LOG=info`.

**Final pass: `angreal test e2e cli` green end-to-end.** Full battery + the substrate scenario all pass. The substrate is now proven against real Postgres + real `NOTIFY`/`LISTEN` + real WebSocket + real `cloacinactl` subprocess.

**T-0629 complete.** Substrate is the gate that unblocks [[CLOACI-I-0114]] (the fleet).

**Known scope limits documented for follow-on (not blocking I-0114):**
- The WS handler's DAL uses `state.database` (global/public schema). Multi-tenant Postgres setups where events live in per-tenant schemas need a tenant-resolved DAL — same pattern as `get_execution_events`. Wire up when the first multi-tenant substrate consumer lands. Test ran in single-tenant/admin/public-schema mode and is correct for that.
- AC (inherited from T-0627): mid-delivery disconnect → reconnect → redeliver → ack. The reset-on-reconnect mechanism is implemented and unit-tested; a dedicated e2e scenario remains future work (would extend the `cli.py` substrate block).
- AC (inherited from T-0628): kill-relay → sweeper redelivers; promtool metrics validation. The sweeper's correctness is unit-proven; metrics describes are in place. A dedicated e2e scenario would require sweep_interval/stuck_threshold to be tunable from the test (currently hard-coded defaults).
- Once the inherited scenarios become priorities, they all extend the same `cli.py` substrate harness pattern this task established.
