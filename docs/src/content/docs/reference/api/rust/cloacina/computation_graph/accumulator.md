---
title: "cloacina::computation_graph::accumulator"
description: "Accumulator trait, runtime, and supporting types."
editUrl: false
sidebar:
  order: 40
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Accumulator trait, runtime, and supporting types.

An accumulator is a long-lived process that consumes events from a source,
optionally aggregates them, and pushes typed boundaries to a reactor.
See CLOACI-S-0004 for the full specification.

## Structs

### <span id="struct.CheckpointHandle"></span><code style="font-size: 0.85em">struct CheckpointHandle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Handle for persisting accumulator state via the DAL.

Wraps simple key-value checkpoint storage keyed by (graph_name, accumulator_name).
Serialization uses bincode wire format.

#### Methods

##### <span id="CheckpointHandle.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    dal: crate::dal::unified::DAL,
    graph_name: String,
    accumulator_name: String,
) -> Self
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

##### <span id="CheckpointHandle.save"></span><code>save</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn save<T: Serialize>(
    &self) -> Result<(,
    state: &T) -> Result<(,
) -> Result<(), AccumulatorError>
```

Persist accumulator state.

<details>
<summary>Source</summary>

```rust
    pub async fn save<T: Serialize>(&self, state: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(state)
            .map_err(|e| AccumulatorError::Checkpoint(format!("serialization failed: {}", e)))?;
        let result = self
            .dal
            .checkpoint()
            .save_checkpoint(&self.graph_name, &self.accumulator_name, bytes)
            .await
            .map_err(|e| AccumulatorError::Checkpoint(e.to_string()));
        // Only count successful writes — failed checkpoints surface via
        // `cloacina_accumulator_events_total` already (the event still
        // emitted) plus the warning log emitted by the caller.
        if result.is_ok() {
            metrics::counter!(
                "cloacina_accumulator_checkpoint_writes_total",
                "graph" => self.graph_name.clone(),
                "accumulator" => self.accumulator_name.clone(),
            )
            .increment(1);
        }
        result
    }
```

</details>

##### <span id="CheckpointHandle.load"></span><code>load</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load<T: DeserializeOwned>(
    &self,
) -> Result<Option<T>, AccumulatorError>
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

##### <span id="CheckpointHandle.dal"></span><code>dal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn dal(&self) -> &crate::dal::unified::DAL
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

##### <span id="CheckpointHandle.graph_name"></span><code>graph_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn graph_name(&self) -> &str
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

##### <span id="CheckpointHandle.accumulator_name"></span><code>accumulator_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn accumulator_name(&self) -> &str
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

### <span id="struct.AccumulatorContext"></span><code style="font-size: 0.85em">struct AccumulatorContext</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Context provided to the accumulator by the runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `output` | `BoundarySender` | Send a boundary to the reactor. |
| `name` | `String` | Accumulator's name (used for registration and logging). |
| `shutdown` | `watch::Receiver<bool>` | Shutdown signal — accumulator should exit run() when this fires. |
| `checkpoint` | `Option<CheckpointHandle>` | Handle to persist accumulator state. None when DAL is not available (e.g., embedded mode without database). |
| `health` | `Option<watch::Sender<AccumulatorHealth>>` | Health state reporter. None when health tracking is not needed (e.g., tests, embedded mode). |

### <span id="struct.BoundarySender"></span><code style="font-size: 0.85em">struct BoundarySender</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Sends serialized boundaries to the reactor.

Wire format: bincode.
Tracks a monotonically increasing sequence number per accumulator
for deduplication and ordering guarantees.

#### Methods

##### <span id="BoundarySender.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    sender: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: SourceName,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: Arc::new(AtomicU64::new(0)),
            last_event_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            buffer_depth: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
            buffer_capacity: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
        }
    }
```

</details>

