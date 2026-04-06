---
id: mvp-release-resilience-wiring-bug
level: initiative
title: "MVP Release — Resilience Wiring, Bug Fixes, and End-to-End Integration"
short_code: "CLOACI-I-0082"
created_at: 2026-04-06T00:57:32.887143+00:00
updated_at: 2026-04-06T10:55:44.539222+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: mvp-release-resilience-wiring-bug
---

# MVP Release — Resilience Wiring, Bug Fixes, and End-to-End Integration Initiative

## Context

CLOACI-I-0081 (Computation Graph Resilience) built all the resilience *mechanisms* — DAL tables, CheckpointHandle, health enums, sequence counters, supervisor failure counting, reactor cache persistence functions. However, a comprehensive review of the `chore/tech-debt-payoff` branch revealed that **almost none of these features are wired end-to-end**. The pieces exist in isolation but the scheduler (the only production code path for creating reactors/accumulators) never connects them.

This initiative addresses the bugs, wiring gaps, and missing integrations required before the computation graph system can ship as an MVP. It is scoped to making what already exists actually work — no new features.

### Relationship to other initiatives
- **CLOACI-I-0081** (Resilience) — built all the mechanisms this initiative wires up. Marked completed but most acceptance criteria aren't met end-to-end. I-0082 is the successor that connects the pieces.
- **CLOACI-I-0071** (WebSocket/Scheduler) — built the ReactiveScheduler and server integration but explicitly excluded resilience wiring. WIRE-1 through WIRE-4 and BUG-4 complete I-0071's scheduler path.
- **CLOACI-I-0073** (Batch/Polling Accumulators) — built the accumulator types but punted "DAL persistence for accumulator checkpoints (later)". PERSIST-1 and PERSIST-2 are that "later."
- **CLOACI-I-0077** (Reactor Depth) — built when_all and sequential strategy. PERSIST-3 and the expected_sources part of WIRE-1 are the durability completion of I-0077's work.
- **CLOACI-I-0051** (Hardening) — workflow-side sibling. WIRE-6 (RecoveryEvent) uses DAL infrastructure from I-0051's domain.
- **CLOACI-I-0079** (Soak Tests) — should be blocked by I-0082; soak-testing without wired persistence/health/supervision has limited value.

### What exists (from I-0081)
- CheckpointHandle with typed save/load via DAL
- Four checkpoint DAL tables (Postgres + SQLite): accumulator_checkpoints, accumulator_boundaries, reactor_state, state_accumulator_buffers
- AccumulatorHealth and ReactorHealth state machine enums
- BoundarySender with sequence numbers
- Supervisor check_and_restart_failed with failure counting and circuit breaker
- Reactor cache persistence (persist_reactor_state)
- State accumulator VecDeque DAL persistence
- EndpointRegistry health tracking infrastructure (register_accumulator_health, list_accumulators_with_health)
- ReactiveScheduler.start_supervision() method
- Graceful shutdown path (scheduler.shutdown_all)

### What's broken or disconnected
See "Detailed Findings" below for the full enumeration.

## Goals & Non-Goals

**Goals:**
- Fix correctness bugs (sequence number race, non-transactional deletes, shutdown ordering)
- Wire DAL, health channels, graph name, and expected sources through the scheduler into reactors
- Extend AccumulatorFactory to accept DAL/health so accumulators get CheckpointHandle
- Start the supervision loop from serve.rs
- Actually apply backoff delays before restart
- Record recovery events in existing RecoveryEvent DAL
- Implement reactor startup gating (Warming → Live) and degraded mode
- Make health endpoints report actual ReactorHealth state machine values
- Add crash-resilient persistence to batch and polling accumulators
- Fix the Accumulator::run() ownership issue so user-defined event loops work
- Validate with integration tests that prove restart recovery works end-to-end

**Non-Goals:**
- New resilience features beyond what I-0081 designed
- Performance optimization of persistence paths
- Distributed coordination or multi-node recovery
- Stream backend offset management

## Detailed Findings

### Bugs (Correctness)

**BUG-1: BoundarySender sequence increment before send** (`accumulator.rs:253`)
`fetch_add` happens before `inner.send()`. If send fails, sequence is ahead of reality. Recovery would see a gap. Must increment after successful send.

**BUG-2: delete_graph_state not transactional** (`checkpoint.rs:849-866`)
Four DELETE statements execute without transaction wrapper. Crash between deletes leaves partial state. Both Postgres and SQLite variants need `conn.transaction()`.

**BUG-3: Accumulator::run() silently dead** (`accumulator.rs:323-329`)
Event loop task never calls `acc.run()` — ownership moved to processor. Any accumulator overriding `run()` has its implementation ignored. StreamBackend works via external feed, but user-defined event loops are broken.

