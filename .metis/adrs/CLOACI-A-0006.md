---
id: 001-interservice-work-delivery
level: adr
title: "Interservice work delivery: transactional outbox + WS push over poll-based coordination"
number: 1
short_code: "CLOACI-A-0006"
created_at: 2026-05-27T14:18:31.434467+00:00
updated_at: 2026-05-27T14:18:31.434467+00:00
decision_date:
decision_maker: dylan.storey@gmail.com
parent:
archived: false

tags:
  - "#adr"
  - "#phase/draft"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-6: Interservice work delivery: transactional outbox + WS push over poll-based coordination

## Context **[REQUIRED]**

Cloacina's control and observability planes are poll-dominated, and the polling falls into two distinct categories that are easy to conflate:

- **DB-as-durable-bus** — `cloacina-compiler` polls a build-queue table (`poll_interval`), claims rows with a lock, and a sweeper resets stale claims; the runner/scheduler uses the same shape for per-task heartbeats with a sweeper resetting `Ready` on heartbeat expiry. Here the database is *both* the source of truth and the coordination mechanism. The model is robust (atomic claim, crash recovery, zero extra infra) but pays steady polling latency and constant SELECT load.
- **REST-poll for observation** — `cloacinactl` polls `GET /executions/:id/events`; the code comment literally instructs users to "poll `cloacinactl execution events <id>`." This is pure waste: a subscriber repeatedly asks "is there anything new" instead of being told.

WebSockets exist today only as a narrow data plane (`/v1/ws/accumulator/*`, `/v1/ws/reactor/*` for computation-graph event ingestion and reactor commands) — there is no control-plane state-change push and no shared message envelope.

The motivating goal (maintainer): move to a **JIT push model** to eliminate constant "anything to do?" polling and the network traffic it generates — work and notifications should be delivered when they exist, not discovered on a timer. This decision is a **prerequisite** for the execution-agent fleet ([[CLOACI-I-0114]]), whose DB-less agents cannot poll/claim from the database at all and must be *pushed* work.

### Deployment scoping (decisive)

This decision applies **only to the Postgres-backed `cloacina-server`** — the multi-tenant, potentially multi-replica deployment ([[CLOACI-A-0005]]: enterprise server). The server is Postgres in practice: tenants *are* Postgres schemas, tenant provisioning and the runtime connection paths (`get_postgres_connection`) are Postgres-specific, and `default = ["postgres"]`. The fleet and any WS push fabric only earn their keep there.

The **SQLite embedded daemon** is explicitly **out of scope**: it is single-process, coordinates via in-process IPC that is already fast enough, never deploys a fleet, and has no cross-process or cross-replica delivery problem. None of the machinery below applies to it.

This scoping dissolves what first looked like the hard constraint. The earlier framing worried that "SQLite ships too, so we can't rely on `LISTEN`/`NOTIFY`." But:

> **The cross-replica fan-out problem and the SQLite backend are mutually exclusive.** A fan-out problem exists only with multiple server replicas; multiple replicas only happen on Postgres; SQLite is inherently single-process. So **no deployment simultaneously needs cross-replica fan-out and lacks `LISTEN`/`NOTIFY`.**

Two constraints remain real:

1. **Push transport is not a durability mechanism.** A dropped WS connection, or a `NOTIFY` emitted while a listener is disconnected, must never mean lost work. The socket — and `LISTEN`/`NOTIFY` — cannot be the system of record.
2. **At-least-once, not exactly-once.** Recipients must be idempotent under redelivery.

## Decision **[REQUIRED]**

For the Postgres-backed server, adopt a **transactional outbox for durability + `LISTEN`/`NOTIFY` for the cross-replica wake + WebSocket push for delivery**, replacing poll-for-changes as the default model:

1. **Transactional outbox table** is the durable record of work/notifications to deliver and their delivery state (`pending → delivered → acked`). The outbox row is written **in the same transaction** as the state change that produced it. This is the system of record. `LISTEN`/`NOTIFY` is **not** durable and never holds this role.
2. **`LISTEN`/`NOTIFY` is the instant, cross-replica wake.** Committing an outbox row fires a NOTIFY; the replica holding the relevant WS connection is woken (rather than polling for the row) and pushes the payload. This is **load-bearing** — it is how a replica learns of work it must deliver, including work produced by a different replica. (Within a single replica, an in-process Tokio channel is the wake; NOTIFY covers the cross-replica case.)
3. **WS push delivers** the payload to the recipient (agent / CLI / dashboard) over a shared versioned envelope. The recipient acks; the outbox row moves to `acked`.
4. **A safety-net sweep** of the outbox (seconds-to-minutes cadence) redelivers rows stuck in `pending`/`delivered` past a threshold — the backstop for a missed NOTIFY (at-most-once), a recipient disconnect, or a replica crash between commit and ack. This is what makes the whole path **at-least-once** and crash-safe.

