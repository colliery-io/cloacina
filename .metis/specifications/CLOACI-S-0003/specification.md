---
id: continuous-task-execution-model
level: specification
title: "Continuous Task Execution Model"
short_code: "CLOACI-S-0003"
created_at: 2026-04-04T11:45:21.677610+00:00
updated_at: 2026-04-04T11:45:21.677610+00:00
parent: CLOACI-I-0053
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Continuous Task Execution Model

## Overview

This specification defines how continuous tasks execute within Cloacina's reactive scheduling system. It covers three areas that are new or significantly extended beyond the archived implementation:

1. **Context injection** — how accumulated boundaries and checkpoint state are delivered to tasks
2. **NoFire conditional propagation** — how tasks signal "no downstream action needed"
3. **Per-task execution modes** — how tasks declare their runtime requirements (async, blocking, dedicated thread)
4. **Checkpoints** — how tasks persist cumulative state across executions

### Relationship to other specs

- **CLOACI-S-0001** (Core Architecture) — defines the scheduler loop that fires tasks and processes their results
- **CLOACI-S-0002** (ComputationBoundary & Accumulators) — defines the accumulator drain that produces the context injected into tasks

## Context Injection

When the scheduler fires a continuous task, it constructs an execution `Context` containing all the information the task needs. The task does not reach outside this context to read state — everything is delivered through it.

### Injected Keys

| Context key | Type | Source | Description |
|-------------|------|--------|-------------|
| `__boundary` | `ComputationBoundary` (JSON) | Accumulator drain (coalesced) | The coalesced boundary from this task's primary input edge |
| `__boundaries` | `Map<String, ComputationBoundary>` | All input edge drains | For multi-input tasks: a map from edge name to coalesced boundary. Contains the latest drained value from each input edge. |
| `__signals_coalesced` | `u64` | Accumulator metrics | Number of raw boundaries merged into the coalesced boundary |
| `__accumulator_lag_ms` | `u64` | Accumulator metrics | Maximum ingestion latency across coalesced boundaries |
| `__checkpoint` | `serde_json::Value` | Checkpoint store | Restored checkpoint from previous execution (if any). See Checkpoints section. |
| `__source_name` | `String` | Edge metadata | Which data source triggered this firing (for JoinMode::Any, identifies which source had new data) |

### Multi-Input Context Assembly

For tasks with multiple input edges (e.g., a decision engine consuming 3 materializers):