**BUG-4: Graceful shutdown ordering** (`serve.rs:127-134`)
Axum server shuts down first (dropping WebSocket connections), then scheduler shuts down. Accumulators/reactors lose channels before orderly flush. Must reverse: shut down scheduler first, then HTTP server.

### Not Wired (Built but Disconnected)

**WIRE-1: Scheduler never wires DAL/health/graph_name/expected_sources** (`scheduler.rs:186-193`)
`load_graph()` creates Reactor::new() but never calls `.with_dal()`, `.with_graph_name()`, `.with_health()`, or `.with_expected_sources()`. All persistence and health features are dead code via the scheduler path.

**WIRE-2: AccumulatorFactory.spawn() cannot pass CheckpointHandle or health** (`scheduler.rs:68-73`)
Trait signature doesn't accept DAL or health sender. All factory implementations create contexts with `checkpoint: None, health: None`.

**WIRE-3: register_accumulator_health never called**
EndpointRegistry has the method and storage. Scheduler never creates health channels or calls it. Health endpoint returns fallback `Live` for everything.

**WIRE-4: Supervision loop never started**
`start_supervision()` exists but `serve.rs` never calls it. Crashed tasks won't auto-restart.

**WIRE-5: Backoff delay calculated but never applied** (`scheduler.rs:311-313`)
Duration computed and logged but no `sleep(backoff)` before respawn. Immediate restart defeats the purpose.

**WIRE-6: RecoveryEvent never recorded**
Supervisor logs failures via tracing but never persists to RecoveryEvent DAL table.

**WIRE-7: Reactor startup gating not implemented** (`reactor.rs:354`)
Immediately reports Live. Never enters Warming. Doesn't subscribe to accumulator health channels.

**WIRE-8: Degraded mode not implemented**
`ReactorHealth::Degraded` defined but never set. No mechanism to watch accumulator disconnection.

**WIRE-9: Health endpoints use hard-coded status** (`health_reactive.rs:59-64`)
Derives "running"/"paused"/"stopped" from booleans, not from ReactorHealth state machine.

### Incomplete Persistence

**PERSIST-1: Batch accumulator buffer not crash-resilient**
No DAL persistence between flushes. Process crash loses all buffered events.

**PERSIST-2: Polling accumulator doesn't restore poll state**
No CheckpointHandle usage, no init/restore logic. Always starts fresh.

**PERSIST-3: Sequential queue persistence gap** (`reactor.rs:461-484`)
Queue only persisted after being fully drained (always empty). Crash mid-drain loses boundaries.

### Minor

**MINOR-1: `_input_strategy` field naming** (`reactor.rs:228`)
Underscore prefix means "unused" in Rust convention, but field IS used at line 359. Misleading.

**MINOR-2: State accumulator uses hardcoded JSON** (`accumulator.rs:684,728`)
Uses `serde_json` directly while rest of system uses profile-based `types::serialize`. Inconsistent but not a bug since they're separate data paths.

## Implementation Plan

### Phase 1 — Bug Fixes
- BUG-1: Move sequence increment after successful send
- BUG-2: Wrap delete_graph_state in transaction
- BUG-3: Fix Accumulator::run() ownership (split init state or use Arc<Mutex>)
- BUG-4: Reverse shutdown ordering in serve.rs
- MINOR-1: Rename _input_strategy field

### Phase 2 — Scheduler Wiring
- WIRE-1: Pass DAL, graph_name, health, expected_sources to Reactor in load_graph
- WIRE-2: Extend AccumulatorFactory trait with DAL + health parameters
- WIRE-3: Create and register health channels in load_graph
- Update packaging_bridge PassthroughAccumulatorFactory accordingly

### Phase 3 — Supervision & Recovery
- WIRE-4: Call start_supervision() from serve.rs startup
- WIRE-5: Add tokio::time::sleep(backoff) before respawn
- WIRE-6: Record recovery events in RecoveryEvent DAL

### Phase 4 — Health State Machines
- WIRE-7: Implement reactor startup gating (subscribe to accumulator health, Warming → Live)
- WIRE-8: Implement degraded mode (watch for accumulator Disconnected)
- WIRE-9: Expose ReactorHealth through health endpoints instead of hard-coded strings

### Phase 5 — Persistence Completion
- PERSIST-1: Periodic batch buffer snapshots to DAL
- PERSIST-2: Polling accumulator checkpoint save/restore
- PERSIST-3: Persist sequential queue before draining, mark items processed

### Phase 6 — Validation
- Integration tests: restart reactor, verify cache restored from DAL
- Integration tests: kill accumulator, verify individual restart + health transition
- End-to-end: push events, crash process, restart, verify no data loss
