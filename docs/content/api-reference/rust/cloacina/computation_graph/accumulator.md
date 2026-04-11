# cloacina::computation_graph::accumulator <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Accumulator trait, runtime, and supporting types.

An accumulator is a long-lived process that consumes events from a source,
optionally aggregates them, and pushes typed boundaries to a reactor.
See CLOACI-S-0004 for the full specification.

## Structs

### `cloacina::computation_graph::accumulator::CheckpointHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Handle for persisting accumulator state via the DAL.

Wraps simple key-value checkpoint storage keyed by (graph_name, accumulator_name).
Serialization uses the same debug-JSON/release-bincode pattern as boundary wire format.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `crate :: dal :: unified :: DAL` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : crate :: dal :: unified :: DAL , graph_name : String , accumulator_name : String ,) -> Self
```

Create a new checkpoint handle for the given graph and accumulator.

<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: crate::dal::unified::DAL,
        graph_name: String,
        accumulator_name: String,
    ) -> Self {
        Self {
            dal,
            graph_name,
            accumulator_name,
        }
    }
```

</details>



##### `save` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn save < T : Serialize > (& self , state : & T) -> Result < () , AccumulatorError >
```

Persist accumulator state.

<details>
<summary>Source</summary>

```rust
    pub async fn save<T: Serialize>(&self, state: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(state)
            .map_err(|e| AccumulatorError::Checkpoint(format!("serialization failed: {}", e)))?;
        self.dal
            .checkpoint()
            .save_checkpoint(&self.graph_name, &self.accumulator_name, bytes)
            .await
            .map_err(|e| AccumulatorError::Checkpoint(e.to_string()))
    }
```

</details>



##### `load` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn load < T : DeserializeOwned > (& self) -> Result < Option < T > , AccumulatorError >
```

Load previously persisted accumulator state.

<details>
<summary>Source</summary>

```rust
    pub async fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, AccumulatorError> {
        let bytes = self
            .dal
            .checkpoint()
            .load_checkpoint(&self.graph_name, &self.accumulator_name)
            .await
            .map_err(|e| AccumulatorError::Checkpoint(e.to_string()))?;
        match bytes {
            Some(data) => {
                let state = types::deserialize(&data).map_err(|e| {
                    AccumulatorError::Checkpoint(format!("deserialization failed: {}", e))
                })?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
```

</details>



##### `dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn dal (& self) -> & crate :: dal :: unified :: DAL
```

Access the underlying DAL for direct checkpoint operations.

<details>
<summary>Source</summary>

```rust
    pub fn dal(&self) -> &crate::dal::unified::DAL {
        &self.dal
    }
```

</details>



##### `graph_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn graph_name (& self) -> & str
```

Get the graph name this handle is scoped to.

<details>
<summary>Source</summary>

```rust
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }
```

</details>



##### `accumulator_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn accumulator_name (& self) -> & str
```

Get the accumulator name this handle is scoped to.

<details>
<summary>Source</summary>

```rust
    pub fn accumulator_name(&self) -> &str {
        &self.accumulator_name
    }
```

</details>





### `cloacina::computation_graph::accumulator::AccumulatorContext`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Context provided to the accumulator by the runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `output` | `BoundarySender` | Send a boundary to the reactor. |
| `name` | `String` | Accumulator's name (used for registration and logging). |
| `shutdown` | `watch :: Receiver < bool >` | Shutdown signal — accumulator should exit run() when this fires. |
| `checkpoint` | `Option < CheckpointHandle >` | Handle to persist accumulator state. None when DAL is not available
(e.g., embedded mode without database). |
| `health` | `Option < watch :: Sender < AccumulatorHealth > >` | Health state reporter. None when health tracking is not needed
(e.g., tests, embedded mode). |



### `cloacina::computation_graph::accumulator::BoundarySender`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Sends serialized boundaries to the reactor.

Wire format: bincode in release, JSON in debug.
Tracks a monotonically increasing sequence number per accumulator
for deduplication and ordering guarantees.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `mpsc :: Sender < (SourceName , Vec < u8 >) >` |  |
| `source_name` | `SourceName` |  |
| `sequence` | `Arc < AtomicU64 >` | Monotonically increasing sequence counter (shared across clones). |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (sender : mpsc :: Sender < (SourceName , Vec < u8 >) > , source_name : SourceName) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }
```

</details>



