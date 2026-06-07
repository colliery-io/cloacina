---
id: interservice-communication
level: initiative
title: "Interservice communication substrate — transactional outbox + WS push delivery (implements S-0012)"
short_code: "CLOACI-I-0115"
created_at: 2026-05-27T17:35:03.798102+00:00
updated_at: 2026-05-28T17:13:32.367515+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: interservice-communication
---

# Interservice communication substrate — transactional outbox + WS push delivery (implements S-0012) Initiative

## Context **[REQUIRED]**

This initiative **implements the design specified in [[CLOACI-S-0012]]** and decided in [[CLOACI-A-0006]]: a reusable JIT push-delivery substrate for the Postgres-backed `cloacina-server`, replacing poll-for-changes. It is the **prerequisite** that unblocks the execution-agent fleet ([[CLOACI-I-0114]] is `blocked_by` this initiative).

The substrate is the standard robust composition:

- **Transactional outbox** — durable system of record; rows written in the same transaction as the state change that produced them; delivery state `pending → delivered → acked`.
- **`LISTEN`/`NOTIFY`** — the load-bearing, instant, cross-replica wake (NOTIFY carries only a row-id pointer, never the payload — it is capped at 8KB and is best-effort).
- **WebSocket push** — delivers the payload over a shared versioned envelope; recipient acks.
- **Safety-net sweeper** — backstops missed NOTIFYs / disconnects / crashes, making delivery at-least-once.

Scope is the Postgres server only; the SQLite daemon is single-process with fast-enough in-process IPC and is explicitly untouched (see [[CLOACI-A-0006]] scoping). Design rationale, requirements (REQ/NFR), and open questions live in [[CLOACI-S-0012]] — this initiative does not restate them, it builds them.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- An outbox table + migration, with an enqueue API usable inside an existing transaction (Postgres).
- A NOTIFY-emit-on-commit + per-replica LISTEN loop that wakes the push relay without polling.
- A shared, versioned WS message envelope replacing today's bespoke per-endpoint framing, reusing the existing ticket-on-upgrade auth.
- A push relay + ack protocol + reconnect/resync (redeliver non-`acked` rows on reconnect).
- A safety-net sweeper with outbox-depth / stuck-`pending` metrics.
- **First consumer — CLI execution-event subscription** migrated off REST-polling of `/executions/:id/events`, proving the substrate end-to-end on a low-risk path before the fleet depends on it.
- Live-server contract coverage for the WS protocol consistent with [[CLOACI-I-0113]] / [[feedback_sdk_live_server_drift]].

**Non-Goals:**
- The fleet's work-delivery consumer and agent logic — that's [[CLOACI-I-0114]]; this initiative only provides the channel it will use.
- Migrating the `cloacina-compiler` build queue off DB-poll (allowed later, not required here).
- SQLite-daemon support; external brokers; ordered/exactly-once delivery.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

Full design is in [[CLOACI-S-0012]]. Summary of the build surface:

```
state change ──(same txn)──► outbox row INSERT ──► NOTIFY(channel, row_id)
                                   │                      │
                                   │                 LISTEN loop (per replica)
                                   ▼                      ▼
                              sweeper (backstop)     push relay ──WS push──► recipient ──ack──► row = acked
```

Touch points: a new outbox table + diesel migration; NOTIFY/LISTEN wiring (Postgres connection dedicated to LISTEN); a WS envelope type + relay in `cloacina-server`; `cloacinactl` execution-events command migrated to subscribe; metrics registration alongside the existing `cloacina_*` counters.

## Detailed Design **[REQUIRED]**

Defer to [[CLOACI-S-0012]] for REQ/NFR and open questions (OQ-A outbox granularity, OQ-C resync bound, OQ-D sweep ownership, OQ-E I-0113 crate coupling, OQ-F NOTIFY channel design + 8KB pointer-only constraint). These resolve during design/decomposition; none block creating the task breakdown.

## Alternatives Considered **[REQUIRED]**

Decided in [[CLOACI-A-0006]]: rejected keep-polling (the traffic we're removing), WS-as-the-queue (lossy), LISTEN/NOTIFY-as-durable-bus (at-most-once, non-durable — adopted only as the wake), external broker (infra weight), and forcing a SQLite-portable mechanism (solves a problem that can't exist).

## Implementation Plan **[REQUIRED]**

Candidate task batches (decompose into vertical slices):

1. **Outbox foundation** — table + migration + transactional enqueue API + state machine. Exit: rows enqueue atomically with a producing txn; unit-tested state transitions.
2. **Wake + relay** — NOTIFY-on-commit, per-replica LISTEN loop, in-process channel for same-replica wake, push relay skeleton. Exit: a committed row wakes a relay and is pushed (single + cross-replica).
3. **WS envelope + auth + ack/resync** — shared versioned envelope, ack protocol, reconnect redelivery of non-`acked` rows, ticket-on-upgrade auth reuse. Exit: at-least-once delivery survives a forced disconnect.
4. **Sweeper + observability** — safety-net sweep, outbox-depth / stuck-`pending` metrics, alerting threshold. Exit: a killed relay's in-flight rows redeliver via sweep; metrics scrape clean.
5. **CLI consumer migration (validation)** — `cloacinactl execution events` subscribes over WS instead of polling; live-server contract test. Exit: CLI gets JIT events with no poll loop; contract suite green against a live server.

Sequencing: 1 → 2 → 3 → 4 are the substrate; 5 validates it and is the gate that lets [[CLOACI-I-0114]] start. The fleet does not begin until 5 is green.
