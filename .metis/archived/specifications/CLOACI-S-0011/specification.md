---
id: continuous-scheduling-recovery
level: specification
title: "Continuous Scheduling Recovery System"
short_code: "CLOACI-S-0011"
created_at: 2026-03-23T02:16:54.959482+00:00
updated_at: 2026-03-23T02:16:54.959482+00:00
parent: CLOACI-S-0008
blocked_by: []
archived: true

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Continuous Scheduling Recovery System

Recovery design for the continuous (reactive, data-driven) scheduling system.
Covers crash recovery, state persistence, semantic guarantees, data source
reconnection, and preparation for distributed executors.

Parent: CLOACI-S-0008 (Continuous Scheduler).
Related: CLOACI-S-0002 (Computation Boundaries), CLOACI-S-0005 (Accumulators),
CLOACI-S-0006 (Watermarks), CLOACI-S-0007 (Execution Ledger).

---

## 1. Overview

The continuous scheduler is a reactive loop that fires tasks based on data
boundaries rather than cron or on-demand triggers. Its state is spread across
five in-memory structures that must survive crashes:

| Structure | Location | What it holds |
|-----------|----------|---------------|
| `BoundaryLedger` | `Arc<RwLock<BoundaryLedger>>` | Per-source watermarks (how far each source has been produced) |
| Per-edge `SignalAccumulator` | `Arc<Mutex<Box<dyn SignalAccumulator>>>` on each `GraphEdge` | Buffered boundaries waiting to trigger a task, plus consumer watermark |
| `ExecutionLedger` | `Arc<RwLock<ExecutionLedger>>` | In-memory log of all scheduler events (bounded VecDeque) |
| `DetectorStateStore` | `DetectorStateStore` on scheduler | Latest/committed detector checkpoint per source |
| Pending Boundary WAL | `pending_boundaries` DB table | Durable write-ahead log of boundaries as they arrive |

The recovery system ensures that after a crash, the scheduler can resume
without data loss (no boundaries skipped) and without unbounded reprocessing
(no boundaries re-executed that already succeeded).

---

## 2. Crash Scenario Analysis

### Scenario 1: Crash while accumulator is buffering data

**Timeline**: Detector completes -> boundaries routed to accumulators -> CRASH
(accumulator buffer is in memory only).

**Current mitigation (already implemented)**: Step 2.5 in `scheduler.run()`
persists every boundary to the `pending_boundaries` WAL table *before*
accumulator readiness is checked. Each edge has a drain cursor
(`edge_drain_cursors` table) tracking which WAL entries it has consumed.

**Recovery**: On restart, `restore_pending_boundaries()` replays WAL entries
after each edge's drain cursor into the corresponding accumulator. Boundaries
that were persisted to the WAL but not yet drained are automatically replayed.

**Gap**: Boundaries are persisted *after* they are routed to accumulators
(Step 2 routes, Step 2.5 persists). If the process crashes between Step 2 and
Step 2.5, the boundary is in the accumulator (lost on crash) but not in the
WAL. On restart, those boundaries are lost.

**Required fix**: Move WAL persistence to *before* accumulator routing. The
sequence must be: (1) persist to WAL, (2) route to accumulator. This is a
write-ahead log in the strict sense -- write before acting.

### Scenario 2: Crash after trigger fires but before task completes

**Timeline**: Accumulator drains -> task execution begins -> CRASH (task
never completes, drain already happened).

**Current mitigation**: The accumulator's `drain()` clears its buffer and
updates its in-memory consumer watermark. The drain cursor is advanced
*after* task execution (in the persistence batch at the end of the loop
iteration). If the process crashes during task execution, the drain cursor
has NOT been advanced yet.

**Recovery**: On restart, `restore_pending_boundaries()` loads WAL entries
after the drain cursor. Since the cursor was not advanced, the same
boundaries are replayed into the accumulator, and the task will fire again.

**Semantic**: This is at-least-once execution. The same boundary range may
produce duplicate task executions. This is the correct default for data
pipelines (idempotent writes are the responsibility of the task).

**Gap**: The accumulator state (`consumer_watermark`) is persisted in the
same post-execution batch. If the crash happens after the task writes its
output but before the accumulator state is persisted, restart will replay
the boundaries AND the consumer watermark will be stale. This is safe
(at-least-once) but could cause a full re-execution of the boundary range.

### Scenario 3: Crash after task completes but before watermark advances

**Timeline**: Task completes -> output written to ledger -> CRASH (before
accumulator state and drain cursors are persisted).