##### <span id="BoundarySender.with_freshness"></span><code>with_​freshness</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_freshness(
    sender: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: SourceName,
    freshness: FreshnessHandle,
) -> Self
```

Create a sender that shares its events_total + last-event with a pre-made `FreshnessHandle` (CLOACI-T-0765), so the registry can sample freshness for the live accumulator.

<details>
<summary>Source</summary>

```rust
    pub fn with_freshness(
        sender: mpsc::Sender<(SourceName, Vec<u8>)>,
        source_name: SourceName,
        freshness: FreshnessHandle,
    ) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: freshness.events_total.clone(),
            last_event_ms: freshness.last_event_ms.clone(),
            buffer_depth: freshness.buffer_depth.clone(),
            buffer_capacity: freshness.buffer_capacity.clone(),
        }
    }
```

</details>

##### <span id="BoundarySender.with_sequence"></span><code>with_​sequence</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_sequence(
    sender: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: SourceName,
    start_sequence: u64,
) -> Self
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
            last_event_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            buffer_depth: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
            buffer_capacity: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
        }
    }
```

</details>

##### <span id="BoundarySender.send"></span><code>send</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn send<T: Serialize>(
    &self) -> Result<(,
    boundary: &T) -> Result<(,
) -> Result<(), AccumulatorError>
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
        // CLOACI-T-0765: stamp last-emit for the freshness probe.
        self.last_event_ms.store(now_unix_ms(), Ordering::Relaxed);
        Ok(())
    }
```

</details>

##### <span id="BoundarySender.source_name"></span><code>source_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn source_name(&self) -> &SourceName
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

##### <span id="BoundarySender.sequence_number"></span><code>sequence_​number</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn sequence_number(&self) -> u64
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

##### <span id="BoundarySender.set_buffer_depth"></span><code>set_​buffer_​depth</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_buffer_depth(&self, depth: u64)
```

Record the current buffered-event count so the health API can report it (CLOACI-T-0744). Called by the buffering runtimes (batch/state) alongside the Prometheus gauge.

<details>
<summary>Source</summary>

```rust
    pub fn set_buffer_depth(&self, depth: u64) {
        self.buffer_depth
            .store(depth as i64, std::sync::atomic::Ordering::Relaxed);
    }
```

</details>

##### <span id="BoundarySender.set_buffer_capacity"></span><code>set_​buffer_​capacity</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_buffer_capacity(&self, capacity: u64)
```

Record the declared buffer capacity for bounded kinds (CLOACI-T-0744). Pass a positive value; unbounded/none kinds just never call this.

<details>
<summary>Source</summary>

```rust
    pub fn set_buffer_capacity(&self, capacity: u64) {
        self.buffer_capacity
            .store(capacity as i64, std::sync::atomic::Ordering::Relaxed);
    }
```

</details>

##### <span id="BoundarySender.freshness"></span><code>freshness</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn freshness(&self) -> FreshnessHandle
```

A shared freshness probe (events_total + last_event), for registration with the graph registry so the health endpoint can report freshness.

<details>
<summary>Source</summary>

```rust
    pub fn freshness(&self) -> FreshnessHandle {
        FreshnessHandle {
            events_total: self.sequence.clone(),
            last_event_ms: self.last_event_ms.clone(),
            buffer_depth: self.buffer_depth.clone(),
            buffer_capacity: self.buffer_capacity.clone(),
        }
    }
```

</details>

### <span id="struct.FreshnessHandle"></span><code style="font-size: 0.85em">struct FreshnessHandle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Read-only freshness probe for an accumulator (CLOACI-T-0765): the monotonic emit count + the wall-clock of the last emit, shared (Arc) with the live `BoundarySender` so the registry/server can sample it without locking.

#### Methods

##### <span id="FreshnessHandle.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

A fresh probe (zeroed counters). Share it into a `BoundarySender` via `BoundarySender::with_freshness` and register it with the graph registry.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            events_total: Arc::new(AtomicU64::new(0)),
            last_event_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            buffer_depth: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
            buffer_capacity: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
        }
    }
