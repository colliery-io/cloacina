# cloacina::computation_graph::scheduler <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Reactive Scheduler — spawns, supervises, and shuts down accumulator/reactor tasks from computation graph declarations.

The reactive counterpart to the Unified Scheduler. Receives declarations
from the reconciler, wires channels, spawns tokio tasks, registers endpoints,
and restarts tasks on panic.

## Structs

### `cloacina::computation_graph::scheduler::ComputationGraphDeclaration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Declaration of a computation graph to be loaded by the Reactive Scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Unique name for this computation graph. |
| `accumulators` | `Vec < AccumulatorDeclaration >` | Accumulator declarations. |
| `reactor` | `ReactorDeclaration` | Reactor declaration. |
| `tenant_id` | `Option < String >` | Tenant that owns this graph (None = global/public). |



### `cloacina::computation_graph::scheduler::AccumulatorDeclaration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Declaration for a single accumulator.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Accumulator name (used as WebSocket endpoint name). |
| `factory` | `Arc < dyn AccumulatorFactory >` | Factory that creates the accumulator instance. |



### `cloacina::computation_graph::scheduler::AccumulatorSpawnConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Configuration passed to [`AccumulatorFactory::spawn`] for resilience wiring.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Option < crate :: dal :: unified :: DAL >` | DAL handle for checkpoint persistence. None in embedded/test mode. |
| `health_tx` | `Option < watch :: Sender < AccumulatorHealth > >` | Health state reporter. None when health tracking is not needed. |
| `graph_name` | `String` | Graph name (used as key for checkpoint persistence). |



### `cloacina::computation_graph::scheduler::ReactorDeclaration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Declaration for the reactor.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `criteria` | `ReactionCriteria` | Reaction criteria (when_any / when_all). |
| `strategy` | `InputStrategy` | Input strategy (latest / sequential). |
| `graph_fn` | `CompiledGraphFn` | The compiled graph function. |



### `cloacina::computation_graph::scheduler::GraphStatus`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Status of a managed computation graph.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulators` | `Vec < String >` |  |
| `reactor_paused` | `bool` |  |
| `running` | `bool` |  |
| `health` | `Option < super :: reactor :: ReactorHealth >` | Reactor health state machine value. None if health tracking is not configured. |



### `cloacina::computation_graph::scheduler::RunningGraph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


State for a running computation graph.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `shutdown_tx` | `watch :: Sender < bool >` | Shutdown signal sender. |
| `shutdown_rx` | `watch :: Receiver < bool >` | Shutdown signal receiver (cloneable, for re-spawning accumulators). |
| `boundary_tx` | `mpsc :: Sender < (SourceName , Vec < u8 >) >` | Boundary channel sender (shared by all accumulators, for re-spawning). |
| `accumulator_handles` | `Vec < (String , JoinHandle < () >) >` | Accumulator task handles. |
| `reactor_handle` | `JoinHandle < () >` | Reactor task handle. |
| `reactor_shared` | `ReactorHandle` | Reactor handle for pause/resume queries. |
| `reactor_health_rx` | `Option < watch :: Receiver < super :: reactor :: ReactorHealth > >` | Reactor health receiver for status reporting. |
| `declaration` | `ComputationGraphDeclaration` | Declaration (for restarts). |
| `failure_counts` | `HashMap < String , u32 >` | Per-component consecutive failure count. |
| `last_success` | `HashMap < String , std :: time :: Instant >` | Timestamp of last successful operation per component (for failure count reset). |



### `cloacina::computation_graph::scheduler::ReactiveScheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


The Reactive Scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `registry` | `EndpointRegistry` | Endpoint registry for WebSocket routing. |
| `graphs` | `Arc < RwLock < HashMap < String , RunningGraph > > >` | Running computation graphs. |
| `dal` | `Option < crate :: dal :: unified :: DAL >` | DAL handle for persistence. None in embedded/test mode. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (registry : EndpointRegistry) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(registry: EndpointRegistry) -> Self {
        Self {
            registry,
            graphs: Arc::new(RwLock::new(HashMap::new())),
            dal: None,
        }
    }
```

</details>



##### `with_dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_dal (registry : EndpointRegistry , dal : crate :: dal :: unified :: DAL) -> Self
```

Create a scheduler with DAL support for persistence and health tracking.

<details>
<summary>Source</summary>

```rust
    pub fn with_dal(registry: EndpointRegistry, dal: crate::dal::unified::DAL) -> Self {
        Self {
            registry,
            graphs: Arc::new(RwLock::new(HashMap::new())),
            dal: Some(dal),
        }
    }
