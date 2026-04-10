---
id: reactor
level: specification
title: "Reactor"
short_code: "CLOACI-S-0005"
created_at: 2026-04-04T16:18:12.963279+00:00
updated_at: 2026-04-04T16:18:12.963279+00:00
parent: CLOACI-I-0069
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Reactor

## Overview

A reactor is a long-lived process that wires accumulators to a compiled computation graph. It has three concerns:

1. **Receiver**: accepts boundaries from accumulators, updates the input cache
2. **Strategy**: evaluates reaction criteria and input strategy to decide when and what to execute
3. **Executor**: calls the compiled graph function and handles the result

The reactor's state is minimal: the input cache (last seen value per source) and dirty flags (one boolean per source). This state is persisted to the DAL for fast recovery — on restart the reactor loads its last cache snapshot and is immediately operational. Accumulators manage their own state independently; the reactor does not coordinate accumulator checkpoints or offsets.

The reactor also registers as a named endpoint on the API server for manual operations (force-fire, inject state, status queries).

### Relationship to other specs

- **CLOACI-S-0004** (Accumulator Trait) — defines the accumulators that feed the reactor
- **CLOACI-I-0069** (parent initiative) — defines the overall architecture, reaction criteria, input strategy, and recovery model
- Node Execution Model spec (forthcoming) — defines what the compiled graph function returns
- Computation Graph Macro spec (forthcoming) — defines how the compiled graph function is generated

## Three Concerns

### Receiver

The receiver runs as a separate task from the executor. It continuously reads from the accumulator channel and updates the cache. It never blocks on graph execution.

```rust
// Receiver task — always running, never blocked by execution
async fn receiver_loop(
    accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,
    cache: Arc<RwLock<InputCache>>,
    dirty: Arc<RwLock<DirtyFlags>>,
    manual_rx: mpsc::Receiver<ManualCommand>,
    strategy_tx: mpsc::Sender<StrategySignal>,
) {
    loop {
        tokio::select! {
            Some((source, bytes)) = accumulator_rx.recv() => {
                cache.write().update(source.clone(), bytes);
                dirty.write().set(source, true);
                strategy_tx.send(StrategySignal::BoundaryReceived).await.ok();
            }
            Some(cmd) = manual_rx.recv() => {
                match cmd {
                    ManualCommand::ForceFire => {
                        strategy_tx.send(StrategySignal::ForceFire).await.ok();
                    }
                    ManualCommand::FireWith(state) => {
                        cache.write().replace_all(state);
                        strategy_tx.send(StrategySignal::ForceFire).await.ok();
                    }
                }
            }
            // shutdown handled by channel close
        }
    }
}
```

The receiver and executor communicate through a strategy channel. The receiver sends signals ("boundary received", "force fire"). The executor reads signals and decides what to do. This decouples reception from execution — the cache always has the latest data regardless of whether the graph is currently running.

### Strategy

The strategy decides when to fire the graph based on reaction criteria and input strategy. It sits between the receiver and executor.

**Reaction criteria** (when to fire):
- **`when_any`**: fire if any dirty flag is set
- **`when_all`**: fire if all dirty flags are set

**Input strategy** (what the executor sees):
- **`latest`**: executor gets a snapshot of the current cache. Dirty flags cleared after snapshot. Intermediate updates during execution are collapsed — the cache keeps updating, next execution sees the freshest values.
- **`sequential`**: executor gets one boundary at a time. Each boundary triggers one execution. No collapsing.

```rust
// Strategy evaluation — called when receiver signals
fn should_fire(criteria: &ReactionCriteria, dirty: &DirtyFlags) -> bool {
    match criteria {
        ReactionCriteria::WhenAny => dirty.any_set(),
        ReactionCriteria::WhenAll => dirty.all_set(),
    }
}
```

### Executor

The executor receives "fire" decisions from the strategy, takes a snapshot of the cache, calls the compiled graph function, and handles the result. After execution completes, it signals batch accumulators to flush.

