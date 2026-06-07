---
id: execution-agent-fleet-db-less
level: initiative
title: "Execution-agent fleet — DB-less remote executor backend for horizontal task execution"
short_code: "CLOACI-I-0114"
created_at: 2026-05-27T14:04:13.232068+00:00
updated_at: 2026-05-28T17:13:33.438996+00:00
parent: CLOACI-V-0001
blocked_by: [CLOACI-I-0115]
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: execution-agent-fleet-db-less
---

# Execution-agent fleet — DB-less remote executor backend for horizontal task execution Initiative

## Context **[REQUIRED]**

Today every task in Cloacina runs in-process. The scheduler marks a task ready, the `Dispatcher` (`crates/cloacina/src/dispatcher/`) routes the `TaskReadyEvent` to a registered `TaskExecutor`, and the only implementation — `ThreadTaskExecutor` (`crates/cloacina/src/executor/thread_task_executor.rs`) — runs it on a local Tokio task pool inside the same process that owns the database. Throughput is therefore bounded by a single host: the box running `cloacina-server` (or an embedded runner) is simultaneously the scheduler, the DB owner, and the compute. There is no way to add execution capacity without scaling that one process vertically.

The architecture was, however, explicitly built for this moment. `TaskExecutor` is a documented pluggable seam — its own rustdoc names "Kubernetes, serverless platforms, or message queues" as anticipated backends — and `DefaultDispatcher` already does glob-pattern routing to *named executor keys* via `RoutingConfig`/`RoutingRule`. `TaskReadyEvent` is a thin identity envelope (`task_execution_id`, `workflow_execution_id`, `task_name`, `attempt`), and `ExecutionResult` is an equally thin outcome envelope. The seam to plug a remote backend into is already load-bearing and in production.

This initiative delivers the **first executor backend beyond threads**: a fleet of remote **execution agents**. A new `FleetExecutor` registers as a dispatcher executor key, tracks a live roster of agents, and pushes work to them; agents are stateless compute processes that fetch the workflow artifact, run the task, and report the result back. This is the foundation for horizontally scaling task execution across many hosts.

### Decisions already taken (this session)

These four directional forks were resolved with the maintainer up front and frame the whole design:

1. **State model: DB-less, server-brokered.** Agents have **no database access**. The server is the sole owner of all persistent state — claiming, context loading, status writes, retries. Agents receive a self-contained work packet and report results over the wire. This is the security/isolation boundary that makes a fleet deployable outside the DB's trust zone.
2. **Work flow: coordinator pushes.** A `FleetExecutor` (a `TaskExecutor` impl) tracks live agents via heartbeat and pushes `TaskReadyEvent`-derived work to a chosen agent. This extends the existing push-based Dispatcher rather than inverting it into a pull/lease queue.
3. **Transport: reuse axum REST + WS, JIT push delivery.** Agents are another consumer of the existing `cloacina-server` HTTP/WebSocket surface — no new transport stack, and it dovetails with the OpenAPI/SDK work in [[CLOACI-I-0113]]. Work is **pushed JIT** to agents rather than polled. Crucially, this rides the **interservice communication substrate** specified in [[CLOACI-S-0012]] / decided in [[CLOACI-A-0006]]: a transactional outbox is the durable system of record, WS push drains it event-driven, delivery is at-least-once, and connection-ownership routing handles multi-replica. The fleet does **not** invent its own one-off WS protocol — it is a consumer of that substrate, which is therefore a **prerequisite** (this initiative is `blocked_by` S-0012).
4. **First scope: multi-agent fleet + capacity-aware routing.** The first milestone is a real fleet — multiple agents, heartbeat-based membership, capacity-aware load balancing — not a single-agent proof of concept.

**Deployment scope: Postgres-backed `cloacina-server` only.** The fleet is a server scaling feature; the SQLite embedded daemon ([[CLOACI-A-0005]]) is single-process, coordinates via in-process IPC that is already fast enough, and never deploys remote agents. This is the same scoping as the substrate it rides ([[CLOACI-A-0006]]) and removes any "does this work on SQLite" concern from the fleet design.

### Consequence of "DB-less": code distribution