```

</details>



##### `load_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn load_graph (& self , decl : ComputationGraphDeclaration) -> Result < () , String >
```

Load and start a computation graph.

<details>
<summary>Source</summary>

```rust
    pub async fn load_graph(&self, decl: ComputationGraphDeclaration) -> Result<(), String> {
        let name = decl.name.clone();

        // Check if already loaded
        {
            let graphs = self.graphs.read().await;
            if graphs.contains_key(&name) {
                return Err(format!("graph '{}' already loaded", name));
            }
        }

        let (shutdown_tx, shutdown_rx) = shutdown_signal();
        let stored_shutdown_rx = shutdown_rx.clone();

        // Create boundary channel (all accumulators → reactor)
        let (boundary_tx, boundary_rx) = mpsc::channel(256);
        let stored_boundary_tx = boundary_tx.clone();

        // Collect expected source names for WhenAll seeding
        let expected_sources: Vec<SourceName> = decl
            .accumulators
            .iter()
            .map(|a| SourceName::new(&a.name))
            .collect();

        // Spawn accumulators with health and DAL wiring
        let mut accumulator_handles = Vec::new();
        let mut acc_health_rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )> = Vec::new();
        for acc_decl in &decl.accumulators {
            // Create health channel for this accumulator
            let (health_tx, health_rx) = health_channel();
            acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));

            let spawn_config = AccumulatorSpawnConfig {
                dal: self.dal.clone(),
                health_tx: Some(health_tx),
                graph_name: name.clone(),
            };

            let (socket_tx, handle) = acc_decl.factory.spawn(
                acc_decl.name.clone(),
                boundary_tx.clone(),
                shutdown_rx.clone(),
                spawn_config,
            );

            // Register socket and health in endpoint registry
            self.registry
                .register_accumulator(acc_decl.name.clone(), socket_tx)
                .await;
            self.registry
                .register_accumulator_health(acc_decl.name.clone(), health_rx)
                .await;

            accumulator_handles.push((acc_decl.name.clone(), handle));
        }

        // Create manual command channel
        let (manual_tx, manual_rx) = mpsc::channel(64);

        // Create reactor health channel
        let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();

        // Create and spawn reactor with full wiring
        let mut reactor = Reactor::new(
            decl.reactor.graph_fn.clone(),
            decl.reactor.criteria.clone(),
            decl.reactor.strategy.clone(),
            boundary_rx,
            manual_rx,
            shutdown_rx,
        )
        .with_graph_name(name.clone())
        .with_health(reactor_health_tx)
        .with_expected_sources(expected_sources)
        .with_accumulator_health(acc_health_rxs);

        if let Some(ref dal) = self.dal {
            reactor = reactor.with_dal(dal.clone());
        }

        let reactor_shared = reactor.handle();

        // Register reactor in endpoint registry
        self.registry
            .register_reactor(name.clone(), manual_tx, reactor_shared.clone())
            .await;

        // Set auth policies based on package tenant ownership.
        // Global packages (tenant_id=None): allow any authenticated key.
        // Tenant-scoped packages: restrict to that tenant's keys + admin.
        let acc_policy = match &decl.tenant_id {
            Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
            None => AccumulatorAuthPolicy::allow_all(),
        };
        let reactor_policy = match &decl.tenant_id {
            Some(tid) => ReactorAuthPolicy::for_tenant(tid),
            None => ReactorAuthPolicy::allow_all(),
        };
        for acc_decl in &decl.accumulators {
            self.registry
                .set_accumulator_policy(acc_decl.name.clone(), acc_policy.clone())
                .await;
        }
        self.registry
            .set_reactor_policy(name.clone(), reactor_policy)
            .await;

        let reactor_handle = tokio::spawn(reactor.run());

        info!(graph = %name, "computation graph loaded and running");

        let running = RunningGraph {
            shutdown_tx,
            shutdown_rx: stored_shutdown_rx,
            boundary_tx: stored_boundary_tx,
            accumulator_handles,
            reactor_handle,
            reactor_shared,
            reactor_health_rx: Some(reactor_health_rx),
            declaration: decl,
            failure_counts: HashMap::new(),
            last_success: HashMap::new(),
        };

        self.graphs.write().await.insert(name, running);
        Ok(())
    }