##### `with_sequence` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_sequence (sender : mpsc :: Sender < (SourceName , Vec < u8 >) > , source_name : SourceName , start_sequence : u64 ,) -> Self
```

Create a sender with a specific starting sequence number (for restart recovery).

<details>
<summary>Source</summary>

```rust
    pub fn with_sequence(
        sender: mpsc::Sender<(SourceName, Vec<u8>)>,
        source_name: SourceName,
        start_sequence: u64,
    ) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: Arc::new(AtomicU64::new(start_sequence)),
        }
    }
```

</details>



##### `send` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn send < T : Serialize > (& self , boundary : & T) -> Result < () , AccumulatorError >
```

Serialize and send a boundary to the reactor. Increments the sequence counter atomically after successful send.

<details>
<summary>Source</summary>

```rust
    pub async fn send<T: Serialize>(&self, boundary: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(boundary)
            .map_err(|e| AccumulatorError::Send(format!("serialization failed: {}", e)))?;
        self.inner
            .send((self.source_name.clone(), bytes))
            .await
            .map_err(|e| AccumulatorError::Send(format!("channel send failed: {}", e)))?;
        self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
```

</details>



##### `source_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn source_name (& self) -> & SourceName
```

Get the source name this sender is associated with.

<details>
<summary>Source</summary>

```rust
    pub fn source_name(&self) -> &SourceName {
        &self.source_name
    }
```

</details>



##### `sequence_number` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn sequence_number (& self) -> u64
```

Get the current sequence number (last emitted).

<details>
<summary>Source</summary>

```rust
    pub fn sequence_number(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }
```

</details>





### `cloacina::computation_graph::accumulator::AccumulatorRuntimeConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Configuration for the accumulator runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `merge_channel_capacity` | `usize` | Merge channel capacity (backpressure). |



### `cloacina::computation_graph::accumulator::NoEventSource`<E>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


Placeholder type for when no event source is provided.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `std :: marker :: PhantomData < E >` |  |



### `cloacina::computation_graph::accumulator::BatchAccumulatorConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Configuration for the batch accumulator runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `Option < std :: time :: Duration >` | Optional timer-based flush interval. If None, only flushes on signal or size threshold. |
| `max_buffer_size` | `Option < usize >` | Optional: flush when buffer reaches this size. |



### `cloacina::computation_graph::accumulator::StateAccumulator`<T: Serialize + DeserializeOwned + Send + Clone + 'static>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A state accumulator holds a bounded VecDeque<T> that receives values from the computation graph (collector or mid-graph writes), persists to DAL on every write, and loads from DAL on startup. Enables cyclic state patterns where the graph's output feeds back as input on the next execution.

Capacity modes:
- `capacity > 0`: bounded — evicts oldest when at capacity
- `capacity < 0` (e.g., -1): unbounded — grows without limit
- `capacity == 0`: write-only sink — no history emitted back

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `buffer` | `std :: collections :: VecDeque < T >` |  |
| `capacity` | `i32` |  |



## Enums

### `cloacina::computation_graph::accumulator::AccumulatorHealth` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Health state of an accumulator, reported via watch channel.

The reactor watches these to gate its own startup (Warming → Live)
and detect degradation (Live → Degraded).

#### Variants

- **`Starting`** - Loading checkpoint from DAL.
- **`Connecting`** - Checkpoint loaded, connecting to source. Socket already active.
- **`Live`** - Connected, processing events, pushing boundaries.
- **`Disconnected`** - Was live, lost source connection. Socket still active. Retrying.
- **`SocketOnly`** - Passthrough — no source to connect to. Healthy by definition.



### `cloacina::computation_graph::accumulator::AccumulatorError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from accumulator operations.

#### Variants

- **`Init`**
- **`Run`**
- **`Send`**
- **`Checkpoint`**



## Functions