Because agents cannot read the workflow registry, they cannot resolve `namespace::task` to runnable code on their own. The server must ship the workflow **artifact** to the agent. Cloacina already has packaged-graph + registry machinery (`demos`: `packaged-graph`, `registry-execution`) and the agent already embeds the `cloacina` runtime to load and run a package. So an agent's execution path is: receive work packet → fetch (or cache) the packaged-graph artifact from the server registry over REST → load it via the runtime → execute the single task with the inlined context → report `ExecutionResult` back. Context that `ThreadTaskExecutor` lazily loads from the DB at execution time must instead be **inlined into the work packet** by the server.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A `FleetExecutor` implementing `TaskExecutor`, registerable under one or more dispatcher executor keys, that pushes work to remote agents and feeds their results back through the existing result/status path unchanged from the scheduler's perspective.
- A standalone **execution-agent binary** (working name `cloacina-agent`) that holds no DB connection, registers with the server, heartbeats, advertises capacity, fetches workflow artifacts, executes a single task per work packet, and reports the outcome.
- An **agent-protocol** layered on the existing axum REST + WS surface: agent registration, heartbeat/capacity, work push (server→agent), result/status report (agent→server), and artifact fetch. Documented as part of the OpenAPI/WS contract alongside [[CLOACI-I-0113]].
- A **work packet** type that fully inlines everything an agent needs to run a task with no DB access: task identity, the dependency context the task consumes, the artifact reference, attempt number, timeout, and tenant scoping.
- A **server-side reconciliation path** that ingests agent-reported results and drives the same status/retry/context-write state machine `ThreadTaskExecutor` drives today — so retries, timeouts, and lost-claim handling behave identically whether a task ran on a thread or an agent.
- **Capacity-aware routing within the fleet**: the `FleetExecutor` load-balances across live agents using their advertised/heartbeated capacity, and reports aggregate fleet capacity up to the dispatcher (`has_capacity`) so the scheduler throttles correctly when the fleet is saturated.
- **Liveness & recovery**: agent heartbeat timeout detection; work assigned to a dead agent is reclaimed and rescheduled without data loss (must reconcile with the existing claim/lease semantics so a task never double-executes a side effect... see open question).
- Tenant isolation preserved end-to-end: an agent only ever receives work and artifacts for the tenant scope the server authorizes.
- `angreal`-driven integration + soak coverage: a real `cloacina-server` plus N real agents, exercising end-to-end execution, agent churn, and saturation.

**Non-Goals:**
- **Autoscaling / provisioning.** The fleet runs against a roster of agents that register themselves; spinning agents up/down (k8s HPA, cloud autoscale) is out of scope. The fleet must *tolerate* agents joining/leaving, but does not *cause* it.
- **Replacing `ThreadTaskExecutor`.** Threads remain the default and the embedded-runner path. The fleet is an additional, opt-in executor key. Both coexist behind the same dispatcher and routing rules.
- **A new transport.** No gRPC, no message broker in this initiative (see Alternatives). Reuse REST + WS.
- **Pull/lease work distribution.** Decided against for v1 (coordinator pushes).
- **Cross-language agents.** The agent is the Rust runtime; running tasks authored only for other runtimes on an agent is out of scope (Python tasks run via the embedded `cloaca` path inside the Rust agent exactly as they do in-process today, no more, no less).
- **Heterogeneous-architecture fleets (v1).** Packaged-graph artifacts are native cdylibs tied to the compiler's target triple (OQ-6). v1 requires agents to run that same triple; mismatches fail closed with a clear error. Target-aware routing / per-target builds are deferred to a follow-on.
- **Scheduling policy changes.** Routing/load-balancing is about *which agent*, not *which task next*; the scheduler is untouched.
- **Backwards-compatibility guarantees for the agent protocol before first tagged release.**
- **Running a fleet on the SQLite embedded daemon.** Fleet is Postgres-server-only; the daemon stays single-process with its existing in-process IPC.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### System Requirements