Recipients consume **at-least-once** and must be idempotent — matching the posture `ThreadTaskExecutor` already has via attempt/claim semantics.

Connection-ownership (a replica preferentially delivering to recipients whose socket it owns) is retained as an **optional latency optimization**, not a structural necessity — `LISTEN`/`NOTIFY` provides the real cross-replica primitive, so the design no longer has to contort routing to avoid cross-replica hops.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Outbox (durability) + LISTEN/NOTIFY (wake) + WS push (chosen)** | Durable + JIT + instant cross-replica; no new infra (Postgres only); at-least-once | Outbox + relay + sweep to build; recipients must be idempotent; NOTIFY is best-effort so sweep is mandatory | Medium | Medium (one substrate, reused by fleet + CLI) |
| **Keep DB-poll everywhere** | Already works; robust claim | The polling traffic/latency we're explicitly removing; no real-time observation | Low | Zero (status quo) |
| **WS as the queue (push without outbox)** | Lowest latency; least machinery | Dropped connection / missed NOTIFY = lost work; no crash recovery | High | Low up front, high in bugs |
| **LISTEN/NOTIFY as the *durable bus*** | Native push on Postgres | At-most-once + not durable + payload limits — cannot be the system of record. (Adopted only as the *wake*, never as the bus.) | High (if relied on for durability) | Medium |
| **External broker (NATS/Redis)** | Purpose-built delivery + fan-out | New external infra dependency; explicit non-goal; operational weight | Medium | High |
| **Force SQLite-portable mechanism (no NOTIFY)** | Single code path | Solves a problem that can't exist (fan-out implies multi-replica implies Postgres); needlessly forgoes NOTIFY | Low | Wasted effort |

## Rationale **[REQUIRED]**

Scoping the decision to the Postgres server is what makes it clean. Once SQLite is correctly excluded (single-process, in-process IPC already fast enough, no fan-out problem), `LISTEN`/`NOTIFY` is universally available wherever the fan-out problem actually occurs, so we use it as the cross-replica wake instead of contorting routing or polling for it.

But `NOTIFY` is best-effort and non-durable, so it cannot be the system of record — the **outbox** remains the durable foundation and the **sweep** remains the at-least-once backstop. The result is the standard robust composition: durable enqueue-in-transaction, instant event-driven wake, push delivery with ack, and a slow backstop. It serves the JIT/low-traffic goal (no steady per-recipient polling) without ever treating the socket or NOTIFY as durable state.

## Consequences **[REQUIRED]**

### Positive
- Eliminates steady-state poll-for-changes traffic; delivery is event-driven and JIT.
- Durable and crash-safe: at-least-once survives dropped connections, missed NOTIFYs, and replica crashes via outbox + sweep.
- Instant cross-replica delivery via `LISTEN`/`NOTIFY` — no new infrastructure, no routing contortions.
- One reusable substrate consumed by the fleet ([[CLOACI-I-0114]]) and the CLI event subscription; the bespoke per-endpoint WS pattern goes away.
- Scope is sharp: server/Postgres only; the SQLite daemon is untouched and stays simple.

### Negative
- New machinery: outbox table + migration, NOTIFY emit + LISTEN loop, relay/push, ack protocol, safety-net sweeper.
- Recipients must be **idempotent** under at-least-once redelivery — a contract every consumer honors.
- New operational signal: outbox depth / stuck-`pending` rows (a backlog means delivery is wedged).
- A second Postgres connection role (a LISTEN-dedicated connection) per replica.

### Neutral
- The compiler build queue's DB-claim model can remain as-is; it may later adopt outbox-drain to drop its poll loop, but this ADR does not force that.
- The SQLite daemon keeps its in-process IPC coordination unchanged; this decision deliberately does not apply to it.
- Connection-ownership remains available as a latency optimization without reopening this decision.

## Review Schedule **[CONDITIONAL: Temporary Decision]**

### Review Triggers
- Outbox-drain latency or sweep load becomes a bottleneck under fleet-scale throughput.
- A future requirement needs ordered or exactly-once delivery beyond at-least-once + idempotency.
- A deployment shape emerges that needs this fabric on SQLite (would contradict the scoping premise — revisit if so).