### `cloacina::computation_graph::accumulator::health_channel`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn health_channel () -> (watch :: Sender < AccumulatorHealth > , watch :: Receiver < AccumulatorHealth > ,)
```

Create a health reporting channel for an accumulator.

<details>
<summary>Source</summary>

```rust
pub fn health_channel() -> (
    watch::Sender<AccumulatorHealth>,
    watch::Receiver<AccumulatorHealth>,
) {
    watch::channel(AccumulatorHealth::Starting)
}
```

</details>



### `cloacina::computation_graph::accumulator::accumulator_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn accumulator_runtime < A : Accumulator > (acc : A , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > , config : AccumulatorRuntimeConfig ,)
```

Run an accumulator as 2-3 tokio tasks connected by a merge channel.

Socket-only mode (no event source):
```text
┌─────────────────┐     ┌─────────────────┐
│  Socket task     │──→  │  Processor task  │──→ BoundarySender ──→ Reactor
│  (always active) │     │  (calls process) │
└─────────────────┘     └─────────────────┘
```
With event source (use [`accumulator_runtime_with_source`]):
```text
┌─────────────────┐
│  Event source    │──→ mpsc<Event> ──┐
│  (pulls events)  │                  │     ┌─────────────────┐
└─────────────────┘                  ├────→│  Processor task  │──→ BoundarySender ──→ Reactor
┌─────────────────┐                  │     │  (calls process) │
│  Socket task     │──→ mpsc<Event> ──┘     └─────────────────┘
│  (always active) │
└─────────────────┘
```

<details>
<summary>Source</summary>

```rust
pub async fn accumulator_runtime<A: Accumulator>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
) {
    accumulator_runtime_inner::<A, NoEventSource<A::Event>>(acc, ctx, socket_rx, config, None).await
}
```

</details>



### `cloacina::computation_graph::accumulator::accumulator_runtime_with_source`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn accumulator_runtime_with_source < A , S > (acc : A , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > , config : AccumulatorRuntimeConfig , source : S ,) where A : Accumulator , S : EventSource < Event = A :: Event > ,
```

Run an accumulator with an active event source that pulls events from an external system. The event source runs on its own task and pushes events into the merge channel concurrently with the socket receiver.

<details>
<summary>Source</summary>

```rust
pub async fn accumulator_runtime_with_source<A, S>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
    source: S,
) where
    A: Accumulator,
    S: EventSource<Event = A::Event>,
{
    accumulator_runtime_inner(acc, ctx, socket_rx, config, Some(source)).await
}
```

</details>



### `cloacina::computation_graph::accumulator::accumulator_runtime_inner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn accumulator_runtime_inner < A : Accumulator , S : EventSource < Event = A :: Event > > (mut acc : A , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > , config : AccumulatorRuntimeConfig , event_source : Option < S > ,)
```

Inner runtime shared by both `accumulator_runtime` and `accumulator_runtime_with_source`.

<details>
<summary>Source</summary>

```rust
async fn accumulator_runtime_inner<A: Accumulator, S: EventSource<Event = A::Event>>(
    mut acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
    event_source: Option<S>,
) {
    // Report starting health
    set_health(&ctx, AccumulatorHealth::Starting);

    // Initialize — may restore state from checkpoint
    if let Err(e) = acc.init(&ctx).await {
        tracing::error!(name = %ctx.name, "accumulator init failed: {}", e);
        return;
    }

    // Create merge channel
    let (event_tx, mut event_rx) = mpsc::channel::<A::Event>(config.merge_channel_capacity);

    // Spawn event source task (or no-op wait if none provided)
    let name_loop = ctx.name.clone();
    let loop_handle = if let Some(source) = event_source {
        set_health(&ctx, AccumulatorHealth::Connecting);
        let shutdown_source = ctx.shutdown.clone();
        let event_tx_source = event_tx.clone();
        let name_source = name_loop.clone();
        tokio::spawn(async move {
            match source.run(event_tx_source, shutdown_source).await {
                Ok(()) => tracing::debug!(name = %name_source, "event source completed"),
                Err(e) => tracing::error!(name = %name_source, "event source failed: {}", e),
            }
        })
    } else {
        set_health(&ctx, AccumulatorHealth::SocketOnly);
        let mut shutdown_loop = ctx.shutdown.clone();
        tokio::spawn(async move {
            let _ = shutdown_loop.changed().await;
            tracing::debug!(name = %name_loop, "event loop task shutting down");
        })
    };

    // Spawn socket receiver task
    let event_tx_socket = event_tx.clone();
    let mut shutdown_socket = ctx.shutdown.clone();
    let name_socket = ctx.name.clone();
    let socket_handle = tokio::spawn(async move {
        let mut socket_rx = socket_rx;
        loop {
            tokio::select! {
                Some(bytes) = socket_rx.recv() => {
                    match types::deserialize::<A::Event>(&bytes) {
                        Ok(event) => {
                            if event_tx_socket.send(event).await.is_err() {
                                break; // merge channel closed
                            }
                        }
                        Err(e) => {
                            tracing::warn!(name = %name_socket, "socket deserialize error: {}", e);
                        }
                    }
                }
                _ = shutdown_socket.changed() => {
                    tracing::debug!(name = %name_socket, "socket task shutting down");
                    break;
                }
            }
        }
    });

    // Processor task (runs on current task — owns &mut acc)
    let mut shutdown_proc = ctx.shutdown.clone();
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                if let Some(boundary) = acc.process(event) {
                    if let Err(e) = ctx.output.send(&boundary).await {
                        tracing::error!(name = %ctx.name, "boundary send failed: {}", e);
                    } else {
                        persist_boundary(&ctx, &boundary).await;
                    }
                }
            }
            _ = shutdown_proc.changed() => {
                tracing::debug!(name = %ctx.name, "processor task shutting down");
                break;
            }
        }
    }

    // Wait for spawned tasks to finish
    let _ = loop_handle.await;
    let _ = socket_handle.await;
}
```

</details>



### `cloacina::computation_graph::accumulator::shutdown_signal`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn shutdown_signal () -> (watch :: Sender < bool > , watch :: Receiver < bool >)
```