```rust
// Executor task
async fn executor_loop(
    strategy_rx: mpsc::Receiver<StrategySignal>,
    cache: Arc<RwLock<InputCache>>,
    dirty: Arc<RwLock<DirtyFlags>>,
    graph: CompiledGraph,
    batch_flush: broadcast::Sender<()>,
    criteria: ReactionCriteria,
) {
    while let Some(signal) = strategy_rx.recv().await {
        let should_run = match signal {
            StrategySignal::BoundaryReceived => should_fire(&criteria, &dirty.read()),
            StrategySignal::ForceFire => true,
        };

        if should_run {
            // Snapshot the cache and clear dirty flags
            let snapshot = cache.read().snapshot();
            dirty.write().clear_all();

            // Execute the compiled graph
            let result = graph.execute(&snapshot).await;

            match result {
                Ok(_) => {
                    // Signal batch accumulators to flush
                    batch_flush.send(()).ok();
                }
                Err(e) => {
                    // Graph execution failed — log, emit metrics
                    // Dirty flags already cleared — next boundary triggers re-execution
                    // with fresh data, not a retry of stale state
                    log::error!("graph execution failed: {}", e);
                }
            }
        }
    }
}
```

## Runtime Process

The reactor spawns two tasks: receiver and executor. They share the cache and dirty flags via `Arc<RwLock<>>`. Communication flows one way: receiver → strategy channel → executor.

```
Accumulators ──→ mpsc ──→ [ Receiver task ]
                                │
                          strategy channel
                                │
                          [ Executor task ] ──→ graph.execute()
                                │
Manual channel ──→ [ Receiver ] batch_flush ──→ Batch accumulators
                                │
API server registration ────────┘
```

```rust
struct Reactor {
    /// Compiled computation graph — the function to call
    graph: CompiledGraph,

    /// Reaction criteria — when_any or when_all
    criteria: ReactionCriteria,

    /// Input strategy — latest or sequential
    input_strategy: InputStrategy,

    /// Channel receiving boundaries from all accumulators
    accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,

    /// Channel for manual operations (force-fire, inject state)
    manual_rx: mpsc::Receiver<ManualCommand>,

    /// Broadcast channel to signal batch accumulators to flush
    batch_flush: broadcast::Sender<()>,

    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl Reactor {
    async fn run(self) {
        let cache = Arc::new(RwLock::new(InputCache::new()));
        let dirty = Arc::new(RwLock::new(DirtyFlags::new()));

        let (strategy_tx, strategy_rx) = mpsc::channel(64);

        // Spawn receiver
        let receiver = tokio::spawn(receiver_loop(
            self.accumulator_rx,
            cache.clone(),
            dirty.clone(),
            self.manual_rx,
            strategy_tx,
        ));

        // Spawn executor
        let executor = tokio::spawn(executor_loop(
            strategy_rx,
            cache,
            dirty,
            self.graph,
            self.batch_flush,
            self.criteria,
        ));

        // Wait for shutdown
        self.shutdown.changed().await.ok();
        receiver.abort();
        executor.abort();
    }
}
```

## Input Cache

The cache holds the last seen boundary per source. It's a typed map — each entry is a source name and serialized boundary bytes (deserialized by the compiled graph function which knows the types).

```rust
struct InputCache {
    entries: HashMap<SourceName, Vec<u8>>,
}

impl InputCache {
    fn update(&mut self, source: SourceName, bytes: Vec<u8>) {
        self.entries.insert(source, bytes);
    }

    fn snapshot(&self) -> InputCache {
        self.clone()
    }
}
```

For `latest` input strategy: the cache is overwritten on every boundary. The executor takes a snapshot before execution. New boundaries arriving during execution update the cache but don't affect the running execution.

For `sequential` input strategy: the strategy layer queues individual boundaries and fires one execution per boundary. The cache still holds last-seen values for all sources, but the "triggering" boundary is the one that caused this specific execution.

## Dirty Flags

One boolean per source. Set when a boundary arrives. Cleared when the executor takes a snapshot.

```rust
struct DirtyFlags {
    flags: HashMap<SourceName, bool>,
}

impl DirtyFlags {
    fn set(&mut self, source: SourceName, dirty: bool) { self.flags.insert(source, dirty); }
    fn any_set(&self) -> bool { self.flags.values().any(|&v| v) }
    fn all_set(&self) -> bool { !self.flags.is_empty() && self.flags.values().all(|&v| v) }
    fn clear_all(&mut self) { for v in self.flags.values_mut() { *v = false; } }
}
```

## Manual Channel

The reactor accepts manual commands via a channel, exposed through the API server's WebSocket endpoint.

```rust
enum ManualCommand {
    /// Fire the graph with current cache state
    ForceFire,
    /// Fire the graph with injected state (replaces cache)
    FireWith(InputCache),
    /// Return current cache state (for inspection/debugging)
    GetState(oneshot::Sender<InputCache>),
    /// Pause/resume execution (receiver keeps updating cache)
    Pause,
    Resume,
}
```

Manual operations go through the receiver, which forwards to the strategy channel. The executor treats a force-fire the same as a criteria-met signal — it snapshots the cache and executes.

