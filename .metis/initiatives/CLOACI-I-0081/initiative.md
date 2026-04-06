---
id: computation-graph-resilience
level: initiative
title: "Computation Graph Resilience — Checkpointing, Health States, and Recovery"
short_code: "CLOACI-I-0081"
created_at: 2026-04-05T21:19:53.627753+00:00
updated_at: 2026-04-05T21:35:53.439530+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: computation-graph-resilience
---

# Computation Graph Resilience — Checkpointing, Health States, and Recovery Initiative

## Context

All computation graph components (accumulators, reactors, scheduler, registry) currently operate as pure in-memory, fire-and-forget systems. On any restart — crash, deployment, or supervisor recovery — every component starts from zero: empty caches, empty buffers, no dirty flags, no checkpoint state. This contrasts sharply with the workflow/daemon side, which has mature database-backed recovery (orphaned task detection, recovery events, stale claim sweeping, cron recovery).

The specifications (CLOACI-S-0004, CLOACI-S-0005) describe a complete persistence and health model:
- **S-0004** specifies `CheckpointHandle` for accumulator state, health state machines (Starting/Connecting/Live/Disconnected/SocketOnly), boundary persistence to DAL, and per-class recovery strategies
- **S-0005** specifies reactor cache persistence to DAL, dirty flag persistence, health state machine (Starting/Warming/Live/Degraded), startup gating on accumulator health, and degraded-mode operation

None of this is implemented. The `AccumulatorError::Checkpoint` variant exists as dead code. Health endpoints hard-code `"running"`. The supervisor does cold restarts only (no state restoration). Individual accumulator restart is explicitly punted in the code.

### What exists on the workflow/daemon side (patterns to build on)
- `RecoveryManager` in `task_scheduler/recovery.rs` — orphan detection, retry limits, recovery event logging
- `CronRecoveryService` — configurable thresholds, recovery context injection, audit trail
- `RecoveryEvent` DAL — database table + CRUD already exists for both Postgres and SQLite
- Daemon graceful shutdown — SIGINT/SIGTERM/SIGHUP handling, configurable timeout, three-way race
- Reconciler database-driven reload — already reloads all packages from DB on startup

## Goals & Non-Goals

**Goals:**
- All computation graph components survive process restarts with minimal data loss
- Accumulators checkpoint their state via `CheckpointHandle` (as specified in S-0004)
- Reactor persists `InputCache` and `DirtyFlags` to DAL after each execution (as specified in S-0005)
- Last-emitted boundaries persisted per accumulator so reactor can self-seed from DAL on restart
- State accumulator `VecDeque` persisted to DAL (as specified in S-0004)
- Health state machines for both accumulators and reactors replace hard-coded status
- Reactor startup gating — waits for all accumulators healthy before entering Live
- Degraded mode — reactor continues with stale data when an accumulator disconnects
- Graceful shutdown wired end-to-end (server → scheduler → components → flush/persist)
- Supervisor improvements: individual accumulator restart, failure counting with backoff
- Recovery events recorded using existing `RecoveryEvent` DAL
- Emission tracking via sequence numbers or watermarks on boundaries

**Non-Goals:**
- Stream backend offset management (Kafka consumer group offsets etc.) — separate concern
- Version migration between package upgrades — out of scope
- Performance optimization of the persistence layer — correctness first
- Distributed consensus or multi-node coordination

## Detailed Design

### 1. DAL Schema & CheckpointHandle

New database tables (Postgres + SQLite via Diesel MultiConnection):

