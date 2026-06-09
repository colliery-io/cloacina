# cloacina::computation_graph::reactor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Reactor — the long-lived process that wires accumulators to a compiled computation graph.

Three concerns:
1. **Receiver**: accepts boundaries from accumulators, updates cache
2. **Strategy**: evaluates reaction criteria to decide when to fire
3. **Executor**: calls the compiled graph function
See CLOACI-S-0005 for the full specification.

## Structs

### `cloacina::computation_graph::reactor::DirtyFlags`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Dirty flags — one boolean per source.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `flags` | `HashMap < SourceName , bool >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
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



##### `with_sources` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_sources (sources : & [SourceName]) -> Self
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



##### `set` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set (& mut self , source : SourceName , dirty : bool)
```

<details>
<summary>Source</summary>

```rust
    pub fn set(&mut self, source: SourceName, dirty: bool) {
        self.flags.insert(source, dirty);
    }
```

</details>



##### `any_set` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn any_set (& self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn any_set(&self) -> bool {
        self.flags.values().any(|&v| v)
    }
```

</details>



##### `all_set` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn all_set (& self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn all_set(&self) -> bool {
        !self.flags.is_empty() && self.flags.values().all(|&v| v)
    }
```

</details>



##### `clear_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn clear_all (& mut self)
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





### `cloacina::computation_graph::reactor::ReactorHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Handle to a running reactor — exposes shared state for WebSocket queries.

Returned by `Reactor::handle()` before calling `run()`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `cache` | `Arc < RwLock < InputCache > >` | Shared cache — readable by WebSocket handlers for GetState. |
| `paused` | `Arc < AtomicBool >` | Pause flag — when true, executor skips graph execution. |

#### Methods

##### `get_state` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_state (& self) -> HashMap < String , String >
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



##### `is_paused` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_paused (& self) -> bool
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



##### `pause` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pause (& self)
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



##### `resume` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn resume (& self)
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





### `cloacina::computation_graph::reactor::Reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


The Reactor.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph` | `CompiledGraphFn` | The compiled graph function to call. |
| `criteria` | `ReactionCriteria` | Reaction criteria. |
| `input_strategy` | `InputStrategy` | Input strategy. |
| `accumulator_rx` | `mpsc :: Receiver < (SourceName , Vec < u8 >) >` | Channel receiving boundaries from accumulators. |
| `manual_rx` | `mpsc :: Receiver < ManualCommand >` | Channel for manual operations. |
| `shutdown` | `watch :: Receiver < bool >` | Shutdown signal. |
| `cache` | `Arc < RwLock < InputCache > >` | Shared cache (also accessible via ReactorHandle). |
| `paused` | `Arc < AtomicBool >` | Pause flag (also accessible via ReactorHandle). |
| `expected_sources` | `Vec < SourceName >` | Expected source names (used to seed DirtyFlags for WhenAll). |
| `graph_name` | `String` | Graph name (for DAL persistence keying). |
| `dal` | `Option < crate :: dal :: unified :: DAL >` | DAL handle for cache persistence. None in embedded mode. |
| `tenant_id` | `Option < String >` | Tenant scope for the reactor's package, used as the
`reactor_firings.tenant_id` row key (CLOACI-I-0100 / T-0599).
`None` for cross-tenant or embedded reactors. |
| `health` | `Option < watch :: Sender < ReactorHealth > >` | Health state reporter. None when health tracking not needed. |
| `accumulator_health_rxs` | `Vec < (String , watch :: Receiver < super :: accumulator :: AccumulatorHealth > ,) >` | Accumulator health receivers for startup gating and degraded mode detection. |
| `batch_flush_senders` | `Vec < mpsc :: Sender < () > >` | Flush senders for batch accumulators — signalled after graph execution. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (graph : CompiledGraphFn , criteria : ReactionCriteria , input_strategy : InputStrategy , accumulator_rx : mpsc :: Receiver < (SourceName , Vec < u8 >) > , manual_rx : mpsc :: Receiver < ManualCommand > , shutdown : watch :: Receiver < bool > ,) -> Self
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
        }
    }
```