```

</details>

##### <span id="FreshnessHandle.events_total"></span><code>events_​total</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn events_total(&self) -> u64
```

Total boundaries emitted since load (monotonic).

<details>
<summary>Source</summary>

```rust
    pub fn events_total(&self) -> u64 {
        self.events_total.load(Ordering::Relaxed)
    }
```

</details>

##### <span id="FreshnessHandle.last_event_ms"></span><code>last_​event_​ms</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn last_event_ms(&self) -> Option<i64>
```

Unix-millis of the last emit, or `None` if nothing has been emitted yet.

<details>
<summary>Source</summary>

```rust
    pub fn last_event_ms(&self) -> Option<i64> {
        let v = self.last_event_ms.load(Ordering::Relaxed);
        if v > 0 { Some(v) } else { None }
    }
```

</details>

##### <span id="FreshnessHandle.buffer_depth"></span><code>buffer_​depth</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn buffer_depth(&self) -> Option<u64>
```

Buffered-event count for buffering kinds (batch/state), or `None` for kinds that don't buffer / runtimes predating the gauge (CLOACI-T-0744).

<details>
<summary>Source</summary>

```rust
    pub fn buffer_depth(&self) -> Option<u64> {
        let v = self.buffer_depth.load(Ordering::Relaxed);
        if v >= 0 { Some(v as u64) } else { None }
    }
```

</details>

##### <span id="FreshnessHandle.buffer_capacity"></span><code>buffer_​capacity</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn buffer_capacity(&self) -> Option<u64>
```

Declared buffer capacity for bounded kinds, or `None` when unbounded / not applicable (CLOACI-T-0744). The UI renders `depth/capacity`.

<details>
<summary>Source</summary>

```rust
    pub fn buffer_capacity(&self) -> Option<u64> {
        let v = self.buffer_capacity.load(Ordering::Relaxed);
        if v > 0 { Some(v as u64) } else { None }
    }
```

</details>

### <span id="struct.AccumulatorRuntimeConfig"></span><code style="font-size: 0.85em">struct AccumulatorRuntimeConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Configuration for the accumulator runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `merge_channel_capacity` | `usize` | Merge channel capacity (backpressure). |

### <span id="struct.BatchAccumulatorConfig"></span><code style="font-size: 0.85em">struct BatchAccumulatorConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Default`

Configuration for the batch accumulator runtime.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `Option<std::time::Duration>` | Optional timer-based flush interval. If None, only flushes on signal or size threshold. |
| `max_buffer_size` | `Option<usize>` | Optional: flush when buffer reaches this size. |

### <span id="struct.StateAccumulator"></span><code style="font-size: 0.85em">struct StateAccumulator&lt;T: Serialize + DeserializeOwned + Send + Clone + &#39;static&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A state accumulator holds a bounded VecDeque<T> that receives values from the computation graph (collector or mid-graph writes), persists to DAL on every write, and loads from DAL on startup. Enables cyclic state patterns where the graph's output feeds back as input on the next execution.

Capacity modes:
- `capacity > 0`: bounded — evicts oldest when at capacity
- `capacity < 0` (e.g., -1): unbounded — grows without limit
- `capacity == 0`: write-only sink — no history emitted back

#### Methods

##### <span id="StateAccumulator.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(capacity: i32) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(capacity: i32) -> Self {
        Self {
            buffer: std::collections::VecDeque::new(),
            capacity,
        }
    }
```

</details>

## Enums

### <span id="enum.AccumulatorHealth"></span><code style="font-size: 0.85em">enum AccumulatorHealth</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Health state of an accumulator, reported via watch channel.

The reactor watches these to gate its own startup (Warming → Live)
and detect degradation (Live → Degraded).

#### Variants

- **`Starting`** - Loading checkpoint from DAL.
- **`Connecting`** - Checkpoint loaded, connecting to source. Socket already active.
- **`Live`** - Connected, processing events, pushing boundaries.
- **`Disconnected`** - Was live, lost source connection. Socket still active. Retrying.
- **`SocketOnly`** - Passthrough — no source to connect to. Healthy by definition.

#### Methods

##### <span id="AccumulatorHealth.as_state_label"></span><code>as_​state_​label</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_state_label(&self) -> &'static str
```