**Current mitigation**: Task completion is written to the in-memory
`ExecutionLedger` immediately. Accumulator state and drain cursors are
persisted in the batch at the end of the loop iteration.

**Recovery**: Since neither the drain cursor nor the accumulator state was
persisted, restart replays the boundaries and the task fires again.

**Semantic**: At-least-once. The task ran, its side effects (external writes)
happened, but the scheduler does not know. The task must be idempotent.

**Required enhancement**: For exactly-once semantics, the task's completion
must be durably recorded *atomically* with the drain cursor advance. See
Section 4.

### Scenario 4: Data source connection lost

**Timeline**: Detector workflow fails to connect to external data source.

**Current behavior**: The detector workflow fails. The scheduler records a
`TaskFailed` event in the execution ledger. No boundaries are emitted. The
accumulator continues to hold whatever boundaries it already has.

**Recovery**: On the next detector scheduling cycle, the detector workflow
runs again and attempts to reconnect. If the connection succeeds, the
detector resumes from its `committed` state (restored from
`detector_state` table via `DetectorStateStore`).

**Gap**: There is no exponential backoff or circuit breaker for detector
re-execution. Rapid detector failures could overwhelm the system.

**Required enhancement**: Add configurable retry policy for detector
workflows (backoff, max attempts, circuit breaker threshold).

### Scenario 5: Multiple data sources with different watermark positions

**Timeline**: Source A is at offset 1000, Source B is at offset 500. Task
depends on both (JoinMode::All). Source A crashes.

**Current behavior**: Each source has independent watermarks in the
`BoundaryLedger` and independent entries in the `pending_boundaries` WAL.
Each edge has its own drain cursor. The `JoinMode::All` check requires all
accumulators to be ready before firing.

**Recovery**: On restart, each edge is independently restored from its own
drain cursor position. Source A's accumulator may have more data than Source
B's. The join mode logic handles the asymmetry -- it waits for all inputs.

**No gap**: The per-edge cursor model handles this correctly.

### Scenario 6: Late-arriving data after watermark has advanced

**Timeline**: Source watermark is at offset 1000. A boundary for offset
range [800, 900] arrives late.

**Current behavior**: The scheduler checks `boundary_ledger.covers()` to
detect late arrivals and applies the edge's `LateArrivalPolicy`:
- `Discard`: Drop silently (data loss by design)
- `AccumulateForward`: Buffer for next cycle (eventual consistency)
- `Retrigger`: Buffer and re-execute (correctness priority)

**Recovery relevance**: Late arrival handling is orthogonal to crash
recovery. The WAL persists ALL boundaries (including late ones) before
routing. After a crash, late boundaries in the WAL are replayed and the
late arrival policy is re-applied based on the restored consumer watermark.

**Gap**: If the consumer watermark was not persisted (Scenario 3), a
boundary that was previously considered "late" may now be considered
"normal" after restart. This is safe (it will be processed) but changes
the execution path. No data loss.

---

## 3. Watermark Persistence Strategy

### 3.1 What to persist

Three kinds of watermark state must survive crashes:

| State | Table | When persisted |
|-------|-------|----------------|
| Consumer watermark (per-edge) | `accumulator_state.consumer_watermark` | After successful task execution |
| Source watermark (per-source) | Currently NOT persisted | Should be persisted on advance |
| Detector checkpoint (per-source) | `detector_state.committed_state` | When all consumers have drained |

### 3.2 Source watermark persistence (NEW)

The `BoundaryLedger` (source watermarks) is currently purely in-memory.
On restart, source watermarks start at zero, which means:

- `WindowedAccumulator` in `WaitForWatermark` mode will block until the
  first detector run re-establishes the watermark.
- Late arrival detection will incorrectly treat all replayed boundaries
  as "not late" until the watermark catches up.

**Design**: Add a `source_watermarks` table:

```sql
CREATE TABLE source_watermarks (
    source_name  TEXT PRIMARY KEY,
    watermark_json TEXT NOT NULL,
    updated_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Persist on every `BoundaryLedger::advance()` call (piggyback on the
existing DAL pattern). Restore in a new `restore_source_watermarks()`
step during startup, called before `restore_pending_boundaries()`.

### 3.3 When to save

| Event | What is persisted | Atomicity requirement |
|-------|-------------------|-----------------------|
| Boundary arrives (WAL append) | `pending_boundaries` row | Single INSERT, auto-commit |
| Source watermark advances | `source_watermarks` row | Single UPSERT, auto-commit |
| Task completes successfully | `accumulator_state` + `edge_drain_cursors` + `detector_state` (if commit gate passes) | Should be a single transaction |
| Task fails | Nothing new persisted (boundaries remain in WAL for retry) | N/A |

### 3.4 Persistence ordering on successful task completion

Current code persists accumulator state, advances drain cursors, and
checks the detector commit gate in separate async operations. These should
be grouped into a single database transaction to prevent partial state:

```
BEGIN TRANSACTION;
  -- For each edge that drained:
  UPDATE accumulator_state SET consumer_watermark = ? WHERE edge_id = ?;
  UPDATE edge_drain_cursors SET last_drain_id = ? WHERE edge_id = ?;
  -- If all edges for a source have drained (commit gate):
  UPDATE detector_state SET committed_state = ? WHERE source_name = ?;
  DELETE FROM pending_boundaries WHERE source_name = ? AND id <= ?;
COMMIT;
```

This eliminates the partial-persistence gaps identified in Scenarios 2
and 3.

---

## 4. Accumulator Recovery

### 4.1 Buffer state on crash

Accumulator buffers are in-memory `Vec<BufferedBoundary>`. They are NOT
persisted directly. Instead, recovery works through the WAL:

1. On startup, each edge's drain cursor tells us "last WAL entry I consumed."
2. `restore_pending_boundaries()` loads all WAL entries after that cursor.
3. Each boundary is deserialized and fed to `accumulator.receive()`.
4. The accumulator is now in the same state as before the crash (modulo
   timing-dependent trigger policy state like `WallClockWindow`).

### 4.2 Trigger policy state

`TriggerPolicy` implementations hold internal state:
- `WallClockWindow`: `last_drain_at: Instant` (wall clock, not persistable)
- `WallClockDebounce`: compares `received_at` against current time
- `BoundaryCount`: stateless (counts buffer length)
- `Immediate`: stateless

On restart, `WallClockWindow` resets to "now" as its reference point. This
means the first trigger after restart may fire earlier or later than the
configured window. This is acceptable -- the window policy is a throughput
optimization, not a correctness guarantee.

**No change needed**: Trigger policies are best-effort timing hints. The
WAL + drain cursor mechanism ensures no data is lost regardless of when
the trigger fires.

### 4.3 Consumer watermark restoration

`restore_from_persisted_state()` loads `accumulator_state` rows and calls
`set_consumer_watermark()` on matching edges. This must run AFTER
`restore_pending_boundaries()` so that the watermark reflects the most
recent drain, not replay state.

**Current code is correct**: The startup sequence in `services.rs` is:
```
scheduler.init_drain_cursors().await;       // 1. Ensure cursor rows exist
scheduler.restore_pending_boundaries().await; // 2. Replay WAL into accumulators
scheduler.restore_from_persisted_state().await; // 3. Restore consumer watermarks
scheduler.restore_detector_state().await;     // 4. Restore detector checkpoints
```

The ordering is intentional. Step 3 overwrites any watermark set by
Step 2's replayed boundaries, which is correct because the persisted
watermark represents the last *committed* drain point.

---

## 5. Boundary Ledger Recovery

### 5.1 Current state

The `ExecutionLedger` is a bounded in-memory `VecDeque` that evicts old
events. It is NOT persisted. On restart, the ledger starts empty.

### 5.2 Impact of empty ledger on restart

The scheduler's `run()` loop reads the ledger for `TaskCompleted` events
from detectors. On restart:

- Cursor starts at 0, ledger is empty. No events to process.
- Detectors are re-scheduled by their triggers (cron, LedgerTrigger, etc).
- First detector completion writes to the ledger, cursor picks it up.
- Boundaries flow through accumulators as normal.

The gap between restart and first detector completion is a "cold start"
period where no new boundaries arrive. Pre-existing boundaries in the WAL
are already restored to accumulators and may fire immediately if the
trigger policy is satisfied.

### 5.3 LedgerTrigger recovery

`LedgerTrigger` watches for task completions to fire derived detectors.
On restart, its cursor starts at 0 and the ledger is empty. Derived
detectors will not fire until their upstream tasks complete at least once
post-restart.

**Acceptable**: The first detector run for each source covers the gap.
The detector resumes from its `committed` state, so it will detect any
changes that occurred during the downtime.

### 5.4 Design decision: Do NOT persist the ExecutionLedger

The `ExecutionLedger` is an observation/debugging tool, not a recovery
mechanism. All durable state lives in the WAL, drain cursors, accumulator
state, and detector state tables. Persisting the ledger would add
complexity without improving recovery correctness.

---

## 6. Exactly-Once vs At-Least-Once Semantics

### 6.1 At-least-once (default)

The current architecture provides at-least-once semantics:

- Boundaries are persisted to WAL before processing.
- Drain cursors are advanced after successful task execution.
- On crash, un-advanced cursors cause boundary replay.
- Tasks may execute more than once for the same boundary range.

**Requirement**: Tasks must be idempotent. For database writes, use
UPSERT or merge operations. For file writes, use deterministic paths.
For API calls, use idempotency keys.

### 6.2 Exactly-once (opt-in)

True exactly-once requires coupling the task's side effects with the
scheduler's state advance. Two approaches:

#### 6.2.1 Transactional outbox (recommended for DB-writing tasks)

If the task writes to the same database as Cloacina's state:

```
BEGIN TRANSACTION;
  -- Task's business writes
  INSERT INTO target_table ...;
  -- Scheduler's state advance
  UPDATE edge_drain_cursors SET last_drain_id = ? WHERE edge_id = ?;
  UPDATE accumulator_state SET consumer_watermark = ? WHERE edge_id = ?;
