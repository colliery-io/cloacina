---
title: "Computation Graph Reference"
description: "Complete reference for computation graph macros, types, and APIs"
weight: 35
---

# Computation Graph Reference

Computation graphs are Cloacina's reactive data processing primitive. A graph defines a DAG of async node functions that execute when upstream data arrives. The `#[computation_graph]` macro compiles a module of node functions into a single async function that the reactor calls on each trigger.

```rust
use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, Accumulator,
    AccumulatorContext, AccumulatorRuntimeConfig, BoundarySender,
};
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
```

---

## #[computation_graph] Macro

The `#[computation_graph]` attribute macro is applied to a module containing async node functions. It declares graph topology and reaction criteria, validates the graph at compile time, and generates a compiled async function.

### Full Syntax

```rust
#[cloacina_macros::computation_graph(
    react = when_any(source1, source2),
    graph = {
        entry_node(source1, source2) -> next_node,
        next_node -> terminal_node,
    }
)]
pub mod my_graph {
    // Node functions here
}
```

### `react` Attribute

The `react` attribute declares which accumulator sources trigger graph execution and the criteria for firing.

| Mode | Syntax | Behavior |
|------|--------|----------|
| `when_any` | `react = when_any(alpha, beta)` | Fire when **any** listed source has new data |
| `when_all` | `react = when_all(alpha, beta)` | Fire only when **all** listed sources have new data |

The names listed in `react` must match the source names used in entry node parenthesized inputs.

### `graph` Attribute

The `graph` attribute declares the topology using a DSL inside braces. Each line declares edges between nodes.

#### Linear Edges

```rust
graph = {
    entry(source_name) -> downstream_node,
    downstream_node -> terminal_node,
}
```

- `entry(source_name)` -- parenthesized names are cache inputs read from `InputCache`
- `->` -- linear connection; output of left node feeds as input to right node
- Nodes with no outgoing edges are automatically detected as terminal nodes

#### Routing Edges (Enum Dispatch)

```rust
graph = {
    decision(alpha, beta) => {
        Trade -> signal_handler,
        NoAction -> audit_logger,
    },
}
```

- `=>` -- routing connection; the node returns a Rust enum
- Each `Variant -> target` maps an enum variant to a downstream node
- The variant's inner data is passed to the target node
- Each routing branch can have its own terminal nodes

#### Fan-Out (One Node to Many)

```rust
graph = {
    compute(source) -> output_handler,
    compute(source) -> audit_logger,
}
```

A node can appear as the source in multiple edge declarations. Its output is cloned to all downstream nodes.

#### Fan-In (Many Nodes to One)

```rust
graph = {
    validate_a(a) -> merge,
    validate_b(b) -> merge,
}
```

Multiple nodes can feed into the same downstream node. The downstream node receives all upstream outputs as separate arguments.

#### Diamond Graphs

```rust
graph = {
    entry(source) -> branch_a,
    entry(source) -> branch_b,
    branch_a -> merge,
    branch_b -> merge,
}
```

Fan-out and fan-in can be combined. Topological sort guarantees correct execution order.

### Generated Function

The macro generates a compiled async function named `{module_name}_compiled`:

```rust
pub async fn my_graph_compiled(
    cache: &InputCache,
) -> GraphResult
```

This function executes all nodes in topological order and returns terminal outputs in `GraphResult::Completed { outputs }`.

### Generated Registration

In embedded mode (not `feature = "packaged"`), the macro generates a `#[ctor]` function that registers the graph in the global registry at program start:

```rust
#[ctor::ctor]
fn _auto_register_graph_my_graph() {
    register_computation_graph_constructor(
        "my_graph".to_string(),
        || ComputationGraphRegistration {
            graph_fn: Arc::new(|cache| Box::pin(async move {
                my_graph_compiled(&cache).await
            })),
            accumulator_names: vec!["source1".to_string(), "source2".to_string()],
            reaction_mode: "when_any".to_string(),
        },
    );
}
```