Project the AccumulatorHealth state machine onto the bounded `cloacina_component_health` gauge label values (`starting | healthy | degraded`). See I-0099 / T-0585 for the rationale: operators want a small, cross-component vocabulary; per-component nuance lives in /v1/health endpoints.

<details>
<summary>Source</summary>

```rust
    pub fn as_state_label(&self) -> &'static str {
        match self {
            Self::Starting | Self::Connecting => "starting",
            Self::Live | Self::SocketOnly => "healthy",
            Self::Disconnected => "degraded",
        }
    }
```

</details>

### <span id="enum.AccumulatorError"></span><code style="font-size: 0.85em">enum AccumulatorError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors from accumulator operations.

#### Variants

- **`Init`**
- **`Run`**
- **`Send`**
- **`Checkpoint`**

## Traits

### <span id="trait.Accumulator"></span><code style="font-size: 0.85em">trait Accumulator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + 'static`

An accumulator consumes events from a source and pushes boundaries to a reactor.

Two input paths:
- Event source (optional): an [`EventSource`](../../../../rust/cloacina/computation_graph/accumulator/#trait.EventSource) actively pulls from a source
- Socket receiver: events pushed in from outside (always active)

Both paths feed through `process()` which is called sequentially.

To add an active event loop, implement [`EventSource`](../../../../rust/cloacina/computation_graph/accumulator/#trait.EventSource) and pass it to
[`accumulator_runtime_with_source`](../../../../rust/cloacina/computation_graph/accumulator/#fn.accumulator_runtime_with_source). The processor owns `&mut self` for
`process()` while the event source runs independently on its own task.

#### Associated types

- `type Output: Serialize + Send + 'static` - The typed boundary produced for the reactor.

#### Methods

##### <span id="Accumulator.process"></span><code>process</code>

```rust
fn process(&mut self, event: Vec<u8>) -> Option<Self::Output>
```

Process raw event bytes and optionally produce a boundary. The implementor owns deserialization — the runtime is format-agnostic. Called sequentially by the processor task — no concurrent `&mut self`.

CLOACI-T-0739: a trait-level default body was investigated and rejected.
A `where Self::Output: DeserializeOwned` default cannot be called from
the generic runtime (`accumulator_runtime<A: Accumulator>`), which calls
`process` for *any* `A` whose `Output` is only bound by `Serialize`;
tightening that bound would break the format-agnostic contract. The
boilerplate-free passthrough path is instead the
`#[passthrough_accumulator]` macro, which generates this method.

<details>
<summary>Source</summary>

```rust
    fn process(&mut self, event: Vec<u8>) -> Option<Self::Output>;
```

</details>

##### <span id="Accumulator.init"></span><code>init</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn init(
    &mut self) -> Result<(,
    _ctx: &AccumulatorContext) -> Result<(,
) -> Result<(), AccumulatorError>
```

Called on startup before first receive. Use to restore state from last checkpoint.

<details>
<summary>Source</summary>

```rust
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
```

</details>

### <span id="trait.EventSource"></span><code style="font-size: 0.85em">trait EventSource</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + 'static`

An event source actively pulls events from an external source and pushes them into the accumulator's merge channel. Runs on its own tokio task, independently of the processor that calls [`Accumulator::process()`](../../../../rust/cloacina/computation_graph/accumulator/#Accumulator.process).

This is the correct way to add an active event loop to an accumulator.
The event source takes ownership (`self`, not `&mut self`) so it can
run concurrently with the processor without borrowing conflicts.

For stream-backed sources, see also [`StreamBackend`](../../../../rust/cloacina/computation_graph/stream_backend/#trait.StreamBackend).

#### Methods

##### <span id="EventSource.run"></span><code>run</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run(
    self,
    events: mpsc::Sender<Vec<u8>>,
    shutdown: watch::Receiver<bool>,
) -> Result<(), AccumulatorError>
```

Run the event loop. Push raw event bytes into `events` until shutdown fires or the source is exhausted. The runtime shuts down if this returns.

<details>
<summary>Source</summary>

```rust
    async fn run(
```

</details>

### <span id="trait.PollingAccumulator"></span><code style="font-size: 0.85em">trait PollingAccumulator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + 'static`

A polling accumulator periodically calls an async poll function to query pull-based data sources (databases, APIs, config files).

Returns `Option<Output>` — `Some` emits a boundary, `None` means "no change".

#### Associated types

- `type Output: Serialize + DeserializeOwned + Send + 'static` - The typed boundary produced for the reactor.

#### Methods

##### <span id="PollingAccumulator.poll"></span><code>poll</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn poll(&mut self) -> Option<Self::Output>
```

Poll the data source. Called on each timer tick. Return `Some(output)` to emit a boundary, `None` to skip.

<details>
<summary>Source</summary>

```rust
    async fn poll(&mut self) -> Option<Self::Output>;
```

</details>

##### <span id="PollingAccumulator.interval"></span><code>interval</code>

```rust
fn interval(&self) -> std::time::Duration
```

Polling interval.

<details>
<summary>Source</summary>

```rust
    fn interval(&self) -> std::time::Duration;
```

</details>

### <span id="trait.BatchAccumulator"></span><code style="font-size: 0.85em">trait BatchAccumulator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + 'static`

A batch accumulator buffers incoming events and processes them all at once on a flush signal. Emits a single boundary containing the batch result.

Flush triggers:
- Timer-based flush interval
- Buffer size threshold (optional)
- Shutdown (drains remaining buffer)

#### Associated types

- `type Output: Serialize + Send + 'static` - The typed boundary produced from the batch.

#### Methods

##### <span id="BatchAccumulator.process_batch"></span><code>process_​batch</code>

```rust
fn process_batch(
    &mut self,
    events: Vec<Vec<u8>>,
) -> Option<Self::Output>
```

Process a batch of raw event bytes and optionally produce a boundary. The implementor owns deserialization — the runtime is format-agnostic. Called when the buffer is flushed. Empty batches are never passed.

<details>
<summary>Source</summary>

```rust
    fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<Self::Output>;
```

</details>

## Functions

### <span id="fn.health_channel"></span><code style="font-size: 0.85em">fn health_​channel</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn health_channel() -> (
    watch::Sender<AccumulatorHealth>,
    watch::Receiver<AccumulatorHealth>,
)
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

### <span id="fn.accumulator_runtime"></span><code style="font-size: 0.85em">fn accumulator_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn accumulator_runtime<A: Accumulator>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
)
```

Run an accumulator as 2-3 tokio tasks connected by a merge channel.

Socket-only mode (no event source):
```text
┌─────────────────┐     ┌─────────────────┐
│  Socket task     │──→  │  Processor task  │──→ BoundarySender ──→ Reactor
│  (always active) │     │  (calls process) │
└─────────────────┘     └─────────────────┘
```

With event source (use [`accumulator_runtime_with_source`](../../../../rust/cloacina/computation_graph/accumulator/#fn.accumulator_runtime_with_source)):
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
    accumulator_runtime_inner::<A, NoEventSource>(acc, ctx, socket_rx, config, None).await
}
```

</details>

### <span id="fn.accumulator_runtime_with_source"></span><code style="font-size: 0.85em">fn accumulator_​runtime_​with_​source</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn accumulator_runtime_with_source<A, S>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
    source: S,
)
where
    A: Accumulator,
    S: EventSource,
