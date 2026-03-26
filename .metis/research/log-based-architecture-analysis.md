# Log-Based Architecture Analysis for Cloacina

## Executive Summary

This document analyzes where log-based data structures and persistence patterns might benefit Cloacina, evaluating each data domain against log semantics. The analysis is structured to be **open to the possibility but skeptical of naive adoption**.

**Key Finding:** Cloacina's data naturally splits into three categories:
1. **Already log-shaped** - Audit trails that are append-only today
2. **Could benefit from event sourcing** - Execution state that's currently mutable
3. **Fundamentally relational** - Scheduling coordination that needs atomic read-modify-write

A hybrid approach is likely optimal: log-based event sourcing for execution history with materialized views for query patterns, while retaining relational semantics for coordination.

---

## Current Data Landscape

### Table Classification by Mutability

| Table | Mutability | Access Pattern | Current Pain Points |
|-------|------------|----------------|---------------------|
| `contexts` | **Immutable** | Write-once, read-many | None - already log-friendly |
| `recovery_events` | **Append-only** | Sequential audit | None - already log-shaped |
| `cron_executions` | **Append-only** | Audit + dedup check | Unique constraint for dedup |
| `trigger_executions` | **Append-only** | Audit + in-progress check | Partial index for dedup |
| `workflow_registry` | **Immutable** | Random by ID | BLOB storage, not log-shaped |
| `pipeline_executions` | **Mutable** | Status updates, random by ID | State machine transitions |
| `task_executions` | **Highly mutable** | Claim, retry, status | Atomic FOR UPDATE claiming |
| `cron_schedules` | **Mutable** | Polling by timestamp | next_run_at updates |
| `trigger_schedules` | **Mutable** | Polling by timestamp | last_poll_at updates |
| `signing_keys` | **Mutable** | Random by fingerprint | Revocation updates |

---

## Domain-by-Domain Analysis

### 1. Execution State (Pipeline + Task Executions)

**Current Implementation:**
- Mutable rows with state machine transitions
- `status` field updated: NotStarted → Ready → Running → Completed/Failed
- Atomic claiming via `FOR UPDATE SKIP LOCKED`
- Error details, retry counts updated in place

**Log-Based Alternative: Event Sourcing**

```
Event Log (append-only):
┌─────────────────────────────────────────────────────────────────┐
│ event_id | entity_id | event_type      | timestamp | payload   │
├─────────────────────────────────────────────────────────────────┤
│ 1        | task-123  | TaskCreated     | T0        | {name}    │
│ 2        | task-123  | TaskBecameReady | T1        | {}        │
│ 3        | task-123  | TaskClaimed     | T2        | {worker}  │
│ 4        | task-123  | TaskDeferred    | T3        | {reason}  │
│ 5        | task-123  | TaskResumed     | T4        | {}        │
│ 6        | task-123  | TaskCompleted   | T5        | {result}  │
└─────────────────────────────────────────────────────────────────┘

Materialized View (derived, eventually consistent):
┌───────────────────────────────────────────────────┐
│ task_id  | current_status | last_event | attempt │
├───────────────────────────────────────────────────┤
│ task-123 | Completed      | T5         | 1       │
└───────────────────────────────────────────────────┘
```

**Benefits:**
- Complete audit trail of every state change (currently lost)
- Natural replay capability - recreate any historical state
- Debugging: "Why did this task fail the first time?"
- Distributed execution: Events can be replicated across nodes
- Backfill: Replay events to rebuild derived state

**Challenges:**
- **Atomic claiming** - Log append is easy, but "claim if not claimed" requires external coordination
  - Possible solutions: Kafka consumer groups, Redis SETNX, or hybrid approach
- **Query performance** - "Find all running tasks" requires scanning or materialized view
- **Exactly-once** - Event deduplication needed (IGGY supports this)

**Verdict: PROMISING but needs careful design for claiming semantics**

### 2. Audit Trails (Recovery, Cron, Trigger Executions)

**Current Implementation:**
- Already append-only INSERT operations
- Unique constraints for deduplication
- Queried for debugging, rarely for hot-path operations

**Log-Based Alternative:**

These tables are **already log-shaped**. They would map 1:1 to log topics:

```
Topic: cloacina.audit.recovery
Topic: cloacina.audit.cron
Topic: cloacina.audit.trigger
```

