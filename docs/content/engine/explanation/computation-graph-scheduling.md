---
title: "Graph Scheduling"
description: "How the graph scheduler manages computation graph lifecycles, accumulator supervision, and health-driven execution"
weight: 50
aliases:
  - "/computation-graphs/explanation/computation-graph-scheduling/"

---

# Graph Scheduling Architecture

The graph scheduler is the counterpart to the workflow scheduler. While the workflow scheduler polls the database for ready tasks and fires them on a timer, the graph scheduler is **event-driven** — it manages long-lived processes and fires computation graphs when data arrives.

Key terms:
- **Accumulator** — a long-lived process that ingests events from an external source (Kafka, WebSocket, database poll) and emits typed data snapshots called *boundaries*
- **Reactor** — the execution engine that watches accumulators, evaluates criteria, and calls the compiled graph function when conditions are met
- **Boundary** — a typed data snapshot emitted by an accumulator, signaling that new input is available
- **Input cache** — an in-memory store holding the latest (or queued) boundary from each accumulator, providing the graph function's input
- **Dirty flags** — per-accumulator booleans indicating whether new data has arrived since the last graph execution
- **DAL** — database access layer, used for checkpoint persistence and crash recovery

## Two Scheduling Models

Cloacina offers two scheduling models for different workload shapes:

| Aspect | Workflow Scheduler | Graph Scheduler |
|--------|-------------------|--------------------|
| **Trigger** | Timer (cron), event trigger polls, or API call | Data arrival from accumulators |
| **Execution** | One-shot: schedule → run tasks → complete | Continuous: graph fires repeatedly as data flows |
| **Lifetime** | Short (seconds to hours per execution) | Long (runs indefinitely until shutdown) |
| **State** | Database-backed context between tasks | In-memory input cache, checkpoint-backed recovery |
| **Scaling** | Multiple runners claim tasks from database | One scheduler supervises each graph; each firing runs in-process or dispatches to the execution-agent fleet |
| **Use case** | ETL, batch jobs, scheduled reports | Streaming analytics, real-time pricing, monitoring |

Choose workflows when your workload has a clear start and end. Choose computation graphs when you need continuous, low-latency reaction to incoming data.

## Architecture

The graph scheduler manages three layers:

```text
┌─────────────────────────────────────────────────────┐
│                 Graph Scheduler                    │
│                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │ Accumulator  │  │ Accumulator  │  │ Accumulator  │  │
│  │ (orderbook)  │  │ (pricing)    │  │ (config)     │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │          │
│         └────────┬────────┘                 │          │
│                  ▼                          │          │
│          ┌──────────────┐                   │          │
│          │   Reactor     │◄─────────────────┘          │
│          │              │                              │
│          │  dirty flags │                              │
│          │  input cache │                              │
│          │  graph_fn()  │                              │
│          └──────────────┘                              │
└─────────────────────────────────────────────────────┘
```

### Declarations

The scheduler receives `ComputationGraphDeclaration` structs from the reconciler (for packaged graphs) or from code registration (for embedded graphs). Each declaration specifies:

- **Graph name** — unique identifier
- **Accumulator declarations** — one per data source, with a factory for spawning
- **Reactor declaration** — reaction criteria, input strategy, and the compiled graph function
- **Tenant ID** — optional, for multi-tenant isolation

### Spawn Sequence

When a declaration arrives:

1. Create boundary channel (accumulators → reactor)
2. Create shutdown channel (scheduler → all tasks)
3. For each accumulator:
   - Call the factory's `spawn()` method
   - Record the socket sender (for WebSocket endpoints) and join handle
   - Wire health monitoring
4. Create the reactor with boundary receiver, criteria, strategy, and graph function
5. Register WebSocket endpoints in the endpoint registry
6. Spawn all tasks on the tokio runtime
7. Track the `ManagedGraph` for lifecycle management

## Accumulator Lifecycle

Each accumulator runs as an independent tokio task with two input paths:

- **Socket receiver** — external events pushed in via WebSocket or API (always active)
- **Event source** (optional) — active pull from an external system (Kafka, database, polling)

Both paths merge into a single channel processed by the accumulator's `process()` method, which transforms raw events into typed boundaries and pushes them to the reactor.

### Health States

| State | Meaning |
|-------|---------|
| `Starting` | Loading checkpoint from DAL |
| `Connecting` | Checkpoint loaded, connecting to source |
| `Live` | Connected, processing events normally |
| `Disconnected` | Lost source connection, retrying |
| `SocketOnly` | Passthrough accumulator — healthy by definition |

The reactor watches accumulator health to gate its own startup and detect degradation.

## Reactor Lifecycle

The reactor is the execution engine — it decides *when* to fire the graph and hands each firing off to be run.

### Three Concerns

1. **Receiver** — accepts serialized boundaries from accumulators, deserializes into the input cache, sets dirty flags
2. **Strategy** — evaluates reaction criteria (`WhenAny` or `WhenAll`) against dirty flags to decide whether to fire
3. **Executor** — hands a snapshot of the input cache to a `GraphExecutor`, which runs the firing and returns the result the reactor awaits

### Where a firing runs

The reactor does not hard-code *where* the compute happens. When it decides to fire, it packages the input-cache snapshot into a firing event and hands it to a `GraphExecutor` — a seam that mirrors the task side's executor. Two implementations sit behind it:

- **In-process (embedded, and the default)** — the executor invokes the graph's compiled closure directly in this process. This is byte-for-byte the original behavior: lowest latency, no network hop, no external dependency. Every firing event carries this closure regardless of which executor is installed.
- **Fleet dispatch (server mode)** — `cloacina-server` installs a fleet executor that resolves the graph's owning package and ships the firing (the input-cache snapshot plus the package's artifact digest) to a live execution agent, then awaits the result over the same agent rendezvous the task path uses. This lets whole-graph compute scale across a fleet of Rust and Python agents while accumulators, dirty flags, reactor state, and the input cache all stay host-side — only the compute leaves.

Because every firing event carries the in-process closure, the fleet executor can always fall back to local execution rather than wedge the reactor's hot path. It falls back *before* dispatch — an embedded graph with no owning package, a Python package (interpreted graphs stay in-process for now), no eligible agent, or an enqueue error — running the closure locally with a warning. It deliberately does **not** fall back *after* dispatch: once an agent has accepted a firing, a timeout or a reported error surfaces as a failed result rather than a local re-run, because double-running one firing is worse than reporting it failed.

### Health States

| State | Meaning | Behavior |
|-------|---------|----------|
| `Starting` | Loading cache from DAL, spawning accumulators | Not processing events |
| `Warming` | Some accumulators healthy, waiting for others | Receiving but not firing |
| `Live` | All accumulators healthy | Evaluating criteria, firing on match |
| `Degraded` | An accumulator disconnected after being live | Firing with stale data, flagging degradation |

The `Warming → Live` transition requires all expected accumulators to report a healthy state. This prevents firing the graph with incomplete data on startup.

### Reaction Criteria

| Criteria | Fires When | Best For |
|----------|-----------|----------|
| `WhenAny` | Any accumulator has new data | Low-latency reaction, independent sources |
| `WhenAll` | Every accumulator has new data since last fire | Correlated data, ensuring complete input |

With `WhenAll`, the reactor pre-seeds dirty flags for all expected sources. This ensures `all_set()` returns false until every source has emitted at least once — not just the sources seen so far.

### Input Strategy

| Strategy | Cache Behavior | Guarantee |
|----------|---------------|-----------|
| `Latest` | Overwrites previous boundary per source | Graph always sees freshest data; intermediates may be skipped |
| `Sequential` | Queues every boundary in arrival order | Every event processed; no skipping |

See [Using Sequential Input Strategy]({{< ref "/engine/computation-graphs/how-to/sequential-strategy" >}}) for guidance on choosing between them.

## Supervision and Restart

The graph scheduler monitors spawned tasks and restarts them on failure with exponential backoff.

### Accumulator Restart

If an individual accumulator task panics or exits, the scheduler restarts **only that accumulator** in-place — the reactor and other accumulators continue running. The reactor transitions to `Degraded` state until the restarted accumulator reports healthy again.

### Full Graph Restart

If the reactor itself panics, the scheduler tears down the entire graph (all accumulators + reactor) and respawns from the declaration with fresh channels. The input cache is recovered from the DAL checkpoint if available.

### Backoff and Limits

| Constant | Value | Purpose |
|----------|-------|---------|
| Max recovery attempts | 5 | After 5 consecutive failures, the graph is marked as failed |
| Backoff base | 1 second | Initial delay before restart |
| Backoff max | 60 seconds | Maximum delay (exponential growth capped here) |
| Success reset | 60 seconds | After running successfully for 60s, the failure counter resets to 0 |

Recovery events are recorded to the DAL for observability, allowing operators to query restart history.

On graceful shutdown, the shutdown channel signals all tasks to drain and exit.

## Reactor Commands

The reactor accepts manual commands via a channel, exposed through WebSocket endpoints in server mode:

| Command | Behavior |
|---------|----------|
| `ForceFire` | Fire the graph immediately with the current cache state, regardless of reaction criteria |
| `FireWith(cache)` | Fire with an injected cache, replacing the current state |
| `GetState` | Return the current cache and dirty flag state |
| `Pause` | Stop evaluating reaction criteria (boundaries still accumulate) |
| `Resume` | Resume evaluating reaction criteria |

These commands are useful for debugging, manual intervention, and testing in production.

## Crash Recovery

When configured with a DAL (database access layer):

1. **Accumulator checkpoints** — accumulators can persist their last-processed offset via `CheckpointHandle`. On restart, `init()` loads the checkpoint to resume from the last known position.
2. **Input cache persistence** — the reactor periodically snapshots the input cache to the DAL. On restart, the cache is restored so the graph doesn't start from scratch.

Without a DAL (embedded mode), state is lost on restart — accumulators and the cache start fresh.

## Comparison with Workflow Cron Scheduling

| Feature | Cron Scheduling | Graph Scheduling |
|---------|----------------|---------------------|
| Minimum latency | Poll interval (typically seconds) | Event-driven (sub-millisecond in-process; a network hop when a firing dispatches to the fleet) |
| Missed execution handling | Catch-up on restart | N/A — continuous processing |
| Multi-runner support | Yes (database-based claiming) | One scheduler supervises each graph, but firings can execute on an execution-agent fleet |
| Guaranteed execution | Two-phase commit with recovery | Checkpoint-based recovery |
| Database requirement | Always (execution state) | Optional (checkpoint persistence) |

## See Also

- [Computation Graph Architecture]({{< ref "/engine/explanation/architecture" >}}) — graph execution model
- [Accumulator Design]({{< ref "/engine/explanation/accumulator-design" >}}) — how accumulators work
- [Cron Scheduling Architecture]({{< ref "/engine/explanation/cron-scheduling" >}}) — time-based workflow scheduling
- [Computation Graph Reference]({{< ref "/reference/computation-graphs" >}}) — API reference
