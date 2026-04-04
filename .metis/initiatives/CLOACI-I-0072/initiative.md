---
id: market-maker-reference
level: initiative
title: "Market Maker Reference Implementation"
short_code: "CLOACI-I-0072"
created_at: 2026-04-04T17:48:56.100724+00:00
updated_at: 2026-04-04T17:48:56.100724+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: market-maker-reference
---

# Market Maker Reference Implementation Initiative

## Context

Third implementation initiative for CLOACI-I-0069. The full market maker example that exercises all core capabilities with real-world complexity: multiple accumulator types, routing graph with enum variants and risk checking, state accumulator for cyclic history, DAL persistence, health states, and crash recovery.

This is the MVP proof point. If it works, the computation graph feature is shippable.

Blocked by: CLOACI-I-0071 (WebSocket + Reactive Scheduler must be in place).

## Goals & Non-Goals

**Goals:**
- Implement `#[state_accumulator]` (bounded VecDeque, DAL-backed, capacity semantics)
- Implement DAL persistence for reactor cache snapshots
- Implement DAL persistence for accumulator last-emitted boundary
- Implement accumulator health states (Starting, Connecting, Live, Disconnected, SocketOnly)
- Implement reactor health states (Starting, Warming, Live, Degraded)
- Implement reactor startup health gate (all accumulators healthy before going live)
- Build the market maker reference implementation as a packaged computation graph:
  - 3 accumulators: stream (alpha — orderbook), passthrough (beta — pricing), stream with state (gamma — fills/exposure)
  - State accumulator for previous outputs (capacity=1 for reconciliation)
  - Computation graph with: decision engine → risk check (routing) → output handler / alert handler / audit logger
  - Loaded via reconciler, runs in server mode
- Integration test: full cycle including crash recovery (kill → restart → verify state restored)
- Example runnable as `angreal demos market-maker`

**Non-Goals:**
- Batch accumulator (I-0073)
- Polling accumulator (I-0073)
- Python bindings (I-0073)
- Soak / performance tests (I-0073)
- `when_all` reaction criteria (I-0073)
- `sequential` input strategy (I-0073)

## Acceptance Criteria

- [ ] `#[state_accumulator]` implemented with capacity semantics (1, N, -1, omitted)
- [ ] State accumulator persists VecDeque to DAL, loads on restart, emits to reactor
- [ ] Reactor persists cache snapshot to DAL after each execution
- [ ] Reactor loads cache from DAL on startup
- [ ] Accumulator health states implemented, reported via `watch` channel to reactor
- [ ] Reactor health states implemented (Starting → Warming → Live, Degraded on disconnect)
- [ ] Reactor waits for all accumulators healthy before entering Live state
- [ ] Health reported via `/v1/health/reactors/{name}` with per-accumulator detail
- [ ] Market maker example: 3 source accumulators + 1 state accumulator → reactor → 5-node graph with enum routing
- [ ] Decision engine routes Signal → risk_check, NoAction → audit_logger
- [ ] Risk check routes Approved → output_handler, Blocked → alert_handler
- [ ] State accumulator receives previous output from output_handler via WebSocket, feeds back into next execution
- [ ] Recovery test: kill server, restart, verify reactor cache and accumulator state restored, graph resumes correctly
- [ ] Example packaged and loaded via reconciler
- [ ] All existing tests continue to pass

## Implementation Plan

1. **State accumulator** — `#[state_accumulator]` macro, VecDeque with capacity, DAL persistence
2. **DAL persistence** — reactor cache snapshot table, accumulator checkpoint table, last-emitted-boundary table
3. **Health states** — accumulator health enum + watch channel, reactor health enum + startup gate
4. **Health reporting** — wire health states to API server REST endpoints
5. **Market maker graph** — accumulators, boundary types, routing enums, 5 node functions, topology declaration
6. **Package it** — manifest metadata, reconciler loads it, reactive scheduler spawns it
7. **Recovery test** — integration test: run → crash → restart → verify state
8. **Angreal demo task** — `angreal demos market-maker` command