**Benefits:**
- Native time-windowed retention (vs manual DELETE WHERE created_at < X)
- Natural partitioning by tenant_id
- Built-in replication for durability
- Consumer groups for downstream processing

**Challenges:**
- Deduplication (currently unique constraints)
  - IGGY: Supports deduplication by message ID
  - Workaround: Hash-based message IDs

**Verdict: EXCELLENT FIT - minimal changes, clear benefits**

### 3. Context Storage

**Current Implementation:**
- Write-once, read-many pattern
- JSON blobs with UUIDs
- Referenced by execution records

**Log-Based Alternative:**

Contexts are immutable and could be stored as log messages:

```
Topic: cloacina.contexts
Key: context_uuid
Value: JSON blob
```

**Benefits:**
- Natural content-addressable storage pattern
- Compaction by key preserves latest (though we only write once)
- Replication for durability

**Challenges:**
- Random access by UUID is not log's strength
  - Solution: Maintain UUID → offset index (or use key-based lookup)
- Binary efficiency - JSON in logs vs BLOB in SQLite

**Verdict: WORKABLE but marginal benefit over current approach**

### 4. Scheduling Coordination (Cron + Trigger Schedules)

**Current Implementation:**
- Polling queries: `WHERE enabled=1 AND next_run_at <= now()`
- Atomic updates: `UPDATE ... SET next_run_at = X WHERE id = Y`
- Distributed safety via database locks

**Log-Based Alternative:**

This is where logs struggle. Scheduling requires:
1. **Point-in-time queries** - "What schedules are due now?"
2. **Atomic read-modify-write** - "Claim this schedule and update next_run_at"
3. **Polling by computed predicate** - Not a sequential scan

**Possible Approaches:**

a) **Event-sourced schedules** (complex)
   - Log: ScheduleCreated, ScheduleEnabled, ScheduleTriggered, ScheduleUpdated
   - Materialized view for "due schedules"
   - Still need coordination for claiming

b) **Hybrid** (pragmatic)
   - Keep schedules in relational store for coordination
   - Emit events to log for audit/replay
   - Best of both worlds

c) **External scheduler** (separation of concerns)
   - Use dedicated scheduling service (temporal, quartz)
   - Log only execution events

**Verdict: POOR FIT for pure log - recommend hybrid or external**

### 5. Workflow Registry

**Current Implementation:**
- Binary BLOB storage (compiled workflow packages)
- Random access by (name, version)
- Immutable once written

**Log-Based Alternative:**

Binary artifacts don't fit logs well:
- Large messages stress log systems
- No benefit from sequential access
- Content-addressable storage (S3, filesystem) is better fit

**Verdict: KEEP RELATIONAL or use object storage**

### 6. Security Infrastructure (Keys, Signatures)

**Current Implementation:**
- Encrypted private keys
- Public key trust relationships
- Revocation via soft-delete

**Log-Based Alternative:**

Security data needs:
- Point lookups by fingerprint
- Revocation checks on every verification
- Atomic trust relationship updates

While an audit log of key operations is valuable, the operational data doesn't fit log patterns.

**Verdict: KEEP RELATIONAL - add audit log for security events**

---

## Architectural Patterns to Consider

### Pattern 1: Event Sourcing with CQRS