```

</details>



##### `unload_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn unload_graph (& self , name : & str) -> Result < () , String >
```

Unload and shut down a computation graph.

<details>
<summary>Source</summary>

```rust
    pub async fn unload_graph(&self, name: &str) -> Result<(), String> {
        let running = {
            let mut graphs = self.graphs.write().await;
            graphs
                .remove(name)
                .ok_or_else(|| format!("graph '{}' not loaded", name))?
        };

        // Send shutdown signal
        let _ = running.shutdown_tx.send(true);

        // Wait for reactor
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), running.reactor_handle).await;

        // Wait for accumulators
        for (acc_name, handle) in running.accumulator_handles {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
            self.registry.deregister_accumulator(&acc_name).await;
        }

        // Deregister reactor
        self.registry.deregister_reactor(name).await;

        info!(graph = %name, "computation graph unloaded");
        Ok(())
    }
```

</details>



##### `list_graphs` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_graphs (& self) -> Vec < GraphStatus >
```

List all loaded computation graphs with status.

<details>
<summary>Source</summary>

```rust
    pub async fn list_graphs(&self) -> Vec<GraphStatus> {
        let graphs = self.graphs.read().await;
        graphs
            .iter()
            .map(|(name, running)| GraphStatus {
                name: name.clone(),
                accumulators: running
                    .accumulator_handles
                    .iter()
                    .map(|(n, _)| n.clone())
                    .collect(),
                reactor_paused: running.reactor_shared.is_paused(),
                running: !running.reactor_handle.is_finished(),
                health: running
                    .reactor_health_rx
                    .as_ref()
                    .map(|rx| rx.borrow().clone()),
            })
            .collect()
    }
```

</details>



