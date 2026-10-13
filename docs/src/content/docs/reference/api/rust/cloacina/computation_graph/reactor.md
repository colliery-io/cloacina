---
title: "cloacina::computation_graph::reactor"
description: "Reactor — the long-lived process that wires accumulators to a compiled computation graph."
editUrl: false
sidebar:
  order: 45
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Reactor — the long-lived process that wires accumulators to a compiled computation graph.

Three concerns:
1. **Receiver**: accepts boundaries from accumulators, updates cache
2. **Strategy**: evaluates reaction criteria to decide when to fire
3. **Executor**: calls the compiled graph function

See CLOACI-S-0005 for the full specification.

## Re-exports

- [`ReactionCriteria`](../../../../rust/cloacina-computation-graph/#enum.ReactionMode) (`cloacina_computation_graph::ReactionMode`)
- `ReactorCommand` (`cloacina_api_types::reactor::ReactorCommand`)
- `ReactorResponse` (`cloacina_api_types::reactor::ReactorResponse`)
- [`CompiledGraphFn`](../../../../rust/cloacina-computation-graph/#type.CompiledGraphFn)

## Structs

### <span id="struct.DirtyFlags"></span><code style="font-size: 0.85em">struct DirtyFlags</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Dirty flags — one boolean per source.

#### Methods

##### <span id="DirtyFlags.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
    }
```

</details>

##### <span id="DirtyFlags.with_sources"></span><code>with_​sources</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_sources(sources: &[SourceName]) -> Self
```

Create dirty flags pre-seeded with expected source names (all initially false).

Required for `WhenAll` — ensures `all_set()` returns false until
every expected source has emitted, not just the sources seen so far.

<details>
<summary>Source</summary>

```rust
    pub fn with_sources(sources: &[SourceName]) -> Self {
        let mut flags = HashMap::new();
        for source in sources {
            flags.insert(source.clone(), false);
        }
        Self { flags }
    }
```

</details>

##### <span id="DirtyFlags.set"></span><code>set</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set(&mut self, source: SourceName, dirty: bool)
```

<details>
<summary>Source</summary>

```rust
    pub fn set(&mut self, source: SourceName, dirty: bool) {
        self.flags.insert(source, dirty);
    }
```

</details>

##### <span id="DirtyFlags.any_set"></span><code>any_​set</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn any_set(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn any_set(&self) -> bool {
        self.flags.values().any(|&v| v)
    }
```

</details>

##### <span id="DirtyFlags.all_set"></span><code>all_​set</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn all_set(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn all_set(&self) -> bool {
        !self.flags.is_empty() && self.flags.values().all(|&v| v)
    }
```

</details>

##### <span id="DirtyFlags.clear_all"></span><code>clear_​all</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn clear_all(&mut self)
```

<details>
<summary>Source</summary>

```rust
    pub fn clear_all(&mut self) {
        for v in self.flags.values_mut() {
            *v = false;
        }
    }
```

</details>

### <span id="struct.FireRecord"></span><code style="font-size: 0.85em">struct FireRecord</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Handle to a running reactor — exposes shared state for WebSocket queries.

Returned by `Reactor::handle()` before calling `run()`.
Operational counters for a reactor, shared between the running reactor and
its [`ReactorHandle`](../../../../rust/cloacina/computation_graph/reactor/#struct.ReactorHandle) so the scheduler/API can report live activity
(CLOACI-I-0124 / WS-10): total graph fires + the wall-clock time of the last
fire. Throughput is derived by the caller from successive `fires` samples.
One recorded reactor fire (CLOACI-T-0766) — the observable unit behind the
operational view's "Recent fires". Captures outcome + wall time + duration.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `fired_at_ms` | `i64` | Unix-epoch millis when the fire completed. |
| `ok` | `bool` | Whether the graph execution completed (false = errored). |
| `error` | `Option<String>` | Error detail for a failed fire. |
| `duration_ms` | `u64` | Graph execution wall-clock for this fire. |
| `inputs` | `std::collections::HashMap<String, serde_json::Value>` | Input boundary values that triggered this fire: source name → value (CLOACI-T-0775). The snapshot the graph ran on. |
| `outputs` | `Vec<serde_json::Value>` | Terminal outputs the graph produced, as JSON (CLOACI-T-0775). Empty when the executor can't serialize them (e.g. the Python reactor path) or on a failed fire. |
| `manual` | `bool` | Whether this fire was triggered by a manual operator command (`force_fire`/`fire_with`) rather than the reactor's criteria over real boundary events (CLOACI-T-0776). Surfaced so the UI can mark it. |

### <span id="struct.ReactorStats"></span><code style="font-size: 0.85em">struct ReactorStats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Default`

#### Methods

##### <span id="ReactorStats.record_fire"></span><code>record_​fire</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record_fire(
    &self,
    duration_ms: u64,
    inputs: std::collections::HashMap<String, serde_json::Value>,
    outputs: Vec<serde_json::Value>,
    manual: bool,
) -> u64
```

Record a successful graph fire with its I/O; returns the new completed total (CLOACI-T-0766; I/O CLOACI-T-0775).

<details>
<summary>Source</summary>

```rust
    pub fn record_fire(
        &self,
        duration_ms: u64,
        inputs: std::collections::HashMap<String, serde_json::Value>,
        outputs: Vec<serde_json::Value>,
        manual: bool,
    ) -> u64 {
        let now = chrono::Utc::now().timestamp_millis();
        self.last_fire_unix_ms
            .store(now, std::sync::atomic::Ordering::Relaxed);
        self.push(FireRecord {
            fired_at_ms: now,
            ok: true,
            error: None,
            duration_ms,
            inputs,
            outputs,
            manual,
        });
        self.fires
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }
```

</details>

##### <span id="ReactorStats.record_failure"></span><code>record_​failure</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record_failure(
    &self,
    duration_ms: u64,
    error: String,
    inputs: std::collections::HashMap<String, serde_json::Value>,
    manual: bool,
)
```