</details>



##### `with_batch_flush_senders` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_batch_flush_senders (mut self , senders : Vec < mpsc :: Sender < () > >) -> Self
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



##### `with_graph_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_graph_name (mut self , name : String) -> Self
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



##### `with_dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_dal (mut self , dal : crate :: dal :: unified :: DAL) -> Self
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



##### `with_tenant_id` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_tenant_id (mut self , tenant_id : Option < String >) -> Self
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



##### `with_health` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_health (mut self , health : watch :: Sender < ReactorHealth >) -> Self
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



##### `with_expected_sources` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_expected_sources (mut self , sources : Vec < SourceName >) -> Self
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



##### `with_accumulator_health` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_accumulator_health (mut self , rxs : Vec < (String , watch :: Receiver < super :: accumulator :: AccumulatorHealth > ,) > ,) -> Self
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



##### `handle` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn handle (& self) -> ReactorHandle
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
        }
    }
```

</details>



##### `run` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run (mut self)
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

        // Load cache from DAL if available (instant recovery)
        let cache = self.cache.clone();
        if let Some(ref dal) = self.dal {
            if !self.graph_name.is_empty() {
                match dal.checkpoint().load_reactor_state(&self.graph_name).await {
                    Ok(Some((cache_data, _dirty_data, _seq_queue))) => {
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

        let dirty = if self.expected_sources.is_empty() {
            Arc::new(RwLock::new(DirtyFlags::new()))
        } else {
            Arc::new(RwLock::new(DirtyFlags::with_sources(
                &self.expected_sources,
            )))
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
        let seq_queue: SeqQueue = Arc::new(RwLock::new(VecDeque::new()));

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
        let fire_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        // Consecutive persist failures — drives the I-0108 / T-0590
        // watchdog that flips the reactor to `Degraded` after 5 in a row.
        let persist_streak = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let health_exec = self.health.clone();

        loop {
            tokio::select! {
                Some(signal) = strategy_rx.recv() => {
                    match input_strategy {
                        InputStrategy::Latest => {
                            let should_run = match signal {
                                StrategySignal::BoundaryReceived => {
                                    let d = dirty_exec.read().await;
                                    match &criteria {
                                        ReactionCriteria::WhenAny => d.any_set(),
                                        ReactionCriteria::WhenAll => d.all_set(),
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
                                let result = (graph)(snapshot).await;
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
                                    GraphResult::Completed { .. } => {
                                        let fires = fire_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
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
                                        let result = (graph)(snapshot).await;
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
                                            GraphResult::Completed { .. } => {
                                                let fires = fire_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
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

### `cloacina::computation_graph::reactor::ReactorHealth` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Health state of a reactor.

#### Variants

- **`Starting`** - Loading cache from DAL, spawning accumulators.
- **`Warming`** - Some accumulators healthy, waiting for all.
- **`Live`** - All accumulators healthy, evaluating criteria.
- **`Degraded`** - Was live, an accumulator disconnected. Running with stale data.



### `cloacina::computation_graph::reactor::ReactionCriteria` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Reaction criteria — when to fire the graph.

#### Variants

- **`WhenAny`** - Fire if any dirty flag is set.
- **`WhenAll`** - Fire if all dirty flags are set.



### `cloacina::computation_graph::reactor::InputStrategy` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Input strategy — how the reactor handles data between executions.

#### Variants

- **`Latest`** - One slot per source, overwritten on each update. Always fires with freshest.
- **`Sequential`** - Boundaries preserved in order, one execution per boundary.



### `cloacina::computation_graph::reactor::StrategySignal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Signals sent from receiver to executor.

#### Variants

- **`BoundaryReceived`** - A boundary was received — check reaction criteria.
- **`ForceFire`** - Force-fire regardless of criteria.



### `cloacina::computation_graph::reactor::ManualCommand` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Manual commands accepted by the reactor.

#### Variants

- **`ForceFire`** - Fire with current cache state.
- **`FireWith`** - Fire with injected state (replaces cache).