##### `check_and_restart_failed` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_and_restart_failed (& self) -> usize
```

Check all graphs for crashed tasks and restart them.

Individual accumulators are restarted in-place without tearing down the
reactor. Reactor crashes trigger a full-graph restart. Failure counting
with exponential backoff prevents infinite restart loops.

<details>
<summary>Source</summary>

```rust
    pub async fn check_and_restart_failed(&self) -> usize {
        let mut restarted = 0;
        let mut graphs = self.graphs.write().await;
        let now = std::time::Instant::now();

        for (graph_name, running) in graphs.iter_mut() {
            // Reset failure counts for components that have been running successfully
            let success_threshold = std::time::Duration::from_secs(SUCCESS_RESET_SECS);
            let names_to_reset: Vec<String> = running
                .last_success
                .iter()
                .filter(|(_, ts)| now.duration_since(**ts) >= success_threshold)
                .map(|(name, _)| name.clone())
                .collect();
            for name in names_to_reset {
                running.failure_counts.remove(&name);
                running.last_success.remove(&name);
            }

            // Check reactor
            if running.reactor_handle.is_finished() {
                let reactor_key = format!("{}::reactor", graph_name);
                let failures = running
                    .failure_counts
                    .entry(reactor_key.clone())
                    .or_insert(0);
                *failures += 1;

                if *failures > MAX_RECOVERY_ATTEMPTS {
                    error!(
                        graph = %graph_name,
                        failures = *failures,
                        "reactor permanently failed — circuit breaker open"
                    );
                    continue;
                }

                let backoff_secs =
                    (BACKOFF_BASE_SECS * 2u64.pow(*failures - 1)).min(BACKOFF_MAX_SECS);
                let backoff = std::time::Duration::from_secs(backoff_secs);
                warn!(
                    graph = %graph_name,
                    attempt = *failures,
                    backoff_secs = backoff_secs,
                    "reactor crashed, restarting (full graph restart)"
                );

                // Record recovery event and wait for backoff
                self.record_recovery_event(&reactor_key, *failures, backoff_secs)
                    .await;
                tokio::time::sleep(backoff).await;

                // Full graph restart: new channels, re-spawn everything
                let (shutdown_tx, shutdown_rx) = shutdown_signal();
                let stored_shutdown_rx = shutdown_rx.clone();
                let (boundary_tx, boundary_rx) = mpsc::channel(256);
                let stored_boundary_tx = boundary_tx.clone();

                let expected_sources: Vec<SourceName> = running
                    .declaration
                    .accumulators
                    .iter()
                    .map(|a| SourceName::new(&a.name))
                    .collect();

                let mut new_acc_handles = Vec::new();
                let mut restart_acc_health_rxs: Vec<(
                    String,
                    watch::Receiver<super::accumulator::AccumulatorHealth>,
                )> = Vec::new();
                for acc_decl in &running.declaration.accumulators {
                    let (health_tx, health_rx) = health_channel();
                    restart_acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));
                    let spawn_config = AccumulatorSpawnConfig {
                        dal: self.dal.clone(),
                        health_tx: Some(health_tx),
                        graph_name: graph_name.clone(),
                    };
                    let (socket_tx, handle) = acc_decl.factory.spawn(
                        acc_decl.name.clone(),
                        boundary_tx.clone(),
                        shutdown_rx.clone(),
                        spawn_config,
                    );
                    self.registry
                        .register_accumulator(acc_decl.name.clone(), socket_tx)
                        .await;
                    self.registry
                        .register_accumulator_health(acc_decl.name.clone(), health_rx)
                        .await;
                    new_acc_handles.push((acc_decl.name.clone(), handle));
                }

                let (manual_tx, manual_rx) = mpsc::channel(64);
                let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();
                let mut reactor = Reactor::new(
                    running.declaration.reactor.graph_fn.clone(),
                    running.declaration.reactor.criteria.clone(),
                    running.declaration.reactor.strategy.clone(),
                    boundary_rx,
                    manual_rx,
                    shutdown_rx,
                )
                .with_graph_name(graph_name.clone())
                .with_health(reactor_health_tx)
                .with_expected_sources(expected_sources)
                .with_accumulator_health(restart_acc_health_rxs);
                if let Some(ref dal) = self.dal {
                    reactor = reactor.with_dal(dal.clone());
                }
                let reactor_shared = reactor.handle();
                let reactor_handle = tokio::spawn(reactor.run());

                self.registry
                    .register_reactor(graph_name.clone(), manual_tx, reactor_shared.clone())
                    .await;

                // Re-set auth policies after restart
                let restart_acc_policy = match &running.declaration.tenant_id {
                    Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
                    None => AccumulatorAuthPolicy::allow_all(),
                };
                let restart_reactor_policy = match &running.declaration.tenant_id {
                    Some(tid) => ReactorAuthPolicy::for_tenant(tid),
                    None => ReactorAuthPolicy::allow_all(),
                };
                for acc_decl in &running.declaration.accumulators {
                    self.registry
                        .set_accumulator_policy(acc_decl.name.clone(), restart_acc_policy.clone())
                        .await;
                }
                self.registry
                    .set_reactor_policy(graph_name.clone(), restart_reactor_policy)
                    .await;

                running.shutdown_tx = shutdown_tx;
                running.shutdown_rx = stored_shutdown_rx;
                running.boundary_tx = stored_boundary_tx;
                running.accumulator_handles = new_acc_handles;
                running.reactor_handle = reactor_handle;
                running.reactor_shared = reactor_shared;
                running.reactor_health_rx = Some(reactor_health_rx);
                running.last_success.insert(reactor_key, now);

                restarted += 1;
                info!(graph = %graph_name, "reactor restarted successfully");
            } else {
                // Check individual accumulators — restart them in-place
                let mut new_handles = Vec::new();
                let mut changed = false;

                for (acc_name, handle) in running.accumulator_handles.drain(..) {
                    if handle.is_finished() {
                        let acc_key = format!("{}::{}", graph_name, acc_name);
                        let failures = running.failure_counts.entry(acc_key.clone()).or_insert(0);
                        *failures += 1;

                        if *failures > MAX_RECOVERY_ATTEMPTS {
                            error!(
                                graph = %graph_name,
                                accumulator = %acc_name,
                                failures = *failures,
                                "accumulator permanently failed — circuit breaker open"
                            );
                            // Don't add handle back — accumulator is abandoned
                            continue;
                        }

                        let backoff_secs =
                            (BACKOFF_BASE_SECS * 2u64.pow(*failures - 1)).min(BACKOFF_MAX_SECS);
                        warn!(
                            graph = %graph_name,
                            accumulator = %acc_name,
                            attempt = *failures,
                            backoff_secs = backoff_secs,
                            "accumulator crashed, restarting individually"
                        );

                        // Record recovery event and wait for backoff
                        self.record_recovery_event(&acc_key, *failures, backoff_secs)
                            .await;
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                        // Find the declaration for this accumulator
                        if let Some(acc_decl) = running
                            .declaration
                            .accumulators
                            .iter()
                            .find(|d| d.name == *acc_name)
                        {
                            // Re-spawn with existing boundary_tx and shutdown_rx
                            let (health_tx, health_rx) = health_channel();
                            let spawn_config = AccumulatorSpawnConfig {
                                dal: self.dal.clone(),
                                health_tx: Some(health_tx),
                                graph_name: graph_name.clone(),
                            };
                            let (socket_tx, new_handle) = acc_decl.factory.spawn(
                                acc_name.clone(),
                                running.boundary_tx.clone(),
                                running.shutdown_rx.clone(),
                                spawn_config,
                            );

                            // Re-register socket, health, and auth policy in endpoint registry
                            self.registry
                                .register_accumulator(acc_name.clone(), socket_tx)
                                .await;
                            self.registry
                                .register_accumulator_health(acc_name.clone(), health_rx)
                                .await;
                            let ind_acc_policy = match &running.declaration.tenant_id {
                                Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
                                None => AccumulatorAuthPolicy::allow_all(),
                            };
                            self.registry
                                .set_accumulator_policy(acc_name.clone(), ind_acc_policy)
                                .await;

                            running.last_success.insert(acc_key, now);
                            let restarted_name = acc_name.clone();
                            new_handles.push((acc_name, new_handle));
                            restarted += 1;
                            changed = true;

                            info!(
                                graph = %graph_name,
                                accumulator = %restarted_name,
                                "accumulator restarted individually"
                            );
                        } else {
                            let lost_name = acc_name.clone();
                            error!(
                                graph = %graph_name,
                                accumulator = %lost_name,
                                "cannot restart: declaration not found"
                            );
                        }
                    } else {
                        new_handles.push((acc_name, handle));
                    }
                }

                running.accumulator_handles = new_handles;

                if changed {
                    // Mark accumulators that are still running as successful
                    for (acc_name, _) in &running.accumulator_handles {
                        let acc_key = format!("{}::{}", graph_name, acc_name);
                        running.last_success.entry(acc_key).or_insert(now);
                    }
                }
            }
        }

        restarted
    }