- REQ-001: A `FleetExecutor` implements `TaskExecutor` (`execute`, `has_capacity`, `metrics`, `name`) and is registerable with `DefaultDispatcher` under a configurable executor key. Routing rules can direct task globs to it exactly like any other executor key.
- REQ-002: An agent registers with the server (identity, advertised max concurrency, supported capabilities), then heartbeats on a configurable interval carrying current available capacity. Missing N consecutive heartbeats marks the agent dead.
- REQ-003: The server pushes a **work packet** to a live agent over the agent protocol. The packet is self-contained: task identity, inlined dependency context, artifact reference (+ fetch credentials/URL), attempt number, per-task timeout, tenant scope.
- REQ-004: An agent with no DB connection can fetch the referenced workflow artifact from the server registry over REST (with caching keyed by artifact digest), load it via the `cloacina` runtime, execute exactly the one task named in the packet, and produce an `ExecutionResult`-equivalent.
- REQ-005: The agent reports the result back over the agent protocol. The server reconciles it through the same status/retry/context-persistence state machine used for thread execution, producing identical observable outcomes (status transitions, retry scheduling, context writes, metrics).
- REQ-006: `FleetExecutor::has_capacity()` reflects aggregate live-agent capacity; the dispatcher's `has_capacity()` therefore correctly throttles task marking when the whole fleet is saturated.
- REQ-007: When an agent dies (heartbeat timeout) or its connection drops mid-task, work assigned to it is detected and rescheduled; the existing claim/lease/attempt semantics prevent duplicate *bookkeeping* (duplicate side-effect execution is bounded by the same at-least-once guarantees threads already provide — see open question OQ-2).
- REQ-008: All agent-facing endpoints enforce auth (API key + tenant) consistent with the rest of the server surface; an agent can only receive work and fetch artifacts within its authorized tenant scope.
- REQ-009: The agent protocol (registration, heartbeat, work push, result report, artifact fetch) is documented in the OpenAPI/WS contract and covered by the live-server contract approach established in [[CLOACI-I-0113]].
- REQ-010: New `angreal` tasks boot a server + N agents and run end-to-end, churn, and saturation scenarios; a soak variant sustains load across the fleet.
- NFR-001: Pushing a task to a warm agent (artifact cached) adds bounded coordination overhead vs in-process dispatch; target a small, measured single-digit-millisecond LAN budget validated by a benchmark (exact number set in discovery).
- NFR-002: No agent ever holds DB credentials or a DB connection; this is an invariant verifiable by the agent binary's dependency graph (no diesel/DAL link).
- NFR-003: Fleet must remain correct under agent churn: agents joining/leaving mid-run cause reschedule, never lost or silently dropped tasks.
- NFR-004: No server-internal types (diesel models, DAL internals) leak across the agent protocol; the work packet and result types live in a protocol/DTO module shareable with the SDK surface.

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

### Use Case 1: Scale execution horizontally
- **Actor**: Platform operator running `cloacina-server`.
- **Scenario**: Execution is CPU-bound and pegging the server host. The operator deploys 4 `cloacina-agent` processes on separate hosts pointed at the server with an API key + tenant. They add a routing rule sending `heavy::*` to the `fleet` executor key. Tasks matching the glob now run across the agents.
- **Expected Outcome**: Throughput for `heavy::*` scales with agent count; the server host's CPU drops; non-routed tasks still run on the local thread executor.

### Use Case 2: Survive an agent dying mid-task
- **Actor**: The fleet, autonomously.
- **Scenario**: An agent crashes while executing a task. Its heartbeat lapses.
- **Expected Outcome**: The server detects the dead agent, reclaims its in-flight work, and the task is rescheduled to another live agent or retried per policy. No task is lost; bookkeeping is consistent.

### Use Case 3: Isolate compute from the database
- **Actor**: Security-conscious operator.
- **Scenario**: Policy forbids the hosts running untrusted workflow code from reaching the database.
- **Expected Outcome**: Agents run on DB-isolated hosts, hold no DB credentials, and interact only with the server's authenticated agent endpoints. Compromise of an agent host does not expose the database.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview

Three pieces sit on top of the existing scheduler→dispatcher→executor flow, none of which change the scheduler:

1. **`FleetExecutor` (server-side `TaskExecutor` impl).** Registered under an executor key (e.g. `"fleet"`). On `execute(event)` it: selects a live agent by capacity, builds a **work packet** (loading + inlining the dependency context the task consumes — the same context `ThreadTaskExecutor` would lazily load), pushes the packet to the agent over WS, and awaits the agent's result report, which it returns as `ExecutionResult`. Tracks the agent roster and aggregate capacity for `has_capacity()`/`metrics()`.