### `cloacina::computation_graph::reactor::ReactorCommand` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Commands sent by WebSocket operators to a reactor.

#### Variants

- **`ForceFire`**
- **`FireWith`**
- **`GetState`**
- **`Pause`**
- **`Resume`**



### `cloacina::computation_graph::reactor::ReactorResponse` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Responses sent back to WebSocket operators.

#### Variants

- **`Fired`**
- **`State`**
- **`Paused`**
- **`Resumed`**
- **`Error`**



## Functions

### `cloacina::computation_graph::reactor::reactor_health_channel`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn reactor_health_channel () -> (watch :: Sender < ReactorHealth > , watch :: Receiver < ReactorHealth >)
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



### `cloacina::computation_graph::reactor::write_reactor_firing`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn write_reactor_firing (dal : & Option < crate :: dal :: unified :: DAL > , tenant_id : & Option < String > , reactor_name : & str , snapshot : & cloacina_computation_graph :: InputCache ,)
```

Write one `reactor_firings` row to the DAL on each fire.

Best-effort: a DAL failure logs a warning but never blocks the
in-process CG dispatch (the supervisor's persist-failure watchdog
catches sustained DAL trouble via `persist_reactor_state`).
Skipped silently when no DAL is wired (embedded mode) or no tenant
is set (cross-tenant / test reactors).

<details>
<summary>Source</summary>

```rust
async fn write_reactor_firing(
    dal: &Option<crate::dal::unified::DAL>,
    tenant_id: &Option<String>,
    reactor_name: &str,
    snapshot: &cloacina_computation_graph::InputCache,
) {
    let Some(dal) = dal else { return };
    let Some(tenant) = tenant_id else { return };
    if reactor_name.is_empty() {
        return;
    }

    // Serialize boundary cache as a stable map keyed by source name string —
    // matches what subscribers will deserialize as their input context.
    let entries: std::collections::HashMap<String, Vec<u8>> = snapshot
        .entries_raw()
        .iter()
        .map(|(name, bytes)| (name.as_str().to_string(), bytes.clone()))
        .collect();
    let payload = match bincode::serialize(&entries) {
        Ok(b) => Some(b),
        Err(e) => {
            tracing::warn!(
                reactor = reactor_name,
                "failed to bincode-serialize firing payload: {}",
                e
            );
            None
        }
    };

    let fired_at = crate::database::universal_types::UniversalTimestamp::now();
    if let Err(e) = dal
        .reactor_subscriptions()
        .insert_firing(reactor_name, tenant, payload, fired_at)
        .await
    {
        tracing::warn!(
            reactor = reactor_name,
            tenant = tenant.as_str(),
            "failed to write reactor_firings row: {}",
            e
        );
        return;
    }

    metrics::counter!(
        "cloacina_reactor_firings_total",
        "graph" => reactor_name.to_string(),
        "reactor" => reactor_name.to_string(),
    )
    .increment(1);
}
```

</details>



### `cloacina::computation_graph::reactor::persist_reactor_state`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn persist_reactor_state (dal : & Option < crate :: dal :: unified :: DAL > , graph_name : & str , cache : & Arc < RwLock < InputCache > > , dirty : & Arc < RwLock < DirtyFlags > > , seq_queue : Option < & SeqQueue > , persist_streak : & Arc < std :: sync :: atomic :: AtomicU32 > , health : & Option < watch :: Sender < ReactorHealth > > ,)
```

Persist reactor state to DAL (best-effort, logs on failure).

Tracks consecutive failures via `persist_streak` and downgrades the
reactor to `ReactorHealth::Degraded` after
`PERSIST_FAILURE_DEGRADE_THRESHOLD` consecutive failures. On the next
successful persist after the threshold has been crossed, the streak is
reset and the reactor reports `Live` again — operators see persist
trouble on `/v1/health/graphs/{name}` without tailing logs.

<details>
<summary>Source</summary>