## Batch Accumulator Flush

After each graph execution completes successfully, the reactor sends a flush signal on a `broadcast::Sender<()>`. Batch accumulators subscribe to this channel and drain their sources when signaled.

This is the reactor's only outbound communication to accumulators. All other accumulator classes ignore it.

```rust
// In executor, after successful execution:
batch_flush.send(()).ok();
```

## API Server Registration

The reactor registers as a named endpoint on the API server on startup:

```rust
struct ReactorRegistration {
    name: String,           // reactor name, used for routing
    graph_name: String,     // computation graph this reactor runs
    sources: Vec<String>,   // accumulator names this reactor consumes
    criteria: String,       // "when_any" or "when_all"
    input_strategy: String, // "latest" or "sequential"
    health: ReactorHealth,  // current health state (starting/warming/live/degraded)
}

enum ReactorHealth {
    Starting,
    Warming { healthy: Vec<String>, waiting: Vec<String> },
    Live,
    Degraded { disconnected: Vec<String> },
}
```

Operators interact with the reactor through the API server WebSocket:
- **Force-fire**: trigger execution with current state
- **Fire-with**: trigger execution with injected state
- **Get state**: inspect current cache contents
- **Pause/resume**: stop/start execution while receiver keeps updating

## Persistence

The reactor persists its cache and dirty flags to the DAL for fast recovery. Without this, a restart means waiting for all accumulators to re-emit — polling and batch accumulators might not emit for seconds or minutes.

**What's persisted:**
- Cache snapshot: last known value per source (`HashMap<SourceName, Vec<u8>>`)
- Dirty flags: which sources had pending updates at persist time

**When it's persisted:**
- After each graph execution completes (cache snapshot + cleared dirty flags)
- Periodically during idle (if cache has been updated but criteria not yet met)

```rust
struct ReactorState {
    cache: HashMap<SourceName, Vec<u8>>,
    dirty: HashMap<SourceName, bool>,
    last_persisted: Timestamp,
}
```

The DAL handle is provided to the reactor at construction. Persistence uses simple key-value storage keyed by reactor name.

## Startup & Health

The reactor does not fire graphs until all its accumulators are healthy. Partial cache state should not feed into the graph on startup.

**Startup sequence:**
1. Reactor loads cache snapshot and dirty flags from DAL → has last known state
2. Reactor spawns all its accumulators
3. Accumulators restore from checkpoints, connect to sources
4. Each accumulator signals "healthy" when connected and has emitted at least one boundary
5. Reactor waits for all accumulators healthy
6. All healthy → reactor enters **live** state, starts evaluating reaction criteria

**Health states:**

| State | Meaning | Executes? |
|---|---|---|
| **Starting** | Loading cache from DAL, spawning accumulators | No |
| **Warming** | Some accumulators healthy, waiting for all | No |
| **Live** | All accumulators healthy, evaluating criteria | Yes |
| **Degraded** | Was live, an accumulator disconnected. Running with stale data for that source. | Yes (with stale data) |

The **degraded** state is key for resilience — if a broker goes down mid-operation, the reactor doesn't stop. It keeps running with the last known value for the disconnected source. It reports degraded status via API server registration so operators know. When the accumulator reconnects and re-emits, the reactor returns to live.

## Recovery

On restart, the reactor follows the startup sequence above:

1. Load cache from DAL (instant)
2. Spawn accumulators, wait for all healthy
3. Once live, first execution uses fully fresh data from all accumulators

Recovery time is bounded by how fast accumulators can reconnect and emit their first boundary. The reactor itself loads from the DAL instantly — the wait is for accumulators to go healthy. For `latest` input strategy: stale DAL data is replaced the moment a fresh boundary arrives from each accumulator.

## Constraints

### Technical Constraints

- The receiver and executor run as separate tasks. The cache uses `Arc<RwLock<>>` — writes are fast (insert into HashMap), reads take a snapshot (clone). The RwLock is held briefly in both cases.
- The strategy channel between receiver and executor is bounded. If the executor falls behind (slow graph execution), the receiver's strategy sends may block. This provides natural backpressure — the cache keeps updating (it's a separate write), but the executor won't queue up unbounded work.
- The reactor persists its cache to the DAL after each execution and periodically during idle. On crash, the reactor restores from the last persist point and is immediately operational with stale-but-valid data. Fresh data arrives as accumulators reconnect.
- Batch flush is best-effort — `broadcast::send(()).ok()`. If a batch accumulator is slow to process the previous flush, it may miss a signal. This is acceptable — the next flush will catch it up.