Create a shutdown signal pair.

<details>
<summary>Source</summary>

```rust
pub fn shutdown_signal() -> (watch::Sender<bool>, watch::Receiver<bool>) {
    watch::channel(false)
}
```

</details>



### `cloacina::computation_graph::accumulator::polling_accumulator_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn polling_accumulator_runtime < P : PollingAccumulator > (mut poller : P , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > ,)
```

Run a polling accumulator as a timer-based loop.

On each tick: calls `poll()`, if Some → serializes and sends boundary.
Also accepts socket events (same as passthrough — external pushes still work).

<details>
<summary>Source</summary>

```rust
pub async fn polling_accumulator_runtime<P: PollingAccumulator>(
    mut poller: P,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
) {
    set_health(&ctx, AccumulatorHealth::Starting);

    // Restore last poll output from checkpoint and emit to reactor
    if let Some(ref handle) = ctx.checkpoint {
        match handle.load::<P::Output>().await {
            Ok(Some(output)) => {
                tracing::info!(name = %ctx.name, "polling accumulator restored last output from checkpoint");
                if let Err(e) = ctx.output.send(&output).await {
                    tracing::warn!(name = %ctx.name, "failed to emit restored poll output: {}", e);
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load polling checkpoint: {}", e);
            }
        }
    }

    let interval = poller.interval();
    let mut timer = tokio::time::interval(interval);
    // Skip the first immediate tick — we want to wait one interval before first poll
    timer.tick().await;

    // Polling accumulators are Live once the timer starts
    set_health(&ctx, AccumulatorHealth::Live);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            _ = timer.tick() => {
                if let Some(output) = poller.poll().await {
                    if let Err(e) = ctx.output.send(&output).await {
                        tracing::error!(name = %ctx.name, "polling boundary send failed: {}", e);
                    } else {
                        persist_boundary(&ctx, &output).await;
                        // Checkpoint the last successful poll output
                        if let Some(ref handle) = ctx.checkpoint {
                            if let Err(e) = handle.save(&output).await {
                                tracing::warn!(name = %ctx.name, "polling checkpoint save failed: {}", e);
                            }
                        }
                    }
                }
            }
            Some(bytes) = socket_rx.recv() => {
                // Socket events are deserialized as Output and sent directly
                match types::deserialize::<P::Output>(&bytes) {
                    Ok(output) => {
                        if let Err(e) = ctx.output.send(&output).await {
                            tracing::error!(name = %ctx.name, "socket boundary send failed: {}", e);
                        } else {
                            persist_boundary(&ctx, &output).await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "socket deserialize error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "polling accumulator shutting down");
                break;
            }
        }
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::flush_signal`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn flush_signal () -> (mpsc :: Sender < () > , mpsc :: Receiver < () >)
```

Create a flush signal pair for batch accumulators.

The sender is held by the reactor (or external code) and used to trigger
a flush. The receiver is passed to `batch_accumulator_runtime`.

<details>
<summary>Source</summary>

```rust
pub fn flush_signal() -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    mpsc::channel(16)
}
```

</details>



### `cloacina::computation_graph::accumulator::batch_accumulator_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn batch_accumulator_runtime < B : BatchAccumulator > (mut acc : B , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > , mut flush_rx : mpsc :: Receiver < () > , config : BatchAccumulatorConfig ,) where B :: Event : Serialize ,
```

Run a batch accumulator that buffers events and flushes on signal, timer, or size threshold.

Primary flush trigger is the `flush_rx` channel — typically sent by the reactor
after each graph execution ("give me everything since last run").
Timer and size threshold are secondary/fallback triggers.

<details>
<summary>Source</summary>

```rust
pub async fn batch_accumulator_runtime<B: BatchAccumulator>(
    mut acc: B,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    mut flush_rx: mpsc::Receiver<()>,
    config: BatchAccumulatorConfig,
) where
    B::Event: Serialize,
{
    set_health(&ctx, AccumulatorHealth::Starting);

    // Restore buffered events from checkpoint if available
    let mut buffer: Vec<B::Event> = Vec::new();
    if let Some(ref handle) = ctx.checkpoint {
        match handle.load::<Vec<Vec<u8>>>().await {
            Ok(Some(raw_events)) => {
                for raw in raw_events {
                    if let Ok(event) = types::deserialize::<B::Event>(&raw) {
                        buffer.push(event);
                    }
                }
                if !buffer.is_empty() {
                    tracing::info!(name = %ctx.name, events = buffer.len(), "batch buffer restored from checkpoint");
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load batch checkpoint: {}", e);
            }
        }
    }

    // Create timer only if interval is configured
    let mut timer = config.flush_interval.map(tokio::time::interval);
    if let Some(ref mut t) = timer {
        // Skip the first immediate tick
        t.tick().await;
    }

    // Batch accumulators are Live once ready to receive events
    set_health(&ctx, AccumulatorHealth::Live);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                match types::deserialize::<B::Event>(&bytes) {
                    Ok(event) => {
                        buffer.push(event);
                        // Persist buffer snapshot for crash resilience
                        persist_batch_buffer(&ctx, &buffer).await;
                        // Check size threshold
                        if let Some(max) = config.max_buffer_size {
                            if buffer.len() >= max {
                                flush_batch(&mut acc, &mut buffer, &ctx).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "batch deserialize error: {}", e);
                    }
                }
            }
            Some(()) = flush_rx.recv() => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                // Clear checkpoint after flush (buffer is empty)
                persist_batch_buffer::<B::Event>(&ctx, &[]).await;
            }
            _ = async {
                match timer.as_mut() {
                    Some(t) => t.tick().await,
                    None => std::future::pending().await,
                }
            } => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "batch accumulator shutting down, draining buffer");
                // Drain remaining buffer on shutdown
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                break;
            }
        }
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::persist_batch_buffer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn persist_batch_buffer < E : Serialize > (ctx : & AccumulatorContext , buffer : & [E])
```

Persist batch buffer snapshot to DAL for crash resilience (best-effort).

<details>
<summary>Source</summary>

```rust
async fn persist_batch_buffer<E: Serialize>(ctx: &AccumulatorContext, buffer: &[E]) {
    if let Some(ref handle) = ctx.checkpoint {
        // Serialize each event to raw bytes, then save the vec of raw bytes
        let raw: Vec<Vec<u8>> = buffer
            .iter()
            .filter_map(|e| types::serialize(e).ok())
            .collect();
        if let Err(e) = handle.save(&raw).await {
            tracing::warn!(name = %ctx.name, "batch buffer checkpoint failed: {}", e);
        }
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::flush_batch`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn flush_batch < B : BatchAccumulator > (acc : & mut B , buffer : & mut Vec < B :: Event > , ctx : & AccumulatorContext ,)
```

Flush the buffer through the batch accumulator and send boundary if produced.

<details>
<summary>Source</summary>

```rust
async fn flush_batch<B: BatchAccumulator>(
    acc: &mut B,
    buffer: &mut Vec<B::Event>,
    ctx: &AccumulatorContext,
) {
    if buffer.is_empty() {
        return;
    }
    let batch = std::mem::take(buffer);
    let count = batch.len();
    if let Some(output) = acc.process_batch(batch) {
        if let Err(e) = ctx.output.send(&output).await {
            tracing::error!(name = %ctx.name, "batch boundary send failed: {}", e);
        } else {
            tracing::debug!(name = %ctx.name, events = count, "batch flushed");
            persist_boundary(ctx, &output).await;
        }
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::set_health`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn set_health (ctx : & AccumulatorContext , health : AccumulatorHealth)
```

Set health state (best-effort, no-op if health channel not configured).

<details>
<summary>Source</summary>

```rust
fn set_health(ctx: &AccumulatorContext, health: AccumulatorHealth) {
    if let Some(ref sender) = ctx.health {
        let _ = sender.send(health);
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::persist_boundary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn persist_boundary < T : Serialize > (ctx : & AccumulatorContext , boundary : & T)
```

Persist last-emitted boundary with sequence number to DAL (best-effort, logs on failure).

<details>
<summary>Source</summary>

```rust
async fn persist_boundary<T: Serialize>(ctx: &AccumulatorContext, boundary: &T) {
    if let Some(ref handle) = ctx.checkpoint {
        let bytes = match types::serialize(boundary) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(name = %ctx.name, "boundary persistence serialization failed: {}", e);
                return;
            }
        };
        let seq = ctx.output.sequence_number() as i64;
        if let Err(e) = handle
            .dal()
            .checkpoint()
            .save_boundary(handle.graph_name(), handle.accumulator_name(), bytes, seq)
            .await
        {
            tracing::warn!(name = %ctx.name, "boundary persistence failed: {}", e);
        }
    }
}
```

</details>



### `cloacina::computation_graph::accumulator::state_accumulator_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn state_accumulator_runtime < T : Serialize + DeserializeOwned + Send + Clone + 'static > (mut acc : StateAccumulator < T > , ctx : AccumulatorContext , socket_rx : mpsc :: Receiver < Vec < u8 > > ,)
```

Run a state accumulator. Receives values via socket, appends to VecDeque, evicts if over capacity, persists to DAL, and emits the full list as boundary.

On startup: loads from DAL and emits current list to reactor.

<details>
<summary>Source</summary>

```rust
pub async fn state_accumulator_runtime<T: Serialize + DeserializeOwned + Send + Clone + 'static>(
    mut acc: StateAccumulator<T>,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
) {
    set_health(&ctx, AccumulatorHealth::Starting);

    // Load from DAL on startup
    if let Some(ref handle) = ctx.checkpoint {
        match handle
            .dal()
            .checkpoint()
            .load_state_buffer(handle.graph_name(), handle.accumulator_name())
            .await
        {
            Ok(Some((data, _cap))) => {
                if let Ok(buffer) = types::deserialize::<std::collections::VecDeque<T>>(&data) {
                    acc.buffer = buffer;
                    tracing::info!(name = %ctx.name, entries = acc.buffer.len(), "state accumulator restored from DAL");
                }
            }
            Ok(None) => {
                tracing::debug!(name = %ctx.name, "no persisted state accumulator buffer found");
            }
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load state buffer: {}", e);
            }
        }

        // Emit current list to reactor immediately (so reactor has state on startup)
        if !acc.buffer.is_empty() && acc.capacity != 0 {
            let list: Vec<T> = acc.buffer.iter().cloned().collect();
            if let Err(e) = ctx.output.send(&list).await {
                tracing::error!(name = %ctx.name, "state accumulator initial emit failed: {}", e);
            }
        }
    }

    set_health(&ctx, AccumulatorHealth::SocketOnly);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                match types::deserialize::<T>(&bytes) {
                    Ok(value) => {
                        // Append to buffer
                        acc.buffer.push_back(value);

                        // Evict if over capacity (bounded mode)
                        if acc.capacity > 0 {
                            while acc.buffer.len() > acc.capacity as usize {
                                acc.buffer.pop_front();
                            }
                        }

                        // Persist to DAL
                        if let Some(ref handle) = ctx.checkpoint {
                            let data = match types::serialize(&acc.buffer) {
                                Ok(d) => d,
                                Err(e) => {
                                    tracing::warn!(name = %ctx.name, "state buffer serialization failed: {}", e);
                                    continue;
                                }
                            };
                            if let Err(e) = handle
                                .dal()
                                .checkpoint()
                                .save_state_buffer(
                                    handle.graph_name(),
                                    handle.accumulator_name(),
                                    data,
                                    acc.capacity,
                                )
                                .await
                            {
                                tracing::warn!(name = %ctx.name, "state buffer persistence failed: {}", e);
                            }
                        }

                        // Emit full list as boundary (unless write-only mode)
                        if acc.capacity != 0 {
                            let list: Vec<T> = acc.buffer.iter().cloned().collect();
                            if let Err(e) = ctx.output.send(&list).await {
                                tracing::error!(name = %ctx.name, "state accumulator emit failed: {}", e);
                            } else {
                                persist_boundary(&ctx, &list).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "state accumulator deserialize error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "state accumulator shutting down");
                break;
            }
        }
    }
}
```

</details>