Record a failed graph fire (CLOACI-T-0766). Inputs are still captured so an operator can see what triggered the failing fire (CLOACI-T-0775).

<details>
<summary>Source</summary>

```rust
    pub fn record_failure(
        &self,
        duration_ms: u64,
        error: String,
        inputs: std::collections::HashMap<String, serde_json::Value>,
        manual: bool,
    ) {
        let now = chrono::Utc::now().timestamp_millis();
        self.last_fire_unix_ms
            .store(now, std::sync::atomic::Ordering::Relaxed);
        self.push(FireRecord {
            fired_at_ms: now,
            ok: false,
            error: Some(error),
            duration_ms,
            inputs,
            outputs: Vec::new(),
            manual,
        });
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
```

</details>

##### <span id="ReactorStats.fires"></span><code>fires</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn fires(&self) -> u64
```

Total successful fires so far.

<details>
<summary>Source</summary>

```rust
    pub fn fires(&self) -> u64 {
        self.fires.load(std::sync::atomic::Ordering::Relaxed)
    }
```

</details>

##### <span id="ReactorStats.failures"></span><code>failures</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn failures(&self) -> u64
```

Total failed fires so far.

<details>
<summary>Source</summary>

```rust
    pub fn failures(&self) -> u64 {
        self.failures.load(std::sync::atomic::Ordering::Relaxed)
    }
```

</details>

##### <span id="ReactorStats.last_fire_unix_ms"></span><code>last_​fire_​unix_​ms</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn last_fire_unix_ms(&self) -> Option<i64>
```

Unix-epoch millis of the last fire, or `None` if it hasn't fired yet.

<details>
<summary>Source</summary>

```rust
    pub fn last_fire_unix_ms(&self) -> Option<i64> {
        match self
            .last_fire_unix_ms
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            0 => None,
            v => Some(v),
        }
    }
```

</details>

##### <span id="ReactorStats.recent_fires"></span><code>recent_​fires</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn recent_fires(&self, limit: usize) -> Vec<FireRecord>
```

The most recent fires, newest first, up to `limit` (CLOACI-T-0766).

<details>
<summary>Source</summary>

```rust
    pub fn recent_fires(&self, limit: usize) -> Vec<FireRecord> {
        self.recent
            .lock()
            .map(|r| r.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }
```

</details>

##### <span id="ReactorStats.fire_timeseries"></span><code>fire_​timeseries</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn fire_timeseries(&self) -> Vec<u32>
```

Fires per minute for the last `FIRE_BUCKETS` minutes, oldest→newest, gaps filled with 0 and aligned to the current minute (CLOACI-T-0766).

<details>
<summary>Source</summary>

```rust
    pub fn fire_timeseries(&self) -> Vec<u32> {
        let now_min = chrono::Utc::now().timestamp_millis() / 60_000;
        let counts: std::collections::HashMap<i64, u32> = self
            .buckets
            .lock()
            .map(|b| b.iter().copied().collect())
            .unwrap_or_default();
        (0..FIRE_BUCKETS as i64)
            .map(|i| {
                let minute = now_min - (FIRE_BUCKETS as i64 - 1 - i);
                counts.get(&minute).copied().unwrap_or(0)
            })
            .collect()
    }
```

</details>

### <span id="struct.ReactorHandle"></span><code style="font-size: 0.85em">struct ReactorHandle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `cache` | `Arc<RwLock<InputCache>>` | Shared cache — readable by WebSocket handlers for GetState. |
| `paused` | `Arc<AtomicBool>` | Pause flag — when true, executor skips graph execution. |
| `stats` | `Arc<ReactorStats>` | Live fire counters (CLOACI-I-0124 / WS-10). |

#### Methods

##### <span id="ReactorHandle.get_state"></span><code>get_​state</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_state(&self) -> HashMap<String, String>
```

Read the current cache as a JSON-friendly map.

<details>
<summary>Source</summary>

```rust
    pub async fn get_state(&self) -> HashMap<String, String> {
        let cache = self.cache.read().await;
        cache.entries_as_json()
    }
```