```

Run an accumulator with an active event source that pulls events from an external system. The event source runs on its own task and pushes raw bytes into the merge channel concurrently with the socket receiver.

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
    S: EventSource,
{
    accumulator_runtime_inner(acc, ctx, socket_rx, config, Some(source)).await
}
```

</details>

### <span id="fn.shutdown_signal"></span><code style="font-size: 0.85em">fn shutdown_​signal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn shutdown_signal() -> (watch::Sender<bool>, watch::Receiver<bool>)
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

### <span id="fn.polling_accumulator_runtime"></span><code style="font-size: 0.85em">fn polling_​accumulator_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn polling_accumulator_runtime<P: PollingAccumulator>(
    mut poller: P,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
)
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
    // Polling accumulators don't buffer — emit a stable 0 series for dashboards.
    set_accumulator_buffer_depth(&ctx, 0.0);

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
                let emit_started = std::time::Instant::now();
                if let Some(output) = poller.poll().await {
                    if let Err(e) = ctx.output.send(&output).await {
                        tracing::error!(name = %ctx.name, "polling boundary send failed: {}", e);
                    } else {
                        persist_boundary(&ctx, &output).await;
                        // Checkpoint the last successful poll output
                        if let Some(ref handle) = ctx.checkpoint {
                            if let Err(e) = handle.save(&output).await {
                                tracing::warn!(name = %ctx.name, "polling checkpoint save failed: {}", e);
                                record_accumulator_persist_failure(&ctx, "checkpoint");
                            }
                        }
                    }
                    record_accumulator_event(&ctx, "polling", emit_started);
                }
            }
            Some(bytes) = socket_rx.recv() => {
                let emit_started = std::time::Instant::now();
                // Socket receives JSON from external sources
                match serde_json::from_slice::<P::Output>(&bytes) {
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
                record_accumulator_event(&ctx, "polling", emit_started);
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

### <span id="fn.flush_signal"></span><code style="font-size: 0.85em">fn flush_​signal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn flush_signal() -> (mpsc::Sender<()>, mpsc::Receiver<()>)
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

### <span id="fn.batch_accumulator_runtime"></span><code style="font-size: 0.85em">fn batch_​accumulator_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn batch_accumulator_runtime<B: BatchAccumulator>(
    mut acc: B,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    mut flush_rx: mpsc::Receiver<()>,
    config: BatchAccumulatorConfig,
)
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
) {
    set_health(&ctx, AccumulatorHealth::Starting);
    set_accumulator_buffer_depth(&ctx, 0.0);
    // CLOACI-T-0744: surface the bounded flush threshold as the buffer
    // capacity so the health API can render fill (`depth/capacity`).
    if let Some(cap) = config.max_buffer_size {
        ctx.output.set_buffer_capacity(cap as u64);
    }

    // Restore buffered events from checkpoint if available
    let mut buffer: Vec<Vec<u8>> = Vec::new();
    if let Some(ref handle) = ctx.checkpoint {
        match handle.load::<Vec<Vec<u8>>>().await {
            Ok(Some(raw_events)) => {
                buffer = raw_events;
                if !buffer.is_empty() {
                    tracing::info!(name = %ctx.name, events = buffer.len(), "batch buffer restored from checkpoint");
                    set_accumulator_buffer_depth(&ctx, buffer.len() as f64);
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
                buffer.push(bytes);
                set_accumulator_buffer_depth(&ctx, buffer.len() as f64);
                // Persist buffer snapshot for crash resilience
                persist_batch_buffer(&ctx, &buffer).await;
                // Check size threshold
                if let Some(max) = config.max_buffer_size {
                    if buffer.len() >= max {
                        flush_batch(&mut acc, &mut buffer, &ctx).await;
                        set_accumulator_buffer_depth(&ctx, 0.0);
                    }
                }
            }
            Some(()) = flush_rx.recv() => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                set_accumulator_buffer_depth(&ctx, 0.0);
                // Clear checkpoint after flush (buffer is empty)
                persist_batch_buffer(&ctx, &[]).await;
            }
            _ = async {
                match timer.as_mut() {
                    Some(t) => t.tick().await,
                    None => std::future::pending().await,
                }
            } => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                set_accumulator_buffer_depth(&ctx, 0.0);
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "batch accumulator shutting down, draining buffer");
                // Drain remaining buffer on shutdown
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                set_accumulator_buffer_depth(&ctx, 0.0);
                break;
            }
        }
    }
}
```

</details>

### <span id="fn.state_accumulator_runtime"></span><code style="font-size: 0.85em">fn state_​accumulator_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn state_accumulator_runtime<
    T: Serialize + DeserializeOwned + Send + Clone + 'static,
>(
    mut acc: StateAccumulator<T>,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
)
```

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
                // JSON first (the current write format — see the save site for
                // why bincode cannot work here), then bincode for rows written
                // by older builds with a concrete T. A row that parses as
                // NEITHER is reported loudly: the silent `if let Ok` this
                // replaces is exactly how packaged windows vanished on
                // takeover without a single log line.
                match serde_json::from_slice::<std::collections::VecDeque<T>>(&data)
                    .ok()
                    .or_else(|| types::deserialize::<std::collections::VecDeque<T>>(&data).ok())
                {
                    Some(buffer) => {
                        acc.buffer = buffer;
                        tracing::info!(name = %ctx.name, entries = acc.buffer.len(), "state accumulator restored from DAL");
                    }
                    None => {
                        tracing::warn!(
                            name = %ctx.name,
                            bytes = data.len(),
                            "persisted state buffer could not be decoded as JSON or legacy \
                             bincode — the window is LOST and will restart empty"
                        );
                    }
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
            match state_window_frame(&list) {
                Ok(frame) => {
                    if let Err(e) = ctx.output.send(&frame).await {
                        tracing::error!(name = %ctx.name, "state accumulator initial emit failed: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(name = %ctx.name, "state window encode failed: {}", e)
                }
            }
        }
    }

    // CLOACI-T-0744: state buffers were previously invisible to BOTH the
    // Prometheus gauge and the health API — report depth (incl. the restored
    // buffer) and the bounded capacity so the UI can render `N/capacity`.
    set_accumulator_buffer_depth(&ctx, acc.buffer.len() as f64);
    if acc.capacity > 0 {
        ctx.output.set_buffer_capacity(acc.capacity as u64);
    }

    set_health(&ctx, AccumulatorHealth::SocketOnly);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                // Socket receives JSON from external sources
                match serde_json::from_slice::<T>(&bytes) {
                    Ok(value) => {
                        // Append to buffer
                        acc.buffer.push_back(value);

                        // Evict if over capacity (bounded mode)
                        if acc.capacity > 0 {
                            while acc.buffer.len() > acc.capacity as usize {
                                acc.buffer.pop_front();
                            }
                        }
                        set_accumulator_buffer_depth(&ctx, acc.buffer.len() as f64);

                        // Persist to DAL
                        if let Some(ref handle) = ctx.checkpoint {
                            // serde_json, NOT the bincode used elsewhere
                            // (CLOACI-T-0851): the packaged path instantiates
                            // this runtime with T = serde_json::Value, and
                            // bincode can SERIALIZE a Value but can never
                            // DESERIALIZE one (Value requires deserialize_any,
                            // which bincode does not support). Bincode here
                            // meant packaged state windows were persisted in a
                            // format that could not be read back — write-only
                            // durability, found when a takeover restored an
                            // empty window on a live cluster. JSON is
                            // self-describing, so every T this runtime accepts
                            // round-trips.
                            let data = match serde_json::to_vec(&acc.buffer) {
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
                            match state_window_frame(&list) {
                                Ok(frame) => {
                                    if let Err(e) = ctx.output.send(&frame).await {
                                        tracing::error!(name = %ctx.name, "state accumulator emit failed: {}", e);
                                    } else {
                                        persist_boundary(&ctx, &list).await;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(name = %ctx.name, "state window encode failed: {}", e)
                                }
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