COMMIT;
```

This is exactly-once because the business write and the cursor advance
are atomic. If the transaction fails, neither takes effect.

**Implementation**: Add a `TransactionalTaskContext` that exposes the
database connection to the task, allowing it to participate in the
scheduler's commit transaction.

#### 6.2.2 Boundary deduplication (recommended for external-system tasks)

For tasks that write to external systems (APIs, files, Kafka):

- Record each completed boundary range in a `completed_boundaries` table.
- On replay, check if the boundary range was already completed.
- Skip execution if it was.

```sql
CREATE TABLE completed_boundaries (
    edge_id       TEXT NOT NULL,
    boundary_json TEXT NOT NULL,
    completed_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (edge_id, boundary_json)
);
```

**Trade-off**: Requires storing every completed boundary, which grows
unboundedly. Add a retention policy that purges entries older than the
source watermark (boundaries behind the watermark will never be replayed).

### 6.3 Configuration

```rust
/// Semantic guarantee level for a continuous task.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeliveryGuarantee {
    /// Task may execute multiple times for the same boundary.
    /// Task must be idempotent. Default.
    AtLeastOnce,
    /// Boundary deduplication prevents re-execution.
    /// Adds per-boundary completion tracking overhead.
    ExactlyOnce,
}
```

Configure per-task in `ContinuousTaskRegistration`:

```rust
pub struct ContinuousTaskRegistration {
    pub id: String,
    pub sources: Vec<String>,
    pub referenced: Vec<String>,
    pub delivery_guarantee: DeliveryGuarantee,  // NEW
}
```

---

## 7. Data Source Reconnection and Resume

### 7.1 Detector resume from committed state

When a detector workflow starts, the scheduler provides the committed
detector state via context key `__detector_committed_state`. The detector
uses this to resume scanning from where it left off.

**Current implementation**: `restore_detector_state()` loads committed
states from the `detector_state` table into `DetectorStateStore`. The
scheduler reads `detector_state_store.get_committed()` when launching
detector workflows.

**Gap**: The committed state is passed to detectors, but the mechanism
for injecting it into the detector's execution context is implicit
(via `__last_known_state` context key). This should be formalized.

### 7.2 Connection failure handling

**Current**: Detector workflow fails -> `TaskFailed` ledger event -> no
boundaries emitted -> next scheduling cycle retries.

**Enhancement**: Add a per-source retry policy:

```rust
pub struct DetectorRetryPolicy {
    /// Initial backoff duration.
    pub initial_backoff: Duration,
    /// Maximum backoff duration.
    pub max_backoff: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential).
    pub multiplier: f64,
    /// Maximum consecutive failures before circuit-breaking.
    pub circuit_breaker_threshold: u32,
    /// How long the circuit stays open before allowing a probe.
    pub circuit_breaker_cooldown: Duration,
}
```

Track consecutive failures per source in the `DetectorStateStore`:

```rust
pub struct DetectorCheckpoint {
    pub latest: Option<serde_json::Value>,
    pub committed: Option<serde_json::Value>,
    pub edge_drain_states: HashMap<String, serde_json::Value>,
    pub consecutive_failures: u32,         // NEW
    pub last_failure_at: Option<DateTime<Utc>>, // NEW
}
```

### 7.3 Stale watermark detection

If a source's detector has not advanced its watermark for a configurable
duration, emit a warning metric. This catches:
- Silent detector failures (detector runs but finds no changes)
- Stuck external systems
- Misconfigured detector workflows

---

## 8. Integration with General Task Recovery

### 8.1 Relationship to cron recovery

The existing `CronRecoveryService` handles orphaned cron executions (stuck
in "Running" state). Continuous tasks have a different model:

| Aspect | Cron tasks | Continuous tasks |
|--------|-----------|-----------------|
| Scheduling | Time-driven | Data-driven |
| State tracking | `cron_executions` table | WAL + drain cursors + accumulator state |
| Orphan detection | `task_executions.status = 'Running'` for too long | Drain cursor not advanced after task started |
| Recovery action | Reset to "Ready" and re-execute | Replay boundaries from WAL (automatic on restart) |

### 8.2 Continuous task orphan detection

Continuous tasks do not use the `task_executions` table (they execute
inline in the scheduler loop). Orphan detection is implicit:

- If the scheduler process crashes, ALL in-flight continuous tasks are lost.
- On restart, the WAL replay mechanism re-fires them automatically.
- No explicit orphan detection is needed for single-process mode.

**For distributed mode** (see Section 9), continuous tasks dispatched to
remote executors DO need orphan detection. This will use the same
`task_executions` table with a `continuous` execution type.

### 8.3 Shared infrastructure

Recovery infrastructure shared between cron and continuous:

- `recovery_events` table: audit log of all recovery actions
- `RecoveryEvent` model: can be extended with `ContinuousReplay` variant
- Database connection pooling and transaction support

---

## 9. Preparation for Distributed Executors

### 9.1 Current single-process model

Today, the continuous scheduler executes tasks inline:

```
scheduler.run() loop:
  poll ledger -> route boundaries -> check readiness -> execute task -> persist state