```rust
async fn persist_reactor_state(
    dal: &Option<crate::dal::unified::DAL>,
    graph_name: &str,
    cache: &Arc<RwLock<InputCache>>,
    dirty: &Arc<RwLock<DirtyFlags>>,
    seq_queue: Option<&SeqQueue>,
    persist_streak: &Arc<std::sync::atomic::AtomicU32>,
    health: &Option<watch::Sender<ReactorHealth>>,
) {
    let dal = match dal {
        Some(d) if !graph_name.is_empty() => d,
        _ => return,
    };

    let cache_snapshot = cache.read().await;
    let dirty_snapshot = dirty.read().await;

    // Serialize cache entries as JSON map
    let cache_bytes = match serde_json::to_vec(&cache_snapshot.entries_raw()) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(graph = %graph_name, "cache serialization failed: {}", e);
            record_reactor_persist_failure(graph_name, "cache_serialize", persist_streak, health);
            return;
        }
    };

    let dirty_bytes = match serde_json::to_vec(&dirty_snapshot.flags) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(graph = %graph_name, "dirty flags serialization failed: {}", e);
            record_reactor_persist_failure(graph_name, "dirty_serialize", persist_streak, health);
            return;
        }
    };

    let seq_bytes = if let Some(q) = seq_queue {
        let queue = q.read().await;
        if queue.is_empty() {
            None
        } else {
            match serde_json::to_vec(&*queue) {
                Ok(b) => Some(b),
                Err(e) => {
                    tracing::warn!(graph = %graph_name, "sequential queue serialization failed: {}", e);
                    record_reactor_persist_failure(
                        graph_name,
                        "seq_serialize",
                        persist_streak,
                        health,
                    );
                    None
                }
            }
        }
    } else {
        None
    };

    if let Err(e) = dal
        .checkpoint()
        .save_reactor_state(graph_name, cache_bytes, dirty_bytes, seq_bytes)
        .await
    {
        tracing::warn!(graph = %graph_name, "reactor state persistence failed: {}", e);
        record_reactor_persist_failure(graph_name, "save", persist_streak, health);
        return;
    }

    // Success path — reset streak and recover health if we'd previously
    // degraded due to persist trouble.
    let prev = persist_streak.swap(0, std::sync::atomic::Ordering::Relaxed);
    if prev >= PERSIST_FAILURE_DEGRADE_THRESHOLD {
        if let Some(ref tx) = health {
            tracing::info!(graph = %graph_name, "persist recovered after {} failures", prev);
            let _ = tx.send(ReactorHealth::Live);
        }
    }
}
```

</details>



### `cloacina::computation_graph::reactor::record_reactor_persist_failure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn record_reactor_persist_failure (graph_name : & str , kind : & 'static str , persist_streak : & Arc < std :: sync :: atomic :: AtomicU32 > , health : & Option < watch :: Sender < ReactorHealth > > ,)
```

Increment the bounded persist-failures counter for a reactor, bump the reactor-level streak, and push the reactor to `Degraded` once the streak crosses [`PERSIST_FAILURE_DEGRADE_THRESHOLD`]. The `kind ∈ {cache_serialize, dirty_serialize, seq_serialize, save}` label surface attributes the failure to a specific branch of [`persist_reactor_state`].

<details>
<summary>Source</summary>

```rust
fn record_reactor_persist_failure(
    graph_name: &str,
    kind: &'static str,
    persist_streak: &Arc<std::sync::atomic::AtomicU32>,
    health: &Option<watch::Sender<ReactorHealth>>,
) {
    metrics::counter!(
        "cloacina_reactor_persist_failures_total",
        "graph" => graph_name.to_string(),
        "reactor" => graph_name.to_string(),
        "kind" => kind,
    )
    .increment(1);
    let streak = persist_streak.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if streak == PERSIST_FAILURE_DEGRADE_THRESHOLD {
        if let Some(ref tx) = health {
            tracing::warn!(
                graph = %graph_name,
                streak,
                "persist failure streak hit threshold — downgrading reactor to Degraded"
            );
            let _ = tx.send(ReactorHealth::Degraded {
                disconnected: vec!["persist".to_string()],
            });
        }
    }
}
```

</details>