```
accumulator_checkpoints:
  graph_name TEXT NOT NULL,
  accumulator_name TEXT NOT NULL,
  checkpoint_data BYTEA NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (graph_name, accumulator_name)

accumulator_boundaries:
  graph_name TEXT NOT NULL,
  accumulator_name TEXT NOT NULL,
  boundary_data BYTEA NOT NULL,
  sequence_number BIGINT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (graph_name, accumulator_name)

reactor_state:
  graph_name TEXT PRIMARY KEY,
  cache_data BYTEA NOT NULL,    -- serialized InputCache
  dirty_flags BYTEA NOT NULL,   -- serialized DirtyFlags
  sequential_queue BYTEA,       -- serialized VecDeque (nullable, only for Sequential strategy)
  updated_at TIMESTAMPTZ NOT NULL

state_accumulator_buffers:
  graph_name TEXT NOT NULL,
  accumulator_name TEXT NOT NULL,
  buffer_data BYTEA NOT NULL,   -- serialized VecDeque<T>
  capacity INTEGER NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (graph_name, accumulator_name)
```

`CheckpointHandle` implementation (as described in S-0004):

```rust
struct CheckpointHandle {
    dal: Arc<dyn CheckpointDal>,
    graph_name: String,
    accumulator_name: String,
}

impl CheckpointHandle {
    async fn save<T: Serialize>(&self, state: &T) -> Result<()>;
    async fn load<T: DeserializeOwned>(&self) -> Result<Option<T>>;
}
```

Wire `CheckpointHandle` into `AccumulatorContext` (currently missing from the struct).

### 2. Accumulator Checkpoint Wiring

Each accumulator class gets persistence appropriate to its shape:

- **Passthrough**: No state to checkpoint. Persist last-emitted boundary only.
- **Stream**: Persist last-emitted boundary. Offset management delegated to `StreamBackend::commit()`.
- **Polling**: Persist poll state via `CheckpointHandle`. Persist last-emitted boundary.
- **Batch**: Persist buffer to DAL periodically (not just on graceful shutdown). Persist last-emitted boundary.
- **State**: Persist `VecDeque<T>` to DAL on every write (append + evict + persist). Load from DAL on startup and emit to reactor.

The accumulator runtime calls `init()` which can now load from `CheckpointHandle`. The `AccumulatorError::Checkpoint` variant (currently dead code) becomes live.

### 3. Reactor Cache Persistence

Per S-0005:
- After each graph execution: persist `InputCache` + `DirtyFlags` snapshot
- Periodically during idle: if cache updated but criteria not yet met, persist
- On startup: load cache from DAL → has last known state immediately
- Sequential strategy: persist `VecDeque<(SourceName, Vec<u8>)>` alongside cache

The reactor's DAL handle is provided at construction via the scheduler.

### 4. Health State Machines

**AccumulatorHealth** (S-0004):
```
Starting → Connecting → Live → Disconnected (retrying)
                    ↘ SocketOnly (passthrough, no source)
```
Reported via `watch::channel` to reactor and via registration updates to API server.

**ReactorHealth** (S-0005):
```
Starting → Warming → Live → Degraded (accumulator disconnected)
```

Replace hard-coded `"status": "running"` in `health_reactive.rs` with actual state machine values.

### 5. Reactor Startup Gating

Per S-0005 startup sequence:
1. Load cache from DAL (instant)
2. Spawn accumulators
3. Accumulators restore from checkpoints, connect to sources
4. Each accumulator signals healthy via `watch` channel
5. Reactor waits for all accumulators healthy
6. All healthy → Live state, start evaluating reaction criteria

### 6. Graceful Shutdown

- Wire `ReactiveScheduler::shutdown_all()` into server shutdown path (currently missing)
- WebSocket connection draining — send close frames before dropping
- Ensure batch accumulators flush buffers on orderly shutdown (already works)
- Ensure reactor persists final cache state before stopping
- Ensure all accumulators persist final checkpoint before stopping

### 7. Supervisor Hardening

- **Individual accumulator restart**: Currently punted ("complex because we need to re-wire the boundary channel"). Implement channel re-wiring so a single crashed accumulator can restart without tearing down the entire graph.
- **Failure counting**: Track consecutive failures per component. Exponential backoff on restarts.
- **Circuit breaking**: After N consecutive failures, stop restarting and report permanently failed.
- **Recovery events**: Record all failures and restarts in the existing `RecoveryEvent` DAL table.