2. **Agent endpoints + reconciliation (server-side).** New axum routes for agent registration, heartbeat/capacity, and result/status report, plus an artifact-fetch route backed by the existing registry. A reconciliation component turns reported results into the same DAL status/retry/context writes the thread path performs — ideally by sharing the result-handling code with `ThreadTaskExecutor` so behavior cannot drift.

3. **`cloacina-agent` (new binary, no DB).** Connects to the server (WS for work push + heartbeat, REST for artifact fetch), advertises capacity, and on each work packet fetches/caches the artifact, loads it via the `cloacina` runtime, runs the single named task with the inlined context, and reports the outcome. A thin local concurrency limiter mirrors `ExecutorConfig::max_concurrent_tasks`.

### Component Diagram (textual)

```
            ┌──────────────────────── cloacina-server ────────────────────────┐
            │                                                                  │
  Scheduler ─(mark_ready)─► Dispatcher ─(route "fleet")─► FleetExecutor        │
            │                                                │  ▲              │
            │                                     work packet│  │ result       │
            │                                                ▼  │              │
            │   registry ◄─artifact fetch (REST)──┐   Agent endpoints (WS+REST)│
            │      ▲                               │       ▲   │               │
            └──────┼───────────────────────────────┼───────┼───┼───────────────┘
                   │ artifact (digest-addressed)    │ heartbeat │ work / result
                   │                                │       │   ▼
              ┌────┴───────────┐   ┌────────────────┴───┐  ┌────┴────────────┐
              │ cloacina-agent │   │   cloacina-agent   │  │ cloacina-agent  │   (no DB)
              │  (runtime)     │   │     (runtime)      │  │   (runtime)     │
              └────────────────┘   └────────────────────┘  └─────────────────┘
```

### Sequence (happy path)

1. Agent → server: `register` (capacity, capabilities) → server adds to roster.
2. Agent ⇄ server: periodic `heartbeat(available_capacity)`.
3. Scheduler marks task ready → Dispatcher routes to `FleetExecutor`.
4. `FleetExecutor` picks a live agent with capacity, loads + inlines the task's dependency context, pushes a **work packet**.
5. Agent fetches artifact (cache miss → REST fetch by digest), loads it, runs the one task with inlined context.
6. Agent → server: `result_report(ExecutionResult)`.
7. Server reconciliation applies status/context/retry writes via the shared result-handling path; `FleetExecutor::execute` returns the `ExecutionResult` to the dispatcher.

### Deployment

Server in the DB trust zone; agents anywhere with network reach to the server's agent endpoints and an API key + tenant. Helm chart gains an optional agent deployment (follow-on; not required for first milestone correctness).

## Detailed Design **[REQUIRED]**

### Work packet (the crux of "DB-less")

The work packet is what makes agents stateless. Everything `ThreadTaskExecutor` reads from the DB at execution time must be present in the packet:

- `task_execution_id`, `workflow_execution_id`, `task_name`, `attempt` (the `TaskReadyEvent` fields).
- **Inlined dependency context** — the merged `Context` the task consumes, materialized by the server from the DB before push (this is the work `ThreadTaskExecutor` does lazily via `DependencyLoader`; the fleet does it eagerly, server-side).
- **Artifact reference** — packaged-graph digest + fetch URL + scoped credential.
- `timeout`, tenant scope, and any execution-scope fields (`ExecutionScope`).

The packet type lives in a protocol DTO module (no diesel), shareable with the SDK surface from [[CLOACI-I-0113]].

### Result reconciliation — share, don't reimplement

The single biggest correctness risk is the agent path diverging from the thread path on status transitions, retry decisions, and context persistence. Mitigation: extract the *result-handling* portion of `ThreadTaskExecutor` (everything after the task closure returns: status write, context persist, retry/timeout classification) into a shared component that both the thread executor and the fleet reconciliation call. The fleet then differs from threads only in *where the closure ran*, not in *how the outcome is recorded*. This sharing is a design requirement, surfaced here so decomposition carves it out as its own task.

### Agent protocol over WS + REST

