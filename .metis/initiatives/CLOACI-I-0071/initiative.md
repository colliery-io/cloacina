---
id: websocket-integration-reactive
level: initiative
title: "WebSocket Integration & Reactive Scheduler"
short_code: "CLOACI-I-0071"
created_at: 2026-04-04T17:48:55.180684+00:00
updated_at: 2026-04-07T15:45:37.558452+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: websocket-integration-reactive
---

# WebSocket Integration & Reactive Scheduler Initiative

## Context

Second implementation initiative for CLOACI-I-0069. Takes the embedded vertical slice from I-0070 and connects it to the API server. Adds the Reactive Scheduler (the computation graph coordinator), WebSocket endpoints for accumulators and reactors, per-endpoint authorization, and reconciler-based loading.

After this initiative, computation graphs are a server-mode feature: packages uploaded via the reconciler spawn long-lived accumulator and reactor processes, accessible via authenticated WebSocket endpoints.

Blocked by: CLOACI-I-0070 (vertical slice must work first). Also depends on CLOACI-I-0049 (API Server) for the HTTP/WebSocket infrastructure.

Specs: CLOACI-S-0007 (Integration Points), CLOACI-S-0005 (Reactor — health states, manual channel).

## Goals & Non-Goals

**Goals:**
- Implement the Reactive Scheduler — spawns/supervises accumulators + reactors from packages loaded via reconciler
- Add WebSocket layer to the API server for accumulator and reactor endpoints
- Implement per-endpoint authorization (AccumulatorAuthPolicy, ReactorAuthPolicy) using PAK auth
- Implement accumulator WebSocket registration — accumulators register as named endpoints, external producers push via WebSocket
- Implement reactor WebSocket registration — operators interact via WebSocket (force-fire, inject state, get state, pause/resume)
- Implement broadcast for same-name accumulators (fan-out for multiple reactors consuming same source)
- Implement health reporting via REST (`/v1/health/accumulators`, `/v1/health/reactors`)
- All writes to accumulators go through WebSocket (external producers, detectors, nodes)
- Computation graphs loaded via reconciler, Postgres-only (server mode)

**Non-Goals:**
- State accumulator, batch accumulator, polling accumulator (I-0072/I-0073)
- DAL persistence for reactor cache or accumulator checkpoints — deferred to I-0082 (MVP Resilience Wiring)
- Accumulator health state transitions — deferred to I-0082
- Python bindings (I-0073)
- Soak tests (I-0073)

> **Note**: I-0082 (MVP Release — Resilience Wiring) picks up the scheduler wiring, supervision startup, shutdown ordering, and health integration that were explicitly excluded here. WIRE-1 through WIRE-4 and BUG-4 in I-0082 complete I-0071's server integration path.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Reactive Scheduler implemented — spawns accumulator + reactor tasks from package declarations
- [ ] Reactive Scheduler supervises tasks (restart on panic)
- [ ] Reconciler routes computation graph packages to Reactive Scheduler (workflow packages to Unified Scheduler)
- [ ] WebSocket endpoints: `ws://api-server/v1/accumulator/{name}`, `ws://api-server/v1/reactor/{name}`
- [ ] Per-endpoint auth: PAK key must be authorized for specific accumulator/reactor name
- [ ] External producer pushes event via WebSocket → accumulator receives → reactor cache updates → graph fires
- [ ] Same-name accumulator broadcast: two accumulators registered as "alpha", WebSocket push broadcasts to both
- [ ] Reactor manual operations via WebSocket: force-fire, fire-with, get-state, pause, resume
- [ ] Health endpoints: GET `/v1/health/reactors`, `/v1/health/reactors/{name}`
- [ ] Package lifecycle: upload → spawn, update → graceful swap, remove → shutdown
- [ ] Detector writes to accumulator via WebSocket (same path as external producer)
- [ ] Integration test: external WebSocket client → push to accumulator → graph executes → verify output
- [ ] All existing tests continue to pass

## Implementation Plan

1. **WebSocket layer** — add WebSocket upgrade support to the axum API server, route `/v1/accumulator/{name}` and `/v1/reactor/{name}`
2. **Registration registry** — internal registry mapping names → process channels, broadcast for same-name
3. **Per-endpoint auth** — extend PAK auth to WebSocket connections, AccumulatorAuthPolicy and ReactorAuthPolicy
4. **Reactive Scheduler** — lifecycle manager, spawns/supervises tasks, receives packages from reconciler
5. **Reconciler routing** — detect computation graph declarations in packages, route to Reactive Scheduler
6. **Reactor manual channel** — wire WebSocket commands to reactor's manual_rx
7. **Health reporting** — REST endpoints aggregating accumulator and reactor health
8. **Integration tests** — WebSocket client pushing events, package lifecycle, auth verification