```
┌─────────────────────────────────────────────────────────────────┐
│                        Write Side                                │
│  ┌─────────┐    ┌─────────────┐    ┌──────────────────────┐    │
│  │ Command │───▶│ Validate &  │───▶│ Append to Event Log  │    │
│  │ Handler │    │ Apply Logic │    │ (IGGY/Kafka/etc)     │    │
│  └─────────┘    └─────────────┘    └──────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Events flow to consumers
┌─────────────────────────────────────────────────────────────────┐
│                        Read Side                                 │
│  ┌──────────────────────┐    ┌─────────────────────────────┐   │
│  │ Event Consumer       │───▶│ Materialized Views          │   │
│  │ (projection builder) │    │ (SQLite/Postgres/Redis)     │   │
│  └──────────────────────┘    └─────────────────────────────┘   │
│                                        │                        │
│                              ┌─────────┴─────────┐              │
│                              ▼                   ▼              │
│                       ┌───────────┐       ┌───────────┐         │
│                       │ Task View │       │ Pipeline  │         │
│                       │ (by status│       │ View      │         │
│                       │ for poll) │       │ (by ID)   │         │
│                       └───────────┘       └───────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

**Trade-offs:**
- (+) Complete history, replay, audit
- (+) Decoupled write/read optimization
- (-) Eventual consistency complexity
- (-) Coordination still needed for claiming

### Pattern 2: Hybrid with Change Data Capture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Operational Database                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Schedules   │  │ Security    │  │ Executions  │             │
│  │ (Postgres)  │  │ (Postgres)  │  │ (Postgres)  │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
          ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Change Data Capture (CDC)                        │
│         (Debezium / pg_logical / SQLite triggers)               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Event Log                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ All changes captured as events for audit, replay,        │   │
│  │ distributed sync, analytics                              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**Trade-offs:**
- (+) Keep existing relational patterns for coordination
- (+) Get log benefits for audit/replay without rewrite
- (-) Additional infrastructure (CDC pipeline)
- (-) Slight delay between write and log availability

### Pattern 3: Log-First with SQLite as Cache

```
┌─────────────────────────────────────────────────────────────────┐
│                      Event Log (Source of Truth)                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ IGGY / Kafka / RedPanda                                  │   │
│  │ - Partitioned by tenant_id                               │   │
│  │ - Compacted topics for latest state                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Local Materialized Cache                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ SQLite (embedded, per-worker)                            │   │
│  │ - Rebuilt from log on startup                            │   │
│  │ - Fast local queries                                     │   │
│  │ - Disposable - log is truth                              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**Trade-offs:**
- (+) Distributed-first architecture
- (+) SQLite keeps query efficiency
- (+) Natural replay/recovery
- (-) Complex coordination
- (-) Startup time (rebuild from log)

---

## Specific IGGY Considerations

IGGY (Apache Incubating) offers some advantages over Kafka:

| Feature | IGGY | Kafka | Relevance to Cloacina |
|---------|------|-------|----------------------|
| Protocol | Binary, low-latency | Binary (Kafka protocol) | Both work |
| Storage | Persistent, file-based | Persistent, file-based | Both work |
| Exactly-once | Built-in dedup | Idempotent producer | Critical for claims |
| Consumer groups | Yes | Yes | Distributed workers |
| Compaction | Topic-level | Topic-level | State snapshots |
| Embedded mode | Possible | No (needs cluster) | Simpler deployments |
| Multi-tenancy | Stream isolation | Topic naming | Need tenant streams |

**IGGY's embedded mode** is particularly interesting - it could allow:
- Single-binary deployment (IGGY + Cloacina)
- Local development without external services
- Gradual migration path

---

## The "Log-Backed SQLite" Plugin Idea

You mentioned exploring a log-based plugin for SQLite. Here's what that might look like:

### Concept: Write-Ahead Log as Event Log

SQLite already has a WAL (Write-Ahead Log). We could:

1. **Intercept WAL writes** and publish to IGGY
2. **On startup**, rebuild SQLite from IGGY log
3. **Cross-node sync** via IGGY replication