### 8. Emission Tracking

Add metadata to boundaries flowing through the system:
- Sequence number per accumulator (monotonically increasing, persisted)
- Enables deduplication at the reactor (skip boundaries already processed)
- Enables ordering guarantees for Sequential strategy
- Wire format: `(SourceName, Vec<u8>)` becomes `(SourceName, Vec<u8>, u64)` or wrapper struct

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `CheckpointHandle` implemented and wired into `AccumulatorContext`
- [ ] DAL tables created for checkpoints, boundaries, reactor state, state accumulator buffers
- [ ] Batch accumulator buffer survives crash (not just graceful shutdown)
- [ ] Polling accumulator restores poll state from checkpoint on restart
- [ ] State accumulator loads `VecDeque` from DAL on startup, persists on every write
- [ ] Reactor loads `InputCache` + `DirtyFlags` from DAL on startup
- [ ] Reactor persists cache after each execution
- [ ] Sequential `VecDeque` persisted and restored
- [ ] `AccumulatorHealth` state machine with `watch` channel reporting
- [ ] `ReactorHealth` state machine with startup gating
- [ ] Health endpoints report actual state (not hard-coded "running")
- [ ] Degraded mode: reactor continues with stale data when accumulator disconnects
- [ ] `ReactiveScheduler::shutdown_all()` called in server shutdown path
- [ ] Individual accumulator restart without full-graph teardown
- [ ] Failure counting with exponential backoff on supervisor restarts
- [ ] Recovery events recorded in existing `RecoveryEvent` DAL
- [ ] Boundaries carry sequence numbers for deduplication
- [ ] Last-emitted boundary persisted per accumulator
- [ ] Integration tests: restart reactor, verify cache restored from DAL
- [ ] Integration tests: kill accumulator, verify individual restart
- [ ] Soak test: verify no state loss across supervisor-triggered restarts

## Alternatives Considered

**In-memory snapshots only (no DAL)**: Rejected — process restarts lose everything. The reconciler already reloads packages from DB, so persistence infrastructure exists. The DAL is the natural persistence layer.

**Event sourcing / WAL**: Rejected for now — adds significant complexity. The checkpoint model (periodic snapshots + last-boundary persistence) is simpler and sufficient for the target workloads. Can be added later if replay guarantees are needed.

**Reactor coordinates accumulator checkpoints**: Rejected per S-0004 — "accumulators manage their own state independently." Each accumulator checkpoints at its own pace. The reactor only persists its own cache.

## Implementation Plan

**Phase 1 — Foundation (DAL + CheckpointHandle)**
- DAL schema migrations
- `CheckpointHandle` implementation
- Wire into `AccumulatorContext`
- Basic save/load round-trip tests

**Phase 2 — Accumulator Persistence**
- Passthrough: last-emitted boundary persistence
- Polling: checkpoint wiring in runtime
- Batch: buffer persistence (periodic, not just graceful shutdown)
- State: `VecDeque` DAL persistence on every write, load on startup
- Stream: last-emitted boundary (offset management stays with backend)

**Phase 3 — Reactor Persistence**
- Cache + dirty flags persistence after execution
- Idle-time periodic persistence
- Startup loading from DAL
- Sequential queue persistence

**Phase 4 — Health State Machines**
- `AccumulatorHealth` enum + `watch` channel
- `ReactorHealth` enum + startup gating
- Wire into health endpoints (replace hard-coded status)
- Degraded mode implementation

**Phase 5 — Shutdown & Supervisor**
- Graceful shutdown integration (server → scheduler → components)
- Individual accumulator restart
- Failure counting + exponential backoff
- Recovery event recording

**Phase 6 — Emission Tracking**
- Sequence numbers on boundaries
- Persistence of sequence counters
- Reactor-side deduplication

**Phase 7 — Integration & Validation**
- Restart recovery integration tests
- Individual component failure tests
- Soak test with supervisor-triggered restarts
- Health endpoint verification tests