</details>

##### <span id="ReactorHandle.stats"></span><code>stats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn stats(&self) -> (u64, Option<i64>)
```

Live operational counters: `(total_fires, last_fire_unix_ms)`.

<details>
<summary>Source</summary>

```rust
    pub fn stats(&self) -> (u64, Option<i64>) {
        (self.stats.fires(), self.stats.last_fire_unix_ms())
    }
```

</details>

##### <span id="ReactorHandle.is_paused"></span><code>is_​paused</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_paused(&self) -> bool
```

Check if the reactor is paused.

<details>
<summary>Source</summary>

```rust
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }
```

</details>

##### <span id="ReactorHandle.pause"></span><code>pause</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn pause(&self)
```

Pause the reactor (stop executing, continue accepting boundaries).

<details>
<summary>Source</summary>

```rust
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }
```

</details>

##### <span id="ReactorHandle.resume"></span><code>resume</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn resume(&self)
```

Resume the reactor.

<details>
<summary>Source</summary>

```rust
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
```

</details>

### <span id="struct.Reactor"></span><code style="font-size: 0.85em">struct Reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

The Reactor.

#### Methods

##### <span id="Reactor.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    graph: CompiledGraphFn,
    criteria: ReactionCriteria,
    input_strategy: InputStrategy,
    accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,
    manual_rx: mpsc::Receiver<ManualCommand>,
    shutdown: watch::Receiver<bool>,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        graph: CompiledGraphFn,
        criteria: ReactionCriteria,
        input_strategy: InputStrategy,
        accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,
        manual_rx: mpsc::Receiver<ManualCommand>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            graph,
            criteria,
            input_strategy,
            accumulator_rx,
            manual_rx,
            shutdown,
            cache: Arc::new(RwLock::new(InputCache::new())),
            paused: Arc::new(AtomicBool::new(false)),
            expected_sources: Vec::new(),
            graph_name: String::new(),
            dal: None,
            tenant_id: None,
            health: None,
            accumulator_health_rxs: Vec::new(),
            batch_flush_senders: Vec::new(),
            stats: Arc::new(ReactorStats::default()),
            evaluator: None,
            graph_executor: super::graph_executor::in_process_graph_executor(),
        }
    }
```

</details>

##### <span id="Reactor.with_graph_executor"></span><code>with_​graph_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_graph_executor(
    mut self,
    executor: Arc<dyn super::graph_executor::GraphExecutor>,
) -> Self
```

Set where firings execute (CLOACI-T-0722): the server injects its fleet executor here when `--default-executor fleet`; embedded and default paths keep the in-process executor.

<details>
<summary>Source</summary>

```rust
    pub fn with_graph_executor(
        mut self,
        executor: Arc<dyn super::graph_executor::GraphExecutor>,
    ) -> Self {
        self.graph_executor = executor;
        self
    }
```

</details>

##### <span id="Reactor.with_evaluator"></span><code>with_​evaluator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_evaluator(
    mut self,
    evaluator: Arc<dyn ReactorFireDecider>,
) -> Self
```

Set a pluggable firing-decision hook (CLOACI-T-0828).

When present, the executor consults `evaluator.should_fire(snapshot)` on
every received boundary (the `Latest` path) instead of the built-in
`WhenAny`/`WhenAll` dirty-flag criteria — so a WASM reactor constructor's
`evaluate` becomes the reactor's firing criteria.

<details>
<summary>Source</summary>

```rust
    pub fn with_evaluator(mut self, evaluator: Arc<dyn ReactorFireDecider>) -> Self {
        self.evaluator = Some(evaluator);
        self
    }
```

</details>

##### <span id="Reactor.with_batch_flush_senders"></span><code>with_​batch_​flush_​senders</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_batch_flush_senders(
    mut self,
    senders: Vec<mpsc::Sender<()>>,
) -> Self
```

Add batch flush senders — reactor will signal these after each graph execution.

<details>
<summary>Source</summary>

```rust
    pub fn with_batch_flush_senders(mut self, senders: Vec<mpsc::Sender<()>>) -> Self {
        self.batch_flush_senders = senders;
        self
    }
```

</details>

##### <span id="Reactor.with_graph_name"></span><code>with_​graph_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_graph_name(mut self, name: String) -> Self
```

Set the graph name (used as key for DAL persistence).

<details>
<summary>Source</summary>

```rust
    pub fn with_graph_name(mut self, name: String) -> Self {
        self.graph_name = name;
        self
    }
```

</details>

##### <span id="Reactor.with_dal"></span><code>with_​dal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_dal(mut self, dal: crate::dal::unified::DAL) -> Self
```

Set the DAL handle for cache persistence.

<details>
<summary>Source</summary>

```rust
    pub fn with_dal(mut self, dal: crate::dal::unified::DAL) -> Self {
        self.dal = Some(dal);
        self
    }
```

</details>

