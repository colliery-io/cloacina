# cloacina::runner::default_runner::service_manager <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Background service lifecycle management for `DefaultRunner`.

Each long-running background loop owned by the runner is wrapped as a
[`BackgroundService`]. The [`ServiceManager`] owns the collection,
orchestrates `start_all()`/`shutdown_all()`, and provides typed slots so
external callers can still reach individual service Arcs (registry,
graph scheduler, unified scheduler, etc).

## Structs

### `cloacina::runner::default_runner::service_manager::ServiceManager`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


Owns and orchestrates the runner's background services.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `services` | `Vec < Box < dyn BackgroundService > >` |  |
| `shutdown_tx` | `broadcast :: Sender < () >` |  |
| `shutdown_sent` | `bool` |  |
| `cron_recovery` | `Option < Arc < CronRecoveryService > >` |  |
| `workflow_registry` | `Option < Arc < dyn WorkflowRegistry > >` |  |
| `unified_scheduler` | `Option < Arc < Scheduler > >` |  |
| `graph_scheduler` | `Arc < RwLock < Option < Arc < ComputationGraphScheduler > > > >` | Shared graph-scheduler slot — set by `DefaultRunner::set_graph_scheduler`
and observed by the registry reconciler. The slot is shared via Arc so
updates are visible to whoever holds a clone. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            services: Vec::new(),
            shutdown_tx,
            shutdown_sent: false,
            cron_recovery: None,
            workflow_registry: None,
            unified_scheduler: None,
            graph_scheduler: Arc::new(RwLock::new(None)),
        }
    }
```

</details>



##### `register` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn register (& mut self , service : Box < dyn BackgroundService >)
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn register(&mut self, service: Box<dyn BackgroundService>) {
        self.services.push(service);
    }
```

</details>



##### `start_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn start_all (& mut self) -> Result < () , WorkflowExecutionError >
```

Start every registered service in registration order.

<details>
<summary>Source</summary>

```rust
    pub(super) async fn start_all(&mut self) -> Result<(), WorkflowExecutionError> {
        for svc in &mut self.services {
            tracing::debug!(service = svc.name(), "starting background service");
            svc.start(self.shutdown_tx.subscribe()).await?;
        }
        Ok(())
    }
```

</details>



##### `shutdown_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn shutdown_all (& mut self) -> Result < () , WorkflowExecutionError >
```

Broadcast shutdown and await each service in reverse registration order.

<details>
<summary>Source</summary>

```rust
    pub(super) async fn shutdown_all(&mut self) -> Result<(), WorkflowExecutionError> {
        if !self.shutdown_sent {
            let _ = self.shutdown_tx.send(());
            self.shutdown_sent = true;
        }
        for svc in self.services.iter_mut().rev() {
            tracing::debug!(service = svc.name(), "stopping background service");
            if let Err(e) = svc.shutdown().await {
                tracing::error!(service = svc.name(), "service shutdown error: {}", e);
            }
        }
        Ok(())
    }
```

</details>





### `cloacina::runner::default_runner::service_manager::TaskSchedulerService`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


Wraps the per-runner `TaskScheduler` polling loop.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `scheduler` | `Arc < TaskScheduler >` |  |
| `span` | `tracing :: Span` |  |
| `handle` | `Option < JoinHandle < () > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (scheduler : Arc < TaskScheduler > , span : tracing :: Span) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(scheduler: Arc<TaskScheduler>, span: tracing::Span) -> Self {
        Self {
            scheduler,
            span,
            handle: None,
        }
    }
```

</details>





### `cloacina::runner::default_runner::service_manager::UnifiedSchedulerService`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


Wraps the unified cron + trigger scheduler loop.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `scheduler` | `Arc < Scheduler >` |  |
| `inner_shutdown_tx` | `watch :: Sender < bool >` |  |
| `span` | `tracing :: Span` |  |
| `handle` | `Option < JoinHandle < () > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (scheduler : Arc < Scheduler > , inner_shutdown_tx : watch :: Sender < bool > , span : tracing :: Span ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        scheduler: Arc<Scheduler>,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            scheduler,
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
```

</details>





### `cloacina::runner::default_runner::service_manager::CronRecoveryServiceWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


Wraps the cron recovery loop.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `service` | `Arc < CronRecoveryService >` |  |
| `inner_shutdown_tx` | `watch :: Sender < bool >` |  |
| `span` | `tracing :: Span` |  |
| `handle` | `Option < JoinHandle < () > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (service : Arc < CronRecoveryService > , inner_shutdown_tx : watch :: Sender < bool > , span : tracing :: Span ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        service: Arc<CronRecoveryService>,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            service,
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
```

</details>





### `cloacina::runner::default_runner::service_manager::RegistryReconcilerService`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


Wraps the registry reconciler loop. Owns the reconciler outright because `start_reconciliation_loop` consumes `self`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `reconciler` | `Option < RegistryReconciler >` |  |
| `inner_shutdown_tx` | `watch :: Sender < bool >` |  |
| `span` | `tracing :: Span` |  |
| `handle` | `Option < JoinHandle < () > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (reconciler : RegistryReconciler , inner_shutdown_tx : watch :: Sender < bool > , span : tracing :: Span ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        reconciler: RegistryReconciler,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            reconciler: Some(reconciler),
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
```

</details>





### `cloacina::runner::default_runner::service_manager::StaleClaimSweeperService`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


Wraps the stale-claim sweeper loop.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `sweeper` | `Option < StaleClaimSweeper >` |  |
| `inner_shutdown_tx` | `watch :: Sender < bool >` |  |
| `span` | `tracing :: Span` |  |
| `handle` | `Option < JoinHandle < () > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (sweeper : StaleClaimSweeper , inner_shutdown_tx : watch :: Sender < bool > , span : tracing :: Span ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        sweeper: StaleClaimSweeper,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            sweeper: Some(sweeper),
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
```

</details>
