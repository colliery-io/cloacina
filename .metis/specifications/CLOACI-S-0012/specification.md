---
id: interservice-communication
level: specification
title: "Interservice communication substrate — WS push delivery with transactional outbox"
short_code: "CLOACI-S-0012"
created_at: 2026-05-27T14:18:32.546120+00:00
updated_at: 2026-05-27T14:18:32.546120+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Interservice communication substrate — WS push delivery with transactional outbox

## Overview **[REQUIRED]**

This specification defines a reusable substrate for **JIT (just-in-time) push delivery** of work and notifications in the **Postgres-backed `cloacina-server`**, replacing poll-for-changes as the default model. It is the concrete design behind [[CLOACI-A-0006]] and the prerequisite that the execution-agent fleet ([[CLOACI-I-0114]]) builds on.

The substrate composes four parts:

1. A **transactional outbox** — the durable record of payloads to deliver and their delivery state. System of record.
2. **`LISTEN`/`NOTIFY`** — the instant, cross-replica wake that tells the replica holding a connection that an outbox row is ready to push (load-bearing, Postgres-native).
3. A **WebSocket push channel** with a shared, versioned message envelope (today's WS endpoints are bespoke and envelope-less).
4. A **safety-net sweeper** that backstops missed NOTIFYs, disconnects, and crashes — giving at-least-once delivery without steady-state polling.

It serves two initial consumers: the **fleet** (server → agent work delivery, [[CLOACI-I-0114]]) and the **CLI** (server → client execution-event subscription, replacing REST-polling of `/executions/:id/events`).

This is **not** a durability mechanism layered on the socket: durability lives in the outbox table; the socket and `NOTIFY` are only transport/wake, both best-effort.

### Scoping (decisive)

In scope: the **Postgres-backed server** only. The **SQLite embedded daemon** ([[CLOACI-A-0005]]) is explicitly out of scope — it is single-process, coordinates via in-process IPC that is already fast enough, and never deploys a fleet, so it has no cross-process delivery problem and needs none of this. The substrate is inert there. This works because the cross-replica fan-out problem and the SQLite backend are mutually exclusive (fan-out ⇒ multi-replica ⇒ Postgres; SQLite ⇒ single-process), so `LISTEN`/`NOTIFY` is always available wherever the fan-out problem actually exists.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Producer (server-side logic)**: writes an outbox row in the same transaction as the state change it reflects (e.g. "task assigned to agent A", "execution event recorded") and fires a NOTIFY. Does not touch the socket directly.
- **Relay / push loop (per replica)**: woken by an in-process channel (local) or `LISTEN`/`NOTIFY` (cross-replica); reads the owned-and-undelivered outbox rows and pushes them over the recipient's WS connection.
- **Recipient (agent / CLI / future dashboard)**: holds a WS connection, receives pushed payloads, processes idempotently, and acks.
- **Sweeper (per replica or singleton)**: periodically reclaims/redelivers outbox rows stuck past a threshold.

### External Systems
- **Postgres**: hosts the outbox table (source of truth) and provides `LISTEN`/`NOTIFY` (the wake). The substrate assumes Postgres; it does not run on the SQLite daemon.
- **cloacina-server HTTP/WS surface**: transport. Auth reuses the existing API-key + tenant model and the single-use WS ticket-on-upgrade pattern in `routes/ws.rs`. Protocol is documented inside the OpenAPI/WS contract from [[CLOACI-I-0113]].

### Boundaries
- **In scope**: outbox schema + state machine, NOTIFY emit + LISTEN loop, WS envelope + auth + reconnect/resync, push + safety-net sweeper, the two initial consumers (fleet work delivery, CLI event subscription).
- **Out of scope**: the SQLite daemon (in-process IPC, untouched); the fleet's own logic (agent lifecycle, artifact fetch, result reconciliation — those live in [[CLOACI-I-0114]]); migrating the compiler build queue off DB-poll (allowed later, not required); a general external message broker; ordered/exactly-once delivery.

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | An outbox row is written in the **same transaction** as the state change that produces it. | Atomicity: never deliver work that wasn't durably committed, never commit work that won't be delivered. |
| REQ-1.1.2 | Outbox rows carry a delivery state (`pending → delivered → acked`), a recipient key, a payload, and timestamps for sweep thresholds. | Drive the delivery state machine. |
| REQ-1.1.3 | Committing an outbox row fires a `NOTIFY`; the replica holding the recipient's connection is woken (no DB poll on the steady-state path). Within a replica, an in-process channel is the wake. | The JIT/low-traffic goal of [[CLOACI-A-0006]]; load-bearing cross-replica wake. |
| REQ-1.2.1 | WS messages use a shared versioned envelope (`protocol_version`, message type, payload) shared by all consumers. | Replace per-endpoint ad-hoc framing; enable mixed-version negotiation. |
| REQ-1.2.2 | A recipient acks each delivered payload; the relay marks the row `acked` on receipt. | Close the at-least-once loop. |
| REQ-1.2.3 | On reconnect, a recipient resyncs: the server redelivers any non-`acked` rows for that recipient. | No lost delivery across disconnects or missed NOTIFYs. |
| REQ-1.3.1 | A safety-net sweeper redelivers/reclaims rows stuck in `pending`/`delivered` past a configurable threshold. | At-least-once + crash recovery; backstop because `NOTIFY` is best-effort/at-most-once. |
| REQ-1.4.1 | The CLI can subscribe to an execution's event stream over WS instead of polling `/executions/:id/events`. | First low-risk consumer; validates the substrate before the fleet stresses it. |
| REQ-1.4.2 | The substrate exposes a work-delivery channel the fleet uses to push work packets to agents. | Primary consumer; the reason this is a prerequisite for [[CLOACI-I-0114]]. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | Assumes Postgres; not deployed on the SQLite daemon. No SQLite-portability requirement. | Fan-out only exists on multi-replica Postgres; SQLite daemon is single-process and out of scope. |
| NFR-1.1.2 | Delivery is **at-least-once**; all recipients must be idempotent under redelivery. | Outbox + NOTIFY + sweep cannot guarantee exactly-once without recipient cooperation; `NOTIFY` is best-effort. |
| NFR-1.2.1 | Steady-state delivery adds no periodic per-recipient polling traffic; only the slow sweep, NOTIFY wakes, and live pushes traverse the network. | The network-traffic reduction that motivates the change. |
| NFR-1.3.1 | Outbox depth / stuck-`pending` count is observable via metrics; sustained growth means delivery is wedged. | Operability — analogous to the compiler sweep-resets signal. |
| NFR-1.4.1 | All WS endpoints enforce API-key + tenant auth on upgrade; recipients only receive payloads within their authorized tenant scope. | Tenant isolation preserved end-to-end. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Work-delivery transport & durability
- **Context**: Replace poll-for-changes with JIT push, without making the socket or `NOTIFY` the system of record.
- **Constraints**: Push and `NOTIFY` are both best-effort/non-durable; delivery must be crash-safe and at-least-once.
- **Required Capabilities**: durable enqueue-in-transaction, event-driven wake, ack + redelivery, instant cross-replica delivery.
- **ADR**: [[CLOACI-A-0006]] (decided: outbox for durability + `LISTEN`/`NOTIFY` for the wake + WS push, server/Postgres-scoped).

### Decision Area: Cross-replica fan-out (resolved)
- **Context**: A recipient (CLI, agent) connected to replica B needs payloads produced on replica A.
- **Resolution**: `LISTEN`/`NOTIFY` is the cross-replica wake; the durable outbox + sweep is the backstop. The SQLite-portability concern is void because fan-out only occurs on multi-replica Postgres. Connection-ownership remains an optional latency optimization, not a necessity.
- **ADR**: [[CLOACI-A-0006]].

## Decision Log **[CONDITIONAL: Has ADRs]**

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| CLOACI-A-0006 | Interservice work delivery: transactional outbox + WS push over poll-based coordination | draft | Server/Postgres-scoped. Outbox = durable system of record; `LISTEN`/`NOTIFY` = load-bearing cross-replica wake; WS push delivers; sweep backstops; at-least-once + idempotent recipients. SQLite daemon out of scope. |

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Postgres-only; the SQLite daemon is out of scope and keeps its in-process IPC.
- Neither the WS socket nor `LISTEN`/`NOTIFY` is the system of record; durability is the outbox table; the sweep guarantees at-least-once.
- Reuse the existing axum REST + WS stack and the ticket-on-upgrade auth pattern; no new transport, no external broker.

## Open Questions (resolve in discovery)

- **OQ-A**: Outbox granularity — one shared table with a `kind` discriminator, or per-consumer tables (work vs. events)?
- **OQ-C**: Resync/catch-up bound — full replay of non-`acked` rows vs. a watermark/cursor the recipient presents on reconnect.
- **OQ-D**: Sweep ownership in multi-replica — per-replica sweep of owned rows vs. a single elected sweeper, and how rows whose owning replica died get reclaimed.
- **OQ-E**: Relationship to [[CLOACI-I-0113]] — does the WS envelope/DTO physically share the SDK protocol crate, and how do the two efforts sequence?
- **OQ-F**: NOTIFY channel design — one channel per recipient/tenant vs. a single channel with a payload key; payload size limits (NOTIFY is capped at 8KB, so it must carry only a pointer/row-id, never the work itself).