##### <span id="Reactor.with_tenant_id"></span><code>with_​tenant_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_tenant_id(mut self, tenant_id: Option<String>) -> Self
```

Set the tenant scope for reactor firings (CLOACI-I-0100 / T-0599).

<details>
<summary>Source</summary>

```rust
    pub fn with_tenant_id(mut self, tenant_id: Option<String>) -> Self {
        self.tenant_id = tenant_id;
        self
    }
```

</details>

##### <span id="Reactor.with_health"></span><code>with_​health</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_health(mut self, health: watch::Sender<ReactorHealth>) -> Self
```

Set the health reporter channel.

<details>
<summary>Source</summary>

```rust
    pub fn with_health(mut self, health: watch::Sender<ReactorHealth>) -> Self {
        self.health = Some(health);
        self
    }
```

</details>

##### <span id="Reactor.with_expected_sources"></span><code>with_​expected_​sources</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_expected_sources(mut self, sources: Vec<SourceName>) -> Self
```

Set the expected source names for WhenAll criteria.

Seeds DirtyFlags so `all_set()` correctly requires all sources to emit
before firing, not just the sources seen so far.

<details>
<summary>Source</summary>

```rust
    pub fn with_expected_sources(mut self, sources: Vec<SourceName>) -> Self {
        self.expected_sources = sources;
        self
    }
```

</details>

##### <span id="Reactor.with_accumulator_health"></span><code>with_​accumulator_​health</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_accumulator_health(
    mut self,
    rxs: Vec<(String, watch::Receiver<super::accumulator::AccumulatorHealth>)>,
) -> Self
```

Set accumulator health receivers for startup gating and degraded mode.

<details>
<summary>Source</summary>

```rust
    pub fn with_accumulator_health(
        mut self,
        rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )>,
    ) -> Self {
        self.accumulator_health_rxs = rxs;
        self
    }
```

</details>

##### <span id="Reactor.handle"></span><code>handle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn handle(&self) -> ReactorHandle
```

Get a handle to this reactor's shared state.

Call before `run()` to get a handle that WebSocket handlers can use
for GetState, Pause, and Resume operations.

<details>
<summary>Source</summary>

```rust
    pub fn handle(&self) -> ReactorHandle {
        ReactorHandle {
            cache: self.cache.clone(),
            paused: self.paused.clone(),
            stats: self.stats.clone(),
        }
    }
```

</details>

##### <span id="Reactor.run"></span><code>run</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run(mut self)
```

Run the reactor. Spawns receiver + executor tasks.

<details>
<summary>Source</summary>