### Complete Example

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot { pub best_bid: f64, pub best_ask: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadSignal { pub spread: f64, pub mid_price: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedOutput { pub message: String }

#[cloacina_macros::computation_graph(
    react = when_any(orderbook),
    graph = {
        ingest(orderbook) -> compute_spread,
        compute_spread -> format_output,
    }
)]
pub mod pricing_pipeline {
    use super::*;

    pub async fn ingest(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal {
        let book = orderbook.expect("orderbook should be present");
        SpreadSignal {
            spread: book.best_ask - book.best_bid,
            mid_price: (book.best_ask + book.best_bid) / 2.0,
        }
    }

    pub async fn compute_spread(input: &SpreadSignal) -> SpreadSignal {
        let spread_bps = (input.spread / input.mid_price) * 10_000.0;
        SpreadSignal { spread: spread_bps, mid_price: input.mid_price }
    }

    pub async fn format_output(input: &SpreadSignal) -> FormattedOutput {
        FormattedOutput {
            message: format!("Mid: {:.2}, Spread: {:.1} bps", input.mid_price, input.spread),
        }
    }
}

// Call the generated compiled function:
let mut cache = InputCache::new();
cache.update(SourceName::new("orderbook"), serialize(&my_book).unwrap());
let result = pricing_pipeline_compiled(&cache).await;
```

---

## Node Functions

Every function in a `#[computation_graph]` module is a node. All node functions must be `pub async fn`. The macro validates that every function in the module appears in the graph topology and every node in the topology has a corresponding function.

### Entry Node Signature

Entry nodes read directly from the `InputCache`. Cache inputs are declared in the topology with parenthesized source names. Each cache input becomes an `Option<&T>` parameter:

```rust
pub async fn entry_node(
    source_a: Option<&MyTypeA>,
    source_b: Option<&MyTypeB>,
) -> OutputType {
    // ...
}
```

The `Option` is `None` when the source has no data in the cache yet. Entry nodes have no incoming edges from other nodes.

### Interior Node Signature

Interior nodes receive the output of their upstream node(s) as `&T` references:

```rust
pub async fn process(input: &UpstreamOutput) -> MyOutput {
    // ...
}
```

If the node has both cache inputs and upstream inputs, cache inputs come first:

```rust
pub async fn enriched_process(
    config: Option<&ConfigData>,   // cache input
    input: &UpstreamOutput,        // from upstream node
) -> MyOutput {
    // ...
}
```

### Terminal Node Signature

Terminal nodes have the same signature as interior nodes. They are identified automatically as nodes with no outgoing edges. Their return value is collected into `GraphResult::Completed { outputs }`:

```rust
pub async fn final_output(input: &ProcessedData) -> FinalResult {
    FinalResult { /* ... */ }
}
```

### Routing Node Signature

A routing node returns a Rust enum. Each variant carries data that is passed to the corresponding downstream node:

```rust
#[derive(Debug, Clone)]
pub enum DecisionOutcome {
    Trade(TradeSignal),
    NoAction(NoActionReason),
}

pub async fn decision(
    orderbook: Option<&OrderBookData>,
    pricing: Option<&PricingData>,
) -> DecisionOutcome {
    // Return the appropriate variant
}
```

### Blocking Nodes

Annotate a node with `#[node(blocking)]` to run it on `spawn_blocking`. Use this for CPU-intensive work that would block the async runtime. The async runtime uses a small pool of worker threads. CPU-intensive synchronous code on these threads blocks other async tasks. `#[node(blocking)]` moves the node to a separate blocking thread pool, keeping the async runtime responsive.

```rust
#[node(blocking)]
pub async fn heavy_computation(input: &LargeDataSet) -> ProcessedResult {
    // CPU-bound work runs on the blocking thread pool
}
```

### Type Constraints

All types flowing through a computation graph must satisfy:

- `Serialize + Deserialize` -- for cache storage and wire format
- `Send + Sync + 'static` -- for async runtime compatibility

Routing enum types do not need `Serialize`/`Deserialize` since they are only used within a single graph execution (not persisted to the cache).

---

## Core Types

### InputCache

The input cache holds the last-seen serialized value per accumulator source. The reactor updates it continuously; the compiled graph function receives a snapshot.

```rust
use cloacina::computation_graph::types::InputCache;

let mut cache = InputCache::new();

// Update a source with serialized bytes
cache.update(SourceName::new("alpha"), serialize(&my_data).unwrap());

// Get and deserialize a cached value (returns Option<Result<T, GraphError>>)
let value: Option<Result<MyType, _>> = cache.get("alpha");

// Check if a source has data
let has_data: bool = cache.has("alpha");

// Get raw bytes without deserialization
let raw: Option<&[u8]> = cache.get_raw("alpha");

// Create an isolated snapshot (clone)
let snapshot: InputCache = cache.snapshot();

// Query size
let count: usize = cache.len();
let empty: bool = cache.is_empty();

// List all source names
let sources: Vec<&SourceName> = cache.sources();
```

### SourceName

Identifies an accumulator source by name. Used as the key in `InputCache`.

```rust
use cloacina::computation_graph::types::SourceName;

let name = SourceName::new("orderbook");
let name_from_str: SourceName = "orderbook".into();
let name_from_string: SourceName = String::from("orderbook").into();
let s: &str = name.as_str();
```

### GraphResult

The return type of compiled graph functions.

```rust
use cloacina::computation_graph::types::GraphResult;

match result {
    GraphResult::Completed { outputs } => {
        // outputs: Vec<Box<dyn Any + Send>>
        // Downcast to expected types:
        for output in &outputs {
            if let Some(val) = output.downcast_ref::<MyOutputType>() {
                // use val
            }
        }
    }
    GraphResult::Error(e) => {
        eprintln!("Graph failed: {}", e);
    }
}

// Constructors:
let ok = GraphResult::completed(vec![Box::new(42u32) as Box<dyn std::any::Any + Send>]);
let empty = GraphResult::completed_empty();
let err = GraphResult::error(GraphError::MissingInput("alpha".to_string()));

// Predicates:
result.is_completed();
result.is_error();
```

### GraphError

Error variants that can occur during graph execution.

| Variant | Meaning |
|---------|---------|
| `Serialization(String)` | Failed to serialize a value for the cache |
| `Deserialization(String)` | Failed to deserialize bytes from the cache |
| `MissingInput(String)` | A required source was not found in the cache |
| `NodeExecution(String)` | A node function panicked or returned an error |
| `Execution(String)` | General graph execution failure |

### serialize / deserialize

Profile-aware serialization helpers used by the cache and wire format.

```rust
use cloacina::computation_graph::types::{serialize, deserialize};

// Serialize: JSON in debug builds, bincode in release builds
let bytes: Vec<u8> = serialize(&my_value)?;

// Deserialize: matches the serialize format
let value: MyType = deserialize(&bytes)?;
```

This means debug builds produce human-readable JSON (inspectable in logs), while release builds use compact binary (fast, smaller payloads).

---

## Accumulator Types

Accumulators are long-lived processes that consume events from sources and push serialized boundaries to the reactor.

### Accumulator Trait

The core trait for passthrough and event-processing accumulators:

```rust
#[async_trait::async_trait]
pub trait Accumulator: Send + 'static {
    /// The raw event type consumed from the source.
    type Event: DeserializeOwned + Send + 'static;

    /// The typed boundary produced for the reactor.
    type Output: Serialize + Send + 'static;

    /// Process a received event and optionally produce a boundary.
    fn process(&mut self, event: Self::Event) -> Option<Self::Output>;

    /// Optional initialization (restore state from checkpoint).
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}
```

Implement `process()` to transform incoming events into boundaries. Return `None` to suppress output (filtering). Return `Some(output)` to emit a boundary to the reactor.

### Passthrough Accumulator

The simplest accumulator -- events pass through unchanged or with minimal transformation:

```rust
struct PricingAccumulator;

#[async_trait::async_trait]
impl Accumulator for PricingAccumulator {
    type Event = PricingUpdate;
    type Output = PricingSignal;

    fn process(&mut self, event: PricingUpdate) -> Option<PricingSignal> {
        Some(PricingSignal {
            price: event.mid_price,
            change_pct: 0.0,
        })
    }
}
```

#### `#[passthrough_accumulator]` Macro

Generates the struct and trait impl from a plain function:

```rust
#[cloacina_macros::passthrough_accumulator]
fn pricing(event: PricingUpdate) -> PricingSignal {
    PricingSignal { price: event.mid_price, change_pct: 0.0 }
}
// Generates: PricingAccumulator struct implementing Accumulator
```

### PollingAccumulator Trait

For pull-based data sources (databases, APIs) that are polled on a timer:

```rust
#[async_trait::async_trait]
pub trait PollingAccumulator: Send + 'static {
    type Output: Serialize + DeserializeOwned + Send + 'static;

    /// Called on each timer tick. Return Some to emit, None to skip.
    async fn poll(&mut self) -> Option<Self::Output>;

    /// Polling interval.
    fn interval(&self) -> std::time::Duration;
}
```

#### `#[polling_accumulator]` Macro

```rust
#[cloacina_macros::polling_accumulator(interval = "5s")]
async fn check_config() -> Option<ConfigSnapshot> {
    // Query database or API
    Some(ConfigSnapshot { /* ... */ })
}
// Generates: CheckConfigAccumulator struct implementing PollingAccumulator
```

Supported interval suffixes: `ms` (milliseconds), `s` (seconds), `m` (minutes).

### BatchAccumulator Trait

Buffers incoming events and processes them in batches on flush:

```rust
#[async_trait::async_trait]
pub trait BatchAccumulator: Send + 'static {
    type Event: DeserializeOwned + Send + 'static;
    type Output: Serialize + Send + 'static;

    /// Process a batch of events. Called on flush (timer, size, or signal).
    fn process_batch(&mut self, events: Vec<Self::Event>) -> Option<Self::Output>;
}
```

Flush triggers:
- Timer-based `flush_interval`
- Buffer size threshold (`max_buffer_size`)
- Explicit signal from the reactor (after each graph execution)
- Shutdown (drains remaining buffer)

#### `#[batch_accumulator]` Macro

```rust
#[cloacina_macros::batch_accumulator(flush_interval = "10s", max_buffer_size = 100)]
fn aggregate_trades(events: Vec<TradeEvent>) -> Option<TradeSummary> {
    if events.is_empty() { return None; }
    Some(TradeSummary { count: events.len(), /* ... */ })
}
// Generates: AggregateTradesAccumulator struct implementing BatchAccumulator
```

### StreamAccumulator

For accumulators backed by a streaming source (e.g., Kafka):

#### `#[stream_accumulator]` Macro

```rust
#[cloacina_macros::stream_accumulator(type = "kafka", topic = "market.ticks", group = "pricing_group")]
fn market_tick(event: RawTick) -> PricingSignal {
    PricingSignal { price: event.price, change_pct: 0.0 }
}
// Generates: MarketTickAccumulator with stream backend config
```

Arguments:

| Argument | Required | Description |
|----------|----------|-------------|
| `type` | yes | Backend type (e.g., `"kafka"`) |
| `topic` | yes | Stream topic to consume from |
| `group` | no | Consumer group (defaults to `{fn_name}_group`) |
| `state` | no | State type for stateful processing |

### StateAccumulator

A specialized accumulator that maintains a bounded `VecDeque<T>` of historical values, persisted via the DAL:

```rust
pub struct StateAccumulator<T> {
    buffer: VecDeque<T>,
    capacity: i32, // -1 = unbounded
}
```

#### `#[state_accumulator]` Macro

```rust
#[cloacina_macros::state_accumulator(capacity = 10)]
fn previous_outputs() -> VecDeque<DecisionOutput>;
// Generates: PreviousOutputsStateAccumulator with create() and name() methods
```

Use `capacity = -1` for unbounded history.

### EventSource Trait

For accumulators that actively pull events from an external source:

```rust
#[async_trait::async_trait]
pub trait EventSource: Send + 'static {
    type Event: Send + 'static;

    async fn run(
        self,
        events: mpsc::Sender<Self::Event>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<(), AccumulatorError>;
}
```

Use with `accumulator_runtime_with_source()` to run the event source on its own task alongside the processor.

---

## Reactor API

The reactor is the execution engine that wires accumulators to a compiled graph. It evaluates reaction criteria, manages the input cache, and calls the compiled graph function.

### Creation

```rust
use cloacina::computation_graph::reactor::{
    Reactor, ReactionCriteria, InputStrategy, CompiledGraphFn,
};
use tokio::sync::{mpsc, watch};

let (boundary_tx, boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(32);
let (_manual_tx, manual_rx) = mpsc::channel::<ManualCommand>(10);
let (shutdown_tx, shutdown_rx) = watch::channel(false);

let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
    Box::pin(async move { my_graph_compiled(&cache).await })
});

let reactor = Reactor::new(
    graph_fn,
    ReactionCriteria::WhenAny,
    InputStrategy::Latest,
    boundary_rx,
    manual_rx,
    shutdown_rx,
);
```

### Builder Methods

```rust
let reactor = Reactor::new(graph_fn, criteria, strategy, boundary_rx, manual_rx, shutdown_rx)
    .with_graph_name("market_maker".to_string())
    .with_expected_sources(vec![SourceName::new("alpha"), SourceName::new("beta")])
    .with_dal(dal)
    .with_health(health_tx)
    .with_accumulator_health(health_receivers)
    .with_batch_flush_senders(flush_senders);
```

| Method | Purpose |
|--------|---------|
| `with_graph_name(String)` | Sets the name used for DAL persistence keying |
| `with_expected_sources(Vec<SourceName>)` | Seeds dirty flags for `WhenAll` correctness |
| `with_dal(DAL)` | Enables cache persistence and crash recovery |
| `with_health(watch::Sender)` | Enables health state reporting |
| `with_accumulator_health(Vec<...>)` | Enables startup gating and degraded mode detection |
| `with_batch_flush_senders(Vec<...>)` | Signals batch accumulators after each execution |

### Running the Reactor

```rust
// Get a handle before running (for WebSocket queries)
let handle = reactor.handle();

// Run (consumes the reactor, blocks until shutdown)
tokio::spawn(reactor.run());
```

### ReactorHandle

Provides shared access to reactor state for external queries:

```rust
let state: HashMap<String, String> = handle.get_state().await;
let paused: bool = handle.is_paused();
handle.pause();
handle.resume();
```

### InputStrategy

| Strategy | Behavior |
|----------|----------|
| `InputStrategy::Latest` | One slot per source, overwritten on update. Always fires with the freshest data. |
| `InputStrategy::Sequential` | Boundaries preserved in order. One graph execution per queued boundary. |

### ReactionCriteria

| Criteria | Behavior |
|----------|----------|
| `ReactionCriteria::WhenAny` | Fire when any source has new data (dirty flag set) |
| `ReactionCriteria::WhenAll` | Fire only when all expected sources have new data |

### ManualCommand

External commands sent to the reactor:

```rust
use cloacina::computation_graph::reactor::ManualCommand;

// Fire with current cache state
manual_tx.send(ManualCommand::ForceFire).await;

// Fire with injected state (replaces cache)
manual_tx.send(ManualCommand::FireWith(custom_cache)).await;
```

### Reactor Health States

The reactor reports its health via a `watch` channel:

| State | Meaning |
|-------|---------|
| `Starting` | Loading cache from DAL, spawning accumulators |
| `Warming { healthy, waiting }` | Some accumulators healthy, waiting for others |
| `Live` | All accumulators healthy, evaluating criteria normally |
| `Degraded { disconnected }` | Was live, an accumulator disconnected. Running with stale data. |

### Channel Architecture

```text
                    socket_tx
External Events ──────────────→ Accumulator ──→ BoundarySender
                                                      │
                    boundary_tx                        │
                  <-------------------------------------+
                  │
                  ▼
              Reactor (boundary_rx)
                  │
                  ├── Receiver task: updates InputCache, sets dirty flags
                  ├── Strategy: evaluates WhenAny/WhenAll criteria
                  └── Executor: calls compiled graph function

ManualCommand ──→ manual_rx ──→ Reactor (ForceFire / FireWith)
```

### Accumulator Runtime

Spawn an accumulator with the full runtime (socket receiver + merge channel + processor):

```rust
let (boundary_tx, boundary_rx) = mpsc::channel(32);
let (socket_tx, socket_rx) = mpsc::channel(10);
let (shutdown_tx, shutdown_rx) = shutdown_signal();

let sender = BoundarySender::new(boundary_tx, SourceName::new("pricing"));
let ctx = AccumulatorContext {
    output: sender,
    name: "pricing".to_string(),
    shutdown: shutdown_rx,
    checkpoint: None,
    health: None,
};

tokio::spawn(accumulator_runtime(
    MyAccumulator, ctx, socket_rx, AccumulatorRuntimeConfig::default(),
));

// Push events via socket channel
socket_tx.send(serialize(&my_event).unwrap()).await.unwrap();
```

---

## Global Registry

The global registry stores computation graph constructors for embedded-mode auto-discovery. Graphs register themselves at program startup via `#[ctor]`. `#[ctor]` runs the annotated function automatically at program startup, before `main()`, ensuring graphs are registered without explicit initialization.

### Registration

```rust
use cloacina::computation_graph::{
    register_computation_graph_constructor,
    ComputationGraphRegistration,
};

register_computation_graph_constructor(
    "my_graph".to_string(),
    || ComputationGraphRegistration {
        graph_fn: Arc::new(|cache| Box::pin(async move {
            my_graph_compiled(&cache).await
        })),
        accumulator_names: vec!["alpha".to_string(), "beta".to_string()],
        reaction_mode: "when_any".to_string(),
    },
);
```

### Querying

```rust
// List all registered graph names
let names: Vec<String> = list_registered_graphs();

// Access the registry directly
let registry = global_computation_graph_registry();
let lock = registry.read();
if let Some(constructor) = lock.get("my_graph") {
    let registration = constructor();
    // registration.graph_fn, .accumulator_names, .reaction_mode
}

// Remove a graph
deregister_computation_graph("my_graph");
```

### ComputationGraphRegistration

```rust
pub struct ComputationGraphRegistration {
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
    /// Accumulator names declared in the graph topology.
    pub accumulator_names: Vec<String>,
    /// Reaction mode: "when_any" or "when_all".
    pub reaction_mode: String,
}
```

The `CompiledGraphFn` type alias:

```rust
pub type CompiledGraphFn = Arc<
    dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + Send + Sync
>;
```

---

## Packaging (for .cloacina packages)

Computation graphs can be compiled into standalone `.cloacina` packages (cdylib shared libraries) for deployment to the Cloacina server without recompilation.

### Feature Flag

Enable the `packaged` feature in your graph crate's `Cargo.toml`:

```toml
[features]
default = []
packaged = ["cloacina-computation-graph", "cloacina-workflow-plugin", "tokio"]

[dependencies]
cloacina-computation-graph = { version = "0.1" }
cloacina-workflow-plugin = { version = "0.1", optional = true }
tokio = { version = "1", features = ["full"], optional = true }

[lib]
crate-type = ["cdylib"]
```

### FFI Exports

When `feature = "packaged"` is active, the `#[computation_graph]` macro generates an FFI module that exposes the graph via the fidius plugin system:

```rust
// Auto-generated (do not write manually):
#[cfg(feature = "packaged")]
pub mod _ffi {
    // Implements CloacinaPlugin trait with:
    // - get_graph_metadata() -> GraphPackageMetadata
    // - execute_graph(request) -> GraphExecutionResult
    // - fidius_plugin_registry!() for dynamic loading
}
```

The generated FFI plugin exposes three methods:

| Method | Purpose |
|--------|---------|
| `get_task_metadata()` | Returns empty (CG packages have no workflow tasks) |
| `get_graph_metadata()` | Returns graph name, reaction mode, accumulator declarations |
| `execute_graph(request)` | Builds InputCache from request, executes compiled graph, returns results |

### Graph Metadata in package.toml

```toml
[package]
name = "my-market-maker"
version = "0.1.0"
type = "computation_graph"

[graph]
name = "market_maker"
reaction_mode = "when_any"
input_strategy = "latest"

[[graph.accumulators]]
name = "orderbook"
type = "passthrough"

[[graph.accumulators]]
name = "pricing"
type = "stream"
topic = "market.pricing"
group = "mm_pricing_group"
```

### How the Reconciler Loads Packaged Graphs

1. The reconciler discovers `.cloacina` packages in the configured package directory
2. For `type = "computation_graph"` packages, `build_declaration_from_ffi()` loads the cdylib via fidius and creates a `ComputationGraphDeclaration`
3. The `ComputationGraphScheduler` spawns accumulators + reactor from the declaration
4. On each reactor fire, `execute_graph()` is called via FFI on the loaded plugin
5. The plugin deserializes the cache, runs the compiled graph, and returns serialized terminal outputs

The FFI boundary always uses JSON strings regardless of build profile. The plugin internally re-serializes using the graph's native format (JSON in debug, bincode in release).

---

## Topology Validation

The macro performs compile-time validation:

| Check | Error |
|-------|-------|
| Every node in the graph has a function in the module | `"node 'X' is referenced in the graph topology but no function with that name exists"` |
| Every function in the module appears in the graph | `"function 'X' exists in the module but is not referenced in the graph topology"` |
| No cycles in the graph | `"cycle detected in graph: nodes involved in cycle: X, Y"` |
| At least one entry node exists | `"computation graph has no entry nodes"` |
| Routing edges have at least one variant | `"routing edge must have at least one variant"` |
| No duplicate `react` or `graph` attributes | `"duplicate 'react' field"` |
| Valid reaction mode | `"unknown reaction mode 'X', expected 'when_any' or 'when_all'"` |

---

## Routing Example (Complete)

A market maker decision engine with enum dispatch:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData { pub best_bid: f64, pub best_ask: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData { pub mid_price: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal { pub direction: String, pub price: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoActionReason { pub reason: String }

#[cloacina_macros::computation_graph(
    react = when_any(orderbook, pricing),
    graph = {
        decision(orderbook, pricing) => {
            Trade -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod market_maker {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Trade(TradeSignal),
        NoAction(NoActionReason),
    }

    pub async fn decision(
        orderbook: Option<&OrderBookData>,
        pricing: Option<&PricingData>,
    ) -> DecisionOutcome {
        let book = match orderbook {
            Some(ob) => ob,
            None => return DecisionOutcome::NoAction(NoActionReason {
                reason: "no order book data".to_string(),
            }),
        };
        let spread = book.best_ask - book.best_bid;
        if spread < 0.20 {
            DecisionOutcome::Trade(TradeSignal {
                direction: "BUY".to_string(),
                price: (book.best_bid + book.best_ask) / 2.0,
            })
        } else {
            DecisionOutcome::NoAction(NoActionReason {
                reason: format!("spread too wide: {:.2}", spread),
            })
        }
    }

    pub async fn signal_handler(signal: &TradeSignal) -> TradeSignal {
        signal.clone()
    }

    pub async fn audit_logger(reason: &NoActionReason) -> NoActionReason {
        reason.clone()
    }
}
```

The macro generates a `match` on the enum, routing variant data to the appropriate downstream node. Each branch executes its target node and pushes the result into `__terminal_results`.