```

</details>



##### `start_supervision` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn start_supervision (self : & Arc < Self > , mut shutdown_rx : watch :: Receiver < bool > , check_interval : std :: time :: Duration ,) -> JoinHandle < () >
```

Start a background supervision loop that checks for crashed tasks.

Returns a `JoinHandle` for the supervision task.

<details>
<summary>Source</summary>

```rust
    pub fn start_supervision(
        self: &Arc<Self>,
        mut shutdown_rx: watch::Receiver<bool>,
        check_interval: std::time::Duration,
    ) -> JoinHandle<()> {
        let scheduler = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            interval.tick().await; // skip first immediate tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let restarted = scheduler.check_and_restart_failed().await;
                        if restarted > 0 {
                            info!("supervision check: restarted {} tasks", restarted);
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::debug!("supervision loop shutting down");
                        break;
                    }
                }
            }
        })
    }
```

</details>



##### `record_recovery_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn record_recovery_event (& self , component : & str , attempt : u32 , backoff_secs : u64)
```

Record a recovery event in the DAL (best-effort, logs on failure).

<details>
<summary>Source</summary>

```rust
    async fn record_recovery_event(&self, component: &str, attempt: u32, backoff_secs: u64) {
        let dal = match &self.dal {
            Some(d) => d,
            None => return,
        };
        use crate::database::universal_types::UniversalUuid;
        use crate::models::recovery_event::NewRecoveryEvent;
        let event = NewRecoveryEvent {
            pipeline_execution_id: UniversalUuid::new_v4(),
            task_execution_id: None,
            recovery_type: "graph_component_restart".to_string(),
            details: Some(format!(
                "component={}, attempt={}, backoff={}s",
                component, attempt, backoff_secs
            )),
        };
        if let Err(e) = dal.recovery_event().create(event).await {
            warn!(component = %component, "failed to record recovery event: {}", e);
        }
    }
```

</details>



##### `shutdown_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn shutdown_all (& self)
```

Graceful shutdown of all graphs.

<details>
<summary>Source</summary>

```rust
    pub async fn shutdown_all(&self) {
        let names: Vec<String> = {
            let graphs = self.graphs.read().await;
            graphs.keys().cloned().collect()
        };

        for name in names {
            if let Err(e) = self.unload_graph(&name).await {
                warn!(graph = %name, error = %e, "failed to unload graph during shutdown");
            }
        }
    }
```

</details>