```rust
    pub async fn run(mut self) {
        // Compute the bounded `strategy` metric label once per run. Mapping
        // collapses the two-axis (criteria × input_strategy) design into the
        // three values documented for `cloacina_reactor_fires_total`:
        // Sequential ⇒ "sequential", Latest+WhenAny ⇒ "when_any",
        // Latest+WhenAll ⇒ "when_all".
        let strategy_label: &'static str = match (&self.input_strategy, &self.criteria) {
            (InputStrategy::Sequential, _) => "sequential",
            (InputStrategy::Latest, ReactionCriteria::WhenAny) => "when_any",
            (InputStrategy::Latest, ReactionCriteria::WhenAll) => "when_all",
        };

        // Report starting health
        if let Some(ref health) = self.health {
            let _ = health.send(ReactorHealth::Starting);
        }

        // Load cache from DAL if available (instant recovery). Dirty flags
        // and the sequential queue are restored too (CLOACI-T-0915): they
        // were persisted before every drain precisely so a crash mid-drain
        // doesn't lose items, but the restore path used to discard both —
        // making Sequential's no-loss promise and WhenAll's accumulated
        // dirty state per-process-lifetime only.
        let cache = self.cache.clone();
        let mut restored_dirty: Option<HashMap<SourceName, bool>> = None;
        let mut restored_seq: Option<VecDeque<(SourceName, Vec<u8>)>> = None;
        if let Some(ref dal) = self.dal {
            if !self.graph_name.is_empty() {
                match dal.checkpoint().load_reactor_state(&self.graph_name).await {
                    Ok(Some((cache_data, dirty_data, seq_data))) => {
                        // Restore cache — deserialize the entries map
                        if let Ok(entries) =
                            serde_json::from_slice::<HashMap<SourceName, Vec<u8>>>(&cache_data)
                        {
                            let mut c = cache.write().await;
                            for (source, bytes) in entries {
                                c.update(source, bytes);
                            }
                            tracing::info!(graph = %self.graph_name, "reactor cache restored from DAL");
                        }
                        match serde_json::from_slice::<HashMap<SourceName, bool>>(&dirty_data) {
                            Ok(flags) => restored_dirty = Some(flags),
                            Err(e) => {
                                tracing::warn!(graph = %self.graph_name, "persisted dirty flags unreadable (starting clean): {}", e);
                            }
                        }
                        if let Some(seq_bytes) = seq_data {
                            match serde_json::from_slice::<VecDeque<(SourceName, Vec<u8>)>>(
                                &seq_bytes,
                            ) {
                                Ok(q) if !q.is_empty() => {
                                    tracing::info!(
                                        graph = %self.graph_name,
                                        items = q.len(),
                                        "sequential queue restored from DAL"
                                    );
                                    restored_seq = Some(q);
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::warn!(graph = %self.graph_name, "persisted sequential queue unreadable (starting empty): {}", e);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!(graph = %self.graph_name, "no persisted reactor state found");
                    }
                    Err(e) => {
                        tracing::warn!(graph = %self.graph_name, "failed to load reactor state: {}", e);
                    }
                }
            }
        }

        let dirty = {
            let mut flags = if self.expected_sources.is_empty() {
                DirtyFlags::new()
            } else {
                DirtyFlags::with_sources(&self.expected_sources)
            };
            if let Some(restored) = restored_dirty {
                // Overlay restored flags. When an expected-source list exists,
                // only overlay sources still expected — a stale flag for a
                // de-scoped source must not poison `all_set()`.
                for (source, set) in restored {
                    if self.expected_sources.is_empty() || flags.flags.contains_key(&source) {
                        flags.set(source, set);
                    }
                }
            }
            Arc::new(RwLock::new(flags))
        };

        // Startup gating — wait for all accumulators to become healthy before going Live
        if !self.accumulator_health_rxs.is_empty() {
            use super::accumulator::AccumulatorHealth;

            let all_names: Vec<String> = self
                .accumulator_health_rxs
                .iter()
                .map(|(n, _)| n.clone())
                .collect();
            let mut healthy_set: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            // Report Warming
            if let Some(ref health) = self.health {
                let _ = health.send(ReactorHealth::Warming {
                    healthy: vec![],
                    waiting: all_names.clone(),
                });
            }

            let mut shutdown_gate = self.shutdown.clone();

            // Wait until all accumulators are healthy or shutdown fires
            'gating: loop {
                // Check current state of all receivers
                for (name, rx) in &self.accumulator_health_rxs {
                    let h = rx.borrow().clone();
                    match h {
                        AccumulatorHealth::Live | AccumulatorHealth::SocketOnly => {
                            healthy_set.insert(name.clone());
                        }
                        _ => {}
                    }
                }

                if healthy_set.len() >= all_names.len() {
                    break 'gating;
                }

                // Wait for any health change or shutdown
                tokio::select! {
                    _ = shutdown_gate.changed() => {
                        tracing::debug!(graph = %self.graph_name, "shutdown during startup gating");
                        return;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Poll again — watch receivers update in place
                    }
                }

                // Update Warming status
                if let Some(ref health) = self.health {
                    let waiting: Vec<String> = all_names
                        .iter()
                        .filter(|n| !healthy_set.contains(*n))
                        .cloned()
                        .collect();
                    let _ = health.send(ReactorHealth::Warming {
                        healthy: healthy_set.iter().cloned().collect(),
                        waiting,
                    });
                }
            }
        }

        // All accumulators healthy — go Live
        if let Some(ref health) = self.health {
            let _ = health.send(ReactorHealth::Live);
        }

        // Spawn degraded-mode monitor — watches accumulator health and toggles
        // between Live and Degraded based on accumulator disconnections.
        let _degraded_monitor = if let Some(ref health) = self.health {
            let health_tx = health.clone();
            let acc_rxs = std::mem::take(&mut self.accumulator_health_rxs);
            let mut shutdown_mon = self.shutdown.clone();
            let graph_name = self.graph_name.clone();
            Some(tokio::spawn(async move {
                use super::accumulator::AccumulatorHealth;
                if acc_rxs.is_empty() {
                    // No accumulators to monitor — just wait for shutdown
                    let _ = shutdown_mon.changed().await;
                    return;
                }
                loop {
                    tokio::select! {
                        _ = shutdown_mon.changed() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                            let disconnected: Vec<String> = acc_rxs
                                .iter()
                                .filter(|(_, rx)| {
                                    matches!(*rx.borrow(), AccumulatorHealth::Disconnected)
                                })
                                .map(|(name, _)| name.clone())
                                .collect();

                            if disconnected.is_empty() {
                                // All healthy — ensure we're Live (not Degraded)
                                if matches!(&*health_tx.borrow(), ReactorHealth::Degraded { .. }) {
                                    tracing::info!(graph = %graph_name, "all accumulators recovered — back to Live");
                                    let _ = health_tx.send(ReactorHealth::Live);
                                }
                            } else {
                                // Some disconnected — enter/stay Degraded
                                if !matches!(&*health_tx.borrow(), ReactorHealth::Degraded { .. }) {
                                    tracing::warn!(graph = %graph_name, ?disconnected, "accumulator(s) disconnected — entering Degraded mode");
                                }
                                let _ = health_tx.send(ReactorHealth::Degraded { disconnected });
                            }
                        }
                    }
                }
            }))
        } else {
            None
        };

        let paused = self.paused.clone();
        let input_strategy = self.input_strategy.clone();

        // Sequential queue — only used when InputStrategy::Sequential
        // Seed with any queue restored from the last checkpoint (T-0915):
        // items persisted before a crash mid-drain resume from here.
        let seq_queue: SeqQueue = Arc::new(RwLock::new(restored_seq.unwrap_or_default()));

        let (strategy_tx, mut strategy_rx) = mpsc::channel::<StrategySignal>(64);

        // Per-source last-arrival timestamps for `cloacina_reactor_cache_age_seconds`.
        // Shared between receiver and executor so both tasks can refresh the
        // gauge — receiver on every boundary, executor before each fire.
        let last_received_at: Arc<RwLock<HashMap<SourceName, std::time::Instant>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Spawn receiver task
        let cache_recv = cache.clone();
        let dirty_recv = dirty.clone();
        let seq_queue_recv = seq_queue.clone();
        let input_strategy_recv = input_strategy.clone();
        let mut shutdown_recv = self.shutdown.clone();
        let mut accumulator_rx = self.accumulator_rx;
        let mut manual_rx = self.manual_rx;
        let strategy_tx_recv = strategy_tx.clone();
        let last_received_recv = last_received_at.clone();
        let graph_name_recv = self.graph_name.clone();

        let receiver_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some((source, bytes)) = accumulator_rx.recv() => {
                        // Record arrival timestamp + refresh `cache_age_seconds`
                        // for every known source. Refreshing all sources on
                        // every arrival is how silent sources show staleness
                        // in dashboards without needing a separate periodic
                        // ticker.
                        let now = std::time::Instant::now();
                        {
                            let mut map = last_received_recv.write().await;
                            map.insert(source.clone(), now);
                            for (src, ts) in map.iter() {
                                let age = now.saturating_duration_since(*ts).as_secs_f64();
                                metrics::gauge!(
                                    "cloacina_reactor_cache_age_seconds",
                                    "graph" => graph_name_recv.clone(),
                                    "reactor" => graph_name_recv.clone(),
                                    "source" => src.as_str().to_string(),
                                )
                                .set(age);
                            }
                        }
                        match input_strategy_recv {
                            InputStrategy::Latest => {
                                cache_recv.write().await.update(source.clone(), bytes);
                                dirty_recv.write().await.set(source, true);
                            }
                            InputStrategy::Sequential => {
                                // Queue the boundary — don't update cache yet
                                seq_queue_recv.write().await.push_back((source, bytes));
                            }
                        }
                        let _ = strategy_tx_recv.send(StrategySignal::BoundaryReceived).await;
                    }
                    Some(cmd) = manual_rx.recv() => {
                        match cmd {
                            ManualCommand::ForceFire => {
                                let _ = strategy_tx_recv.send(StrategySignal::ForceFire).await;
                            }
                            ManualCommand::FireWith(new_cache) => {
                                cache_recv.write().await.replace_all(new_cache);
                                let _ = strategy_tx_recv.send(StrategySignal::ForceFire).await;
                            }
                        }
                    }
                    _ = shutdown_recv.changed() => {
                        tracing::debug!("reactor receiver shutting down");
                        break;
                    }
                }
            }
        });

        // Executor runs on current task
        let cache_exec = cache.clone();
        let dirty_exec = dirty.clone();
        let seq_queue_exec = seq_queue.clone();
        let mut shutdown_exec = self.shutdown.clone();
        let graph = self.graph.clone();
        let criteria = self.criteria.clone();
        let dal_exec = self.dal.clone();
        let tenant_id_exec = self.tenant_id.clone();
        let graph_name_exec = self.graph_name.clone();
        let batch_flush = self.batch_flush_senders.clone();
        // Shared with the ReactorHandle so the scheduler/API can report live
        // fire counts + last-fire time (WS-10).
        let fire_counter = self.stats.clone();
        // Consecutive persist failures — drives the I-0108 / T-0590
        // watchdog that flips the reactor to `Degraded` after 5 in a row.
        let persist_streak = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let health_exec = self.health.clone();
        // CLOACI-T-0828: when set, replaces the WhenAny/WhenAll criteria on the
        // Latest path with an external firing decision (a WASM reactor `evaluate`).
        let evaluator = self.evaluator.clone();
        // CLOACI-T-0722: firings run through the pluggable executor (in-process
        // by default; the fleet executor ships them to an agent).
        let graph_executor = self.graph_executor.clone();

        loop {
            tokio::select! {
                Some(signal) = strategy_rx.recv() => {
                    // CLOACI-T-0776: a ForceFire signal is only sent for a manual
                    // operator command (force_fire/fire_with), so the resulting
                    // fire is a manual intervention.
                    let is_manual = matches!(signal, StrategySignal::ForceFire);
                    match input_strategy {
                        InputStrategy::Latest => {
                            let should_run = match signal {
                                StrategySignal::BoundaryReceived => {
                                    // CLOACI-T-0828: a plugged-in evaluator (e.g. a
                                    // WASM reactor `evaluate`) decides firing when
                                    // present; otherwise the dirty-flag criteria do.
                                    if let Some(ref ev) = evaluator {
                                        let snap = cache_exec.read().await.snapshot();
                                        ev.should_fire(&snap).await
                                    } else {
                                        let d = dirty_exec.read().await;
                                        match &criteria {
                                            ReactionCriteria::WhenAny => d.any_set(),
                                            ReactionCriteria::WhenAll => d.all_set(),
                                        }
                                    }
                                }
                                StrategySignal::ForceFire => true,
                            };

                            if should_run && !paused.load(Ordering::SeqCst) {
                                let snapshot = cache_exec.read().await.snapshot();
                                dirty_exec.write().await.clear_all();
                                let fire_started = std::time::Instant::now();
                                write_reactor_firing(
                                    &dal_exec, &tenant_id_exec, &graph_name_exec, &snapshot,
                                ).await;
                                // CLOACI-T-0775: capture the triggering inputs before
                                // the snapshot is moved into the graph call.
                                let fire_inputs = capture_fire_inputs(&snapshot);
                                let result = graph_executor
                                    .execute(super::graph_executor::GraphFireEvent {
                                        graph_name: graph_name_exec.clone(),
                                        tenant_id: tenant_id_exec.clone(),
                                        snapshot,
                                        in_process: graph.clone(),
                                    })
                                    .await;
                                metrics::counter!(
                                    "cloacina_reactor_fires_total",
                                    "graph" => graph_name_exec.clone(),
                                    "reactor" => graph_name_exec.clone(),
                                    "strategy" => strategy_label,
                                )
                                .increment(1);
                                metrics::histogram!(
                                    "cloacina_reactor_fire_duration_seconds",
                                    "graph" => graph_name_exec.clone(),
                                    "reactor" => graph_name_exec.clone(),
                                )
                                .record(fire_started.elapsed().as_secs_f64());
                                match &result {
                                    GraphResult::Completed { outputs_json, .. } => {
                                        let fires = fire_counter.record_fire(
                                            fire_started.elapsed().as_millis() as u64,
                                            fire_inputs,
                                            outputs_json.clone(),
                                            is_manual,
                                        );
                                        tracing::info!(graph = %graph_name_exec, fires, "graph execution completed");
                                        persist_reactor_state(
                                            &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec, None,
                                            &persist_streak, &health_exec,
                                        ).await;
                                        for sender in &batch_flush {
                                            let _ = sender.try_send(());
                                        }
                                    }
                                    GraphResult::Error(e) => {
                                        fire_counter.record_failure(
                                            fire_started.elapsed().as_millis() as u64,
                                            e.to_string(),
                                            fire_inputs,
                                            is_manual,
                                        );
                                        tracing::error!(graph = %graph_name_exec, "graph execution failed: {}", e);
                                    }
                                }
                            }
                        }
                        InputStrategy::Sequential => {
                            if paused.load(Ordering::SeqCst) {
                                continue;
                            }
                            // Persist queue BEFORE draining so crash mid-drain doesn't lose items
                            persist_reactor_state(
                                &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec,
                                Some(&seq_queue_exec),
                                &persist_streak, &health_exec,
                            ).await;
                            // Drain the queue — one execution per queued boundary
                            loop {
                                let item = seq_queue_exec.write().await.pop_front();
                                match item {
                                    Some((source, bytes)) => {
                                        cache_exec.write().await.update(source, bytes);
                                        let snapshot = cache_exec.read().await.snapshot();
                                        let fire_started = std::time::Instant::now();
                                        write_reactor_firing(
                                            &dal_exec, &tenant_id_exec, &graph_name_exec, &snapshot,
                                        ).await;
                                        // CLOACI-T-0775: capture inputs before the move.
                                        let fire_inputs = capture_fire_inputs(&snapshot);
                                        let result = graph_executor
                                            .execute(super::graph_executor::GraphFireEvent {
                                                graph_name: graph_name_exec.clone(),
                                                tenant_id: tenant_id_exec.clone(),
                                                snapshot,
                                                in_process: graph.clone(),
                                            })
                                            .await;
                                        metrics::counter!(
                                            "cloacina_reactor_fires_total",
                                            "graph" => graph_name_exec.clone(),
                                            "reactor" => graph_name_exec.clone(),
                                            "strategy" => strategy_label,
                                        )
                                        .increment(1);
                                        metrics::histogram!(
                                            "cloacina_reactor_fire_duration_seconds",
                                            "graph" => graph_name_exec.clone(),
                                            "reactor" => graph_name_exec.clone(),
                                        )
                                        .record(fire_started.elapsed().as_secs_f64());
                                        match &result {
                                            GraphResult::Completed { outputs_json, .. } => {
                                                let fires = fire_counter.record_fire(
                                                    fire_started.elapsed().as_millis() as u64,
                                                    fire_inputs,
                                                    outputs_json.clone(),
                                                    is_manual,
                                                );
                                                tracing::info!(graph = %graph_name_exec, fires, "graph execution completed");
                                                persist_reactor_state(
                                                    &dal_exec, &graph_name_exec, &cache_exec,
                                                    &dirty_exec, Some(&seq_queue_exec),
                                                    &persist_streak, &health_exec,
                                                ).await;
                                                // Signal batch accumulators to flush
                                                for sender in &batch_flush {
                                                    let _ = sender.try_send(());
                                                }
                                            }
                                            GraphResult::Error(e) => {
                                                fire_counter.record_failure(
                                                    fire_started.elapsed().as_millis() as u64,
                                                    e.to_string(),
                                                    fire_inputs,
                                                    is_manual,
                                                );
                                                tracing::error!("graph execution failed: {}", e);
                                            }
                                        }
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                }
                _ = shutdown_exec.changed() => {
                    tracing::debug!("reactor executor shutting down");
                    // Final persist on orderly shutdown
                    persist_reactor_state(
                        &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec,
                        Some(&seq_queue_exec),
                        &persist_streak, &health_exec,
                    ).await;
                    break;
                }
            }
        }

        // Wait for receiver to finish
        let _ = receiver_handle.await;
    }
```