```
┌─────────────────────────────────────────────────────────────────┐
│ Node A                                   Node B                  │
│ ┌───────────┐                           ┌───────────┐           │
│ │ SQLite    │                           │ SQLite    │           │
│ │ (primary) │                           │ (replica) │           │
│ └─────┬─────┘                           └─────▲─────┘           │
│       │ WAL intercept                         │ replay          │
│       ▼                                       │                 │
│ ┌───────────────────────────────────────────────────────────┐  │
│ │                    IGGY Event Log                          │  │
│ │ (replicated across nodes)                                  │  │
│ └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Options

**Option A: Application-Level Capture**
- Wrap all DAL operations to emit events
- Pro: Full control, semantic events
- Con: Requires touching all write paths

**Option B: SQLite VFS Extension**
- Custom VFS that intercepts writes
- Pro: Transparent to application
- Con: Complex, low-level

**Option C: External CDC (Debezium-like)**
- Poll SQLite for changes
- Pro: Non-invasive
- Con: Polling latency

**Option D: cr-sqlite (Conflict-free Replicated SQLite)**
- Existing project for SQLite sync via CRDTs
- Pro: Battle-tested
- Con: Different consistency model

---

## Recommended Exploration Path

Based on this analysis, here's a pragmatic path forward:

### Phase 1: Audit Log Enhancement (Low Risk, High Value)

**Goal:** Prove log infrastructure without disrupting core operations

1. Deploy IGGY (or Kafka) alongside existing Postgres/SQLite
2. Emit events for operations that are already append-only:
   - Recovery events
   - Cron executions
   - Trigger executions
3. Build simple consumers:
   - Metrics aggregation
   - Compliance reporting
   - Debug tools

**Success Criteria:**
- Can replay 30 days of audit history
- Dashboard shows execution patterns
- No impact on core performance

### Phase 2: Execution Event Sourcing (Medium Risk, High Value)

**Goal:** Event-source task execution for replay/debug capability

1. Add event emission on state transitions (parallel to existing DB writes)
2. Build projection that can reconstruct task state from events
3. Compare projected state to DB state (validation)
4. Build replay tooling:
   - "Show me everything that happened to task X"
   - "Rebuild execution history from timestamp Y"

**Success Criteria:**
- Can reconstruct any task's history from log
- Replay matches actual DB state
- Debugging significantly improved

### Phase 3: Distributed Execution (High Risk, High Value)

**Goal:** Use log as coordination mechanism for distributed workers

1. Experiment with claiming via log (consumer groups)
2. Measure latency vs FOR UPDATE claiming
3. Evaluate consistency trade-offs
4. Decide: hybrid or full migration

**Success Criteria:**
- Multi-node execution without shared database
- Acceptable claiming latency
- Clear consistency model

### Phase 4: Full Evaluation (Decision Point)

After phases 1-3, you'll have real data on:
- Log infrastructure operational costs
- Replay/audit value realized
- Distributed execution feasibility
- Developer experience impact

**Decision:** Full migration, hybrid long-term, or log-for-audit-only

---

## What We'd Lose by Going Full Log

Being skeptical, here's what relational gives us that logs don't:

1. **Ad-hoc queries** - `SELECT * FROM tasks WHERE status='Failed' AND created_at > X`
   - Log equivalent: Full scan or pre-built projection

2. **Atomic read-modify-write** - `UPDATE ... WHERE id=X AND status='Ready'`
   - Log equivalent: External coordination or optimistic locking

3. **Foreign key integrity** - Database enforces relationships
   - Log equivalent: Application-level validation

4. **Index flexibility** - Add index for new query pattern
   - Log equivalent: Build new projection

5. **Tooling ecosystem** - SQL clients, ORMs, migrations
   - Log equivalent: Custom tooling, less mature

6. **ACID transactions** - Multi-table atomic operations
   - Log equivalent: Saga patterns, eventual consistency

---

## What We'd Gain by Going Log-First

Being open to the possibility, here's the upside:

1. **Complete audit trail** - Every state change preserved
   - Current: Lost when row updated

2. **Natural replay** - Rebuild state from any point
   - Current: Would need shadow tables

3. **Distributed-native** - Log replication built-in
   - Current: Database replication is complex

4. **Temporal queries** - "What was state at time T?"
   - Current: Would need event tables

5. **Decoupled scaling** - Read/write paths independent
   - Current: Database is bottleneck

6. **Simpler backfill** - Replay with modified logic
   - Current: Complex migration scripts

---

## Conclusion

The question "What would it mean to utilize more log-based patterns?" has a nuanced answer:

**For audit trails:** Adopt immediately. Already log-shaped, clear benefits.

**For execution state:** Event sourcing is promising for replay/audit, but keep materialized views in relational store for queries. Hybrid approach.

**For coordination (scheduling):** Keep relational. Logs don't help with point-in-time queries and atomic claiming.

**For workflow storage:** Keep relational or object storage. Binary blobs don't fit logs.

**For distributed execution:** Log-based coordination is possible but requires careful design. Worth exploring but not obvious win.

**Full Postgres replacement?** Unlikely to be clean. Hybrid with log-for-events, relational-for-coordination is more pragmatic.

---

## Next Steps

1. **Spike:** Deploy IGGY in dev environment
2. **Prototype:** Emit execution events alongside DB writes
3. **Measure:** Compare write latency, storage costs
4. **Build:** Simple replay tool for debugging
5. **Evaluate:** After 30 days, assess value vs complexity