```

All state transitions happen in a single process. The WAL + drain cursor
model works because there is exactly one writer.

### 9.2 Distributed execution model

In the distributed model:

```
Scheduler (coordinator):
  poll ledger -> route boundaries -> check readiness -> DISPATCH to executor

Executor (remote worker):
  receive task + boundary context -> execute -> REPORT completion

Scheduler (coordinator):
  receive completion -> persist state -> advance cursors
```

### 9.3 Required changes for distribution

| Concern | Single-process | Distributed |
|---------|---------------|-------------|
| Task dispatch | Direct function call | Message queue / RPC |
| Completion notification | Return value | Callback / completion event |
| State persistence | Scheduler persists after execution | Scheduler persists after receiving completion |
| Failure detection | Exception / timeout | Heartbeat timeout / executor crash |
| At-least-once guarantee | WAL replay on restart | WAL replay + executor-side dedup |

### 9.4 Architectural preparation (do now)

To minimize future refactoring, the following abstractions should be
introduced:

1. **`TaskDispatcher` trait**: Abstract task execution behind a trait.
   Single-process mode uses `InlineDispatcher`. Distributed mode will use
   `RemoteDispatcher`.

```rust
#[async_trait]
pub trait TaskDispatcher: Send + Sync {
    /// Dispatch a task for execution. Returns a handle to track completion.
    async fn dispatch(
        &self,
        task_id: &str,
        context: Context<serde_json::Value>,
        boundary: ComputationBoundary,
    ) -> Result<DispatchHandle, DispatchError>;
}
```

2. **`CompletionReceiver` trait**: Abstract how the scheduler learns about
   task completions. Currently it reads the `ExecutionLedger` directly.
   In distributed mode, completions arrive via a message channel.

3. **Execution ID**: Each task firing should get a unique execution ID
   (UUID) that is:
   - Written to the WAL alongside the boundary
   - Passed to the executor
   - Returned in the completion notification
   - Used for deduplication and orphan detection

4. **Fencing tokens**: When the scheduler restarts, it must invalidate
   any in-flight executions from the previous incarnation. Use a
   monotonic `scheduler_epoch` (persisted, incremented on startup) as a
   fencing token. Completions from a previous epoch are discarded.

### 9.5 State table evolution

The `pending_boundaries` table should gain an `execution_id` and
`scheduler_epoch` column:

```sql
ALTER TABLE pending_boundaries ADD COLUMN execution_id TEXT;
ALTER TABLE pending_boundaries ADD COLUMN scheduler_epoch BIGINT DEFAULT 0;
```

The `accumulator_state` table should gain a `scheduler_epoch`:

```sql
ALTER TABLE accumulator_state ADD COLUMN scheduler_epoch BIGINT DEFAULT 0;
```

---

## 10. Configuration Options

### 10.1 Per-scheduler configuration

```rust
pub struct ContinuousSchedulerConfig {
    // Existing fields
    pub poll_interval: Duration,
    pub max_fired_tasks: usize,
    pub task_timeout: Option<Duration>,