</details>

## Enums

### <span id="enum.ReactorHealth"></span><code style="font-size: 0.85em">enum ReactorHealth</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Health state of a reactor.

#### Variants

- **`Starting`** - Loading cache from DAL, spawning accumulators.
- **`Warming`** - Some accumulators healthy, waiting for all.
- **`Live`** - All accumulators healthy, evaluating criteria.
- **`Degraded`** - Was live, an accumulator disconnected. Running with stale data.

#### Methods

##### <span id="ReactorHealth.as_state_label"></span><code>as_​state_​label</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_state_label(&self) -> &'static str
```

Project the ReactorHealth state machine onto the bounded `cloacina_component_health` gauge label values (`starting | healthy | degraded`). See I-0099 / T-0585.

<details>
<summary>Source</summary>

```rust
    pub fn as_state_label(&self) -> &'static str {
        match self {
            Self::Starting | Self::Warming { .. } => "starting",
            Self::Live => "healthy",
            Self::Degraded { .. } => "degraded",
        }
    }
```

</details>

### <span id="enum.InputStrategy"></span><code style="font-size: 0.85em">enum InputStrategy</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Input strategy — how the reactor handles data between executions.

#### Variants

- **`Latest`** - One slot per source, overwritten on each update. Always fires with freshest.
- **`Sequential`** - Boundaries preserved in order, one execution per boundary.

### <span id="enum.StrategySignal"></span><code style="font-size: 0.85em">enum StrategySignal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Signals sent from receiver to executor.

#### Variants

- **`BoundaryReceived`** - A boundary was received — check reaction criteria.
- **`ForceFire`** - Force-fire regardless of criteria (only sent for a manual operator
  command — `force_fire`/`fire_with`). CLOACI-T-0776.

### <span id="enum.ManualCommand"></span><code style="font-size: 0.85em">enum ManualCommand</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Manual commands accepted by the reactor.

#### Variants

- **`ForceFire`** - Fire with current cache state.
- **`FireWith`** - Fire with injected state (replaces cache).

## Traits

### <span id="trait.ReactorFireDecider"></span><code style="font-size: 0.85em">trait ReactorFireDecider</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

A pluggable firing-decision hook for a [`Reactor`](../../../../rust/cloacina/computation_graph/reactor/#struct.Reactor) (CLOACI-T-0828).

When a reactor is built [`with_evaluator`](../../../../rust/cloacina/computation_graph/reactor/#Reactor.with_evaluator), its
executor consults this decider on every received boundary INSTEAD of the
built-in `WhenAny`/`WhenAll` dirty-flag criteria — letting an external
evaluator (e.g. a WASM reactor constructor's sync `evaluate`, bridged by
`WasmReactorConstructor`) decide whether to fire the graph. Only consulted on
the [`InputStrategy::Latest`](../../../../rust/cloacina/computation_graph/reactor/#enum.InputStrategy) path; a manual `ForceFire` still fires
unconditionally.

#### Methods

##### <span id="ReactorFireDecider.should_fire"></span><code>should_​fire</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn should_fire(&self, snapshot: &InputCache) -> bool
```

Given the reactor's current boundary cache snapshot, decide whether to fire the downstream graph this evaluation.

<details>
<summary>Source</summary>

```rust
    async fn should_fire(&self, snapshot: &InputCache) -> bool;
```

</details>

## Functions

### <span id="fn.reactor_health_channel"></span><code style="font-size: 0.85em">fn reactor_​health_​channel</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn reactor_health_channel() -> (
    watch::Sender<ReactorHealth>,
    watch::Receiver<ReactorHealth>,
)
```

Create a reactor health reporting channel.

<details>
<summary>Source</summary>

```rust
pub fn reactor_health_channel() -> (watch::Sender<ReactorHealth>, watch::Receiver<ReactorHealth>) {
    watch::channel(ReactorHealth::Starting)
}
```

</details>