1. The scheduler drains all ready accumulators per JoinMode semantics
2. Each drain produces a `CoalescedResult` with a single coalesced boundary
3. All coalesced boundaries are assembled into the `__boundaries` map, keyed by edge name
4. The `__boundary` key contains the boundary from the edge that triggered firing (for JoinMode::Any) or the first edge (for JoinMode::All)
5. If an edge has not been drained yet (first execution, or JoinMode::Any where that source hasn't fired), its entry in `__boundaries` is absent — the task must handle missing edges

This means the task receives a **correlated snapshot** — the latest value from each input at the time of drain. The scheduler ensures atomicity: all accumulators are drained in a single operation, so no concurrent writes can produce a partial snapshot.

### Example: Decision Engine Context

A decision engine with edges from `alpha_materializer`, `beta_materializer`, and `gamma_materializer` receives:

```json
{
  "__boundaries": {
    "alpha_materializer": { "kind": "Custom", "value": { "top_high": 1.05, "top_low": 0.95 } },
    "beta_materializer": { "kind": "Custom", "value": { "estimate": 0.52 } },
    "gamma_materializer": { "kind": "Custom", "value": { "exposure": 150.0 } }
  },
  "__boundary": { "kind": "Custom", "value": { "top_high": 1.05, "top_low": 0.95 } },
  "__source_name": "alpha_materializer",
  "__signals_coalesced": 3,
  "__accumulator_lag_ms": 2,
  "__checkpoint": { "previous_output_high": 0.85, "previous_output_low": 0.62 }
}
```

## NoFire Conditional Propagation

### Problem

In the state-materialization pattern, many task executions produce no meaningful change. Examples:
- An orderbook update arrives but the top-of-book is identical to the previous update
- A pricing estimate is within epsilon of the previous value
- A decision engine computes the same output as last time

Without NoFire, every execution unconditionally triggers downstream tasks, wasting computation.

### Design

A continuous task can return one of two result variants:

```rust
enum ContinuousTaskResult {
    /// Normal completion — state changed, propagate to downstream edges
    Fire {
        context: Context,           // output context for downstream tasks
        checkpoint: Option<Value>,  // optional checkpoint to persist
    },
    /// State updated but no downstream action needed
    NoFire {
        checkpoint: Option<Value>,  // still persist checkpoint even on NoFire
        reason: Option<String>,     // optional human-readable reason for observability
    },
}
```

### Scheduler Behavior

When a task returns `ContinuousTaskResult::Fire`:
1. Extract output context
2. Route output as boundaries to downstream edges (per DataSourceGraph)
3. Persist checkpoint if provided
4. Record `TaskCompleted` in ExecutionLedger

When a task returns `ContinuousTaskResult::NoFire`:
1. **Do NOT route** output to downstream edges — downstream tasks do not fire
2. Persist checkpoint if provided (NoFire does not mean "nothing happened" — the task may have updated internal state)
3. Record `TaskCompletedNoFire` in ExecutionLedger with optional reason

### Key Design Decisions

- **NoFire is explicit, not implicit.** A task must return the NoFire variant — there is no way to infer it from an empty context or null output. This makes the decision visible and auditable.
- **Checkpoints persist on NoFire.** A materializer that receives a boundary, updates cumulative state (e.g., position tracking), but determines the downstream value hasn't changed should still checkpoint the new cumulative state. NoFire suppresses downstream propagation, not state persistence.
- **NoFire is recorded in the ledger.** This enables observability — you can measure how often tasks suppress downstream propagation and identify tasks that fire but rarely produce meaningful changes.

### Use Cases

| Task | NoFire condition | Why |
|------|-----------------|-----|
| beta_materializer | `abs(new_estimate - previous_estimate) < 0.001` | Skip if estimate hasn't meaningfully changed |
| gamma_materializer | Never (always fires — exposure changes are always meaningful) | — |
| Decision engine | Guard: `estimate` is None (no beta data yet) | Can't compute without all inputs |
| Decision engine | Reconcile: `output_high` and `output_low` unchanged from last checkpoint | Skip if output hasn't changed |

## Per-Task Execution Mode

### Problem

The archived continuous scheduler runs all tasks on the tokio async runtime. This works for compute-bound tasks but causes problems for:
- Tasks wrapping blocking I/O (database queries, synchronous SDK calls) — these block a tokio worker thread, starving other tasks
- Tasks with strict latency requirements that need guaranteed thread access without sharing with other async work

### Execution Modes

| Mode | Runtime behavior | Suitable for |
|------|-----------------|--------------|
| `Inline` | Runs directly on the scheduler's tokio runtime as an async task | Fast, non-blocking tasks (math, in-memory state updates, context manipulation) |
| `SpawnBlocking` | Offloaded to tokio's blocking thread pool via `tokio::task::spawn_blocking` | Tasks wrapping synchronous I/O (database queries, HTTP calls to synchronous services, file I/O) |
| `Dedicated` | Spawned on a dedicated OS thread (not part of the tokio thread pool) | Tasks requiring guaranteed thread access, tasks with their own event loop, tasks calling non-Send FFI |

### Declaration

Execution mode is declared at task definition time, not at runtime:

**Via macro attribute:**
```rust
#[continuous_task(sources = ["source_alpha"], execution_mode = "spawn_blocking")]
async fn my_materializer(ctx: Context) -> ContinuousTaskResult {
    // ...
}
```

**Via manifest field:**
```yaml
continuous_tasks:
  - name: "my_materializer"
    function: "materialize"
    sources: ["source_alpha"]
    execution_mode: "spawn_blocking"
```

Default is `Inline` if not specified.

### Scheduler Integration

The scheduler's task firing step (step 5 in CLOACI-S-0001 scheduler tick) dispatches based on execution mode:

- **Inline**: `tokio::spawn(task.execute(context))`
- **SpawnBlocking**: `tokio::task::spawn_blocking(move || { runtime.block_on(task.execute(context)) })`
- **Dedicated**: `std::thread::spawn(move || { runtime.block_on(task.execute(context)) })` with a oneshot channel for the result

All three modes return the result asynchronously to the scheduler via the ExecutionLedger (TaskCompleted/TaskFailed event). The scheduler never blocks on task completion.

### Thread Pool Sizing

- **Inline**: Uses the shared tokio runtime (sized by `DefaultRunnerConfig`)
- **SpawnBlocking**: Uses tokio's default blocking thread pool (512 threads, configurable via `TOKIO_BLOCKING_THREADS`)
- **Dedicated**: Creates a new OS thread per execution. Use sparingly — suitable for long-lived tasks or FFI calls, not for high-frequency firings.

## Checkpoints

### Purpose

Checkpoints allow continuous tasks to persist cumulative state across executions. Unlike boundaries (which describe what changed in the data source), checkpoints represent the task's own internal state that grows over time.

Example: A gamma_materializer tracking cumulative exposure. Each execution receives a fill event, updates `exposure += filled_qty` or `exposure -= filled_qty`, and checkpoints the new total. On restart, the checkpoint is restored so the materializer doesn't lose its cumulative position.

### API

```rust
// In ContinuousTaskResult::Fire or ::NoFire
checkpoint: Option<serde_json::Value>

// In the task's input context
ctx.get("__checkpoint") -> Option<serde_json::Value>
```

Tasks read their previous checkpoint from `__checkpoint` in the input context and return an updated checkpoint in their result. The scheduler persists checkpoints to the DAL.

### Persistence

Checkpoints are stored in a `task_checkpoints` table:

| Column | Type | Description |
|--------|------|-------------|
| `task_id` | `TEXT` | Continuous task identifier |
| `checkpoint` | `JSONB` | Serialized checkpoint value |
| `updated_at` | `TIMESTAMP` | When the checkpoint was last written |

One row per task. Updated on every execution that returns a checkpoint (both Fire and NoFire).

### Recovery

On scheduler restart, `restore_from_persisted_state()` loads the latest checkpoint for each continuous task and includes it in the first execution's context as `__checkpoint`. If no checkpoint exists (first-ever execution), `__checkpoint` is absent from the context.

### Design Decisions

- **Checkpoints are JSON (serde_json::Value)**, not typed. This keeps the checkpoint store generic — the scheduler doesn't need to know the shape of each task's state.
- **Checkpoints persist on NoFire.** A task that updates internal state but suppresses downstream propagation still needs to persist its state. Otherwise, a crash after NoFire would lose the state update.
- **One checkpoint per task, not per execution.** Checkpoints are overwritten on each execution. Historical checkpoints are not retained — this is a current-state store, not a journal. The ExecutionLedger provides the journal if audit is needed.

## Macro Support

### `#[continuous_task]` Attribute

The `#[continuous_task]` proc macro generates the boilerplate for registering a function as a continuous task:

```rust
#[continuous_task(
    sources = ["source_alpha", "source_beta"],  // triggering input edges
    referenced = ["config_source"],              // read-only side edges
    execution_mode = "inline",                   // optional, default "inline"
)]
async fn my_decision_engine(ctx: Context) -> ContinuousTaskResult {
    let boundaries = ctx.get("__boundaries");
    let checkpoint = ctx.get("__checkpoint");
    // ... compute ...
    ContinuousTaskResult::Fire {
        context: output_ctx,
        checkpoint: Some(json!({ "last_output": output })),
    }
}
```

The macro generates:
- A `ContinuousTaskRegistration` with the declared sources, referenced edges, and execution mode
- Registration into the global continuous task registry (similar to how `#[task]` registers into the task registry)
- The function signature is validated at compile time: must accept `Context` and return `ContinuousTaskResult`

### Python Support

Python continuous tasks use the same pattern but through PyO3 bindings:

```python
@continuous_task(sources=["source_alpha"], execution_mode="spawn_blocking")
def my_materializer(ctx):
    boundary = ctx["__boundaries"]["source_alpha"]
    checkpoint = ctx.get("__checkpoint", {})
    # ... compute ...
    return ContinuousTaskResult.fire(
        context={"output": value},
        checkpoint={"cumulative": new_total}
    )
```

Python tasks always use `SpawnBlocking` mode (regardless of declaration) because the GIL makes `Inline` mode counterproductive — a Python task holding the GIL would block the async runtime.

## Constraints

### Technical Constraints

- Context injection must be complete before task execution begins — no lazy loading or deferred reads. The task receives everything it needs in a single Context.
- NoFire must be a distinct return type, not inferrable from context state. The scheduler must be able to determine propagation behavior from the return value alone, without inspecting the output context.
- Checkpoint writes must be durable before the scheduler considers the execution complete. A crash between task completion and checkpoint write would lose state.
- `Dedicated` execution mode creates a new OS thread per firing. Tasks using this mode should not fire at high frequency (>10/sec) or thread creation overhead will dominate execution time.
- Python continuous tasks must always use `SpawnBlocking` regardless of declared mode, due to GIL constraints.