Built **on the [[CLOACI-S-0012]] substrate**, not beside it: work push is an outbox-backed, at-least-once JIT delivery (agent acks; unacked work is redelivered on reconnect/sweep), and the WS envelope is the shared versioned envelope from the substrate. Agents must be idempotent on work redelivery — consistent with the at-least-once posture the thread executor already has.

- **WS** (long-lived, server→agent push + agent→server heartbeat/result): registration handshake (`protocol_version`, capacity, capabilities), `heartbeat`, `work` (server→agent, outbox-drained), `result` (agent→server). Reuses the existing WS infrastructure patterns (`crates/cloacina-server/src/routes/ws.rs`) and the substrate's drain/ack machinery.
- **REST**: artifact fetch (digest-addressed, cacheable), and a fallback/registration endpoint if a non-WS registration is wanted. Documented in OpenAPI.
- Reconnection: agent re-registers on reconnect; server treats a reconnecting known agent idempotently and reconciles any work it believed was in flight.

### Capacity & routing

`FleetExecutor` holds a roster `HashMap<AgentId, AgentState{ max, in_flight, last_heartbeat }>`. Selection picks the live agent with the most free capacity (simple greedy first; pluggable later). `has_capacity()` is `any agent free`. Aggregate `ExecutorMetrics` sums the roster so existing dashboards keep working.

### Liveness

A sweeper marks agents dead after a heartbeat-timeout (pattern mirrors the compiler-service heartbeat sweeper in [[CLOACI-A-0004]]). Dead-agent in-flight work is reclaimed and re-dispatched. Reconciling this with the existing claim/lease semantics so a task is not double-*recorded* is a named discovery task; double-*execution of side effects* inherits the same at-least-once posture threads already have (OQ-2).

### Reuse of existing seams

- `TaskExecutor` trait — implemented, not modified.
- `DefaultDispatcher` glob routing — used as-is; fleet is just another key.
- Registry / packaged-graph load path — reused for artifact fetch + runtime load.
- WS infra + auth (API key + tenant) — reused.
- Likely shares the protocol DTO crate/module with [[CLOACI-I-0113]] (coordinate so the agent protocol is part of the same OpenAPI/SDK surface).

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

### Integration Testing
- **Strategy**: Boot a real `cloacina-server` + N real `cloacina-agent` processes via `angreal`; run workflows routed to the fleet end-to-end against both sqlite and Postgres backends. Verify identical observable outcomes vs the same workflow on the thread executor (status, context, retries, metrics).
- **Churn**: kill an agent mid-task; assert reschedule + no lost/double-recorded task.
- **Saturation**: drive more ready tasks than fleet capacity; assert `has_capacity()` throttling and no dropped work.

### System / Soak Testing
- New `angreal test soak` variant sustaining load across a multi-agent fleet, watching for roster leaks, stuck in-flight entries, and reconciliation drift over time. (Recall the prior soak surfaced executor-deadlock and routing gaps — [[project_soak_test_gaps]] — so soak is a first-class deliverable, not an afterthought.)

### Contract Testing
- The agent protocol is exercised against a live server per the [[CLOACI-I-0113]] live-server contract discipline ([[feedback_sdk_live_server_drift]]), not spec-vs-spec.

## Alternatives Considered **[REQUIRED]**

- **Shared-DB agents (remote `ThreadTaskExecutor`).** Rejected this session: simplest code reuse, but every agent needs DB credentials + reach, collapsing the isolation boundary that is half the point of a fleet.
- **Pull/lease work queue.** Rejected for v1: more elastic and churn-resilient, but inverts the push-based Dispatcher and would want a broker. Revisit if push-based routing hits a scaling ceiling.
- **gRPC transport.** Rejected for this initiative: strong streaming story, but a whole new transport + proto toolchain when the axum REST+WS surface already exists and is being spec'd in [[CLOACI-I-0113]].
- **Message broker (NATS/Redis).** Rejected: best fit for pull/lease + autoscaling, both of which are non-goals; adds external infra.
- **Reimplementing result handling on the server reconciliation path.** Rejected: guarantees thread/fleet behavioral drift. We extract and share instead.

## Open Questions (resolve in discovery; not blocking decomposition)