    // Recovery configuration (NEW)
    /// How often to checkpoint accumulator state (even without drains).
    /// Reduces replay window on crash. Set to None to only checkpoint on drain.
    pub checkpoint_interval: Option<Duration>,

    /// Maximum WAL size per source before forced compaction.
    /// Prevents unbounded WAL growth if a consumer falls behind.
    pub max_wal_entries_per_source: usize,

    /// Scheduler epoch (auto-incremented on startup).
    /// Used as fencing token for distributed executors.
    pub scheduler_epoch: u64,
}
```

### 10.2 Per-task configuration

```rust
pub struct ContinuousTaskRegistration {
    pub id: String,
    pub sources: Vec<String>,
    pub referenced: Vec<String>,

    // Recovery configuration (NEW)
    pub delivery_guarantee: DeliveryGuarantee,

    /// Maximum number of retry attempts for failed executions.
    /// 0 = no retries (fail fast). Default: 3.
    pub max_retries: u32,

    /// Backoff between retries.
    pub retry_backoff: Duration,
}
```

### 10.3 Per-source configuration

```rust
pub struct DataSource {
    pub name: String,
    pub connection: Box<dyn DataConnection>,
    pub detector_workflow: String,
    pub lineage: DataSourceMetadata,

    // Recovery configuration (NEW)
    pub detector_retry_policy: DetectorRetryPolicy,

    /// Maximum acceptable watermark staleness before alerting.
    pub watermark_staleness_threshold: Option<Duration>,
}
```

---

## 11. Startup Recovery Sequence (Revised)

The complete startup sequence, incorporating all recovery mechanisms:

```
1. Increment scheduler_epoch in DB (fencing for distributed mode)
2. Assemble DataSourceGraph from registered sources and tasks
3. Initialize drain cursors for all edges (init_drain_cursors)
4. Restore source watermarks into BoundaryLedger (NEW)
5. Restore pending boundaries from WAL into accumulators (restore_pending_boundaries)
6. Restore consumer watermarks from accumulator_state (restore_from_persisted_state)
7. Restore detector committed states (restore_detector_state)
8. Register task implementations
9. Start detector scheduling (triggers begin polling)
10. Enter main scheduling loop
```

Steps 3-7 are the critical recovery window. All are idempotent --
running them multiple times produces the same result. Step 1 ensures
any completions from a previous scheduler incarnation are fenced off.

---

## 12. Implementation Priority

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| P0 | Move WAL write before accumulator routing (fix Scenario 1 gap) | S | Prevents boundary loss on crash |
| P0 | Transaction batch for post-execution persistence (Section 3.4) | M | Prevents partial state on crash |
| P1 | Source watermark persistence (Section 3.2) | S | Faster recovery for WindowedAccumulator |
| P1 | Detector retry policy with backoff (Section 7.2) | M | Prevents storm on connection failure |
| P2 | Exactly-once via boundary deduplication (Section 6.2.2) | M | Opt-in for non-idempotent tasks |
| P2 | TaskDispatcher trait abstraction (Section 9.4) | M | Prepares for distributed executors |
| P2 | Scheduler epoch / fencing token (Section 9.4) | S | Prepares for distributed executors |
| P3 | Transactional outbox for DB-writing tasks (Section 6.2.1) | L | Exactly-once for co-located tasks |
| P3 | Stale watermark alerting (Section 7.3) | S | Operational visibility |
| P3 | Periodic checkpoint interval (Section 10.1) | S | Reduces replay window |

---

## Constraints

### Technical Constraints
- Must work with both SQLite and PostgreSQL backends (existing `dispatch_backend!` macro)
- WAL entries must be ordered per-source (existing `pending_boundaries.id` auto-increment)
- Watermark monotonicity must be preserved across restarts (existing `BoundaryLedger::advance()` rejects backward movement)
- In-memory `ExecutionLedger` is NOT a recovery mechanism; all durable state lives in DB tables

### Design Constraints
- Default to at-least-once semantics (exactly-once is opt-in)
- No mandatory schema migrations for existing deployments (source watermark table is additive)
- Recovery must be automatic on restart (no manual intervention)
- All restore operations must be idempotent