- **OQ-1**: Context size — inlining large dependency contexts into every work packet could be heavy. Threshold above which we pass a context *reference* the agent fetches over REST instead of inlining? Measure first.
- **OQ-2**: Exactly-once vs at-least-once side effects on agent death mid-task. What does the existing claim/attempt model actually guarantee today for threads, and do we match it or tighten it? Likely "match threads," but confirm and document.
- **OQ-3**: Artifact distribution at scale — cache eviction policy, digest pinning, cold-start cost of first fetch on a fresh agent.
- **OQ-4**: Protocol-versioning / mixed-version fleets (agent vs server skew). Lockstep like [[CLOACI-I-0113]] SDKs, or a negotiated `protocol_version` in the handshake?
- **OQ-5**: Does the agent protocol DTO module physically share a crate with the SDK types from [[CLOACI-I-0113]], and how do the two initiatives sequence?
- **OQ-6 (architecture/ABI portability — sharp)**: A packaged-graph artifact is a **compiled native cdylib** (`workflow_packages.compiled_data`, built by `cloacina-compiler` for *its* target triple; the reconciler loads the bytes directly, never rebuilds). Shipping it to a DB-less agent and `dlopen`-ing it **assumes the agent's target triple (arch + OS + ABI/glibc) matches the compiler's** — a heterogeneous fleet breaks silently at load time, per task. Options: (A) homogeneous fleet, documented as a v1 constraint; (B) tag artifacts with their build target triple, have agents advertise their triple at registration, and have `FleetExecutor` route only to triple-matching agents (capability-matched routing); (C) compiler builds per-target variants. Intersects the compiler ([[CLOACI-I-0105]]) and the build queue. Affects T-0631 (artifact reference must carry the target triple) and T-0632 (agent must report its triple + fail closed on mismatch). **Resolved 2026-05-27 → Option A (homogeneous v1 + fail-closed):** v1 requires agents to run the compiler's target triple, documented as a constraint/non-goal; but the artifact reference carries its build target triple and agents advertise their triple at registration, so a mismatch **fails closed with a clear error** rather than a cryptic `dlopen` crash. True heterogeneity (target-aware routing / per-target builds) is deferred to a follow-on.

## Implementation Plan **[REQUIRED]**

Phased; each phase is a candidate task batch at decomposition. **Not yet decomposed — pending phase transition and maintainer sign-off.**

0. **(Prerequisite, separate work)** Interservice communication substrate — [[CLOACI-S-0012]] / [[CLOACI-A-0006]]. Outbox + WS push drain + connection-ownership routing, validated first by migrating the CLI execution-event subscription off REST-poll. This initiative is `blocked_by` it.
1. **Result-handling extraction** — carve the post-execution result/status/retry/context-write logic out of `ThreadTaskExecutor` into a shared component; thread path keeps passing unchanged. Exit: thread executor green on the shared component; no behavior change.
2. **Agent protocol + work packet** — define DTOs (no diesel), the WS message set, registration/heartbeat/work/result, artifact-fetch REST route. Document in OpenAPI/WS. Exit: protocol types + endpoints land, contract-test scaffold green against a live server.
3. **`cloacina-agent` binary** — DB-less binary: register, heartbeat, fetch+cache artifact, load runtime, execute one task, report. Exit: a single agent executes a routed task end-to-end against a live server.
4. **`FleetExecutor` + routing + reconciliation** — server-side executor, roster/capacity, greedy selection, push, result reconciliation via the shared component, `has_capacity` aggregation. Exit: multi-agent fleet runs routed workflows with outcomes identical to the thread path.
5. **Liveness, churn, saturation** — heartbeat sweeper, dead-agent reclaim/reschedule, capacity throttling. Exit: churn + saturation integration tests green.
6. **Hardening: soak, docs, ops** — fleet soak variant, Diataxis docs (deploy an agent, route to a fleet, operate it), optional Helm agent deployment. Exit: soak stable; docs published.

Sequencing: phase 1 unblocks everything (shared result handling). Phases 2–3 can overlap once DTOs exist. Phase 4 needs 1–3. Phases 5–6 follow 4. Coordinate phase 2 with [[CLOACI-I-0113]] so the agent protocol lands inside the same OpenAPI/SDK surface rather than beside it.
