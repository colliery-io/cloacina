# cloacina::dispatcher::default <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Default dispatcher implementation.

Provides the standard dispatcher that routes tasks to executors based on
configurable glob patterns.

## Structs

### `cloacina::dispatcher::default::DefaultDispatcher`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Default dispatcher implementation with glob-based routing.

The DefaultDispatcher maintains a registry of executor backends and routes
tasks based on pattern matching rules. It handles the full dispatch lifecycle
including state transitions and result handling.

**Examples:**

```rust,ignore
use cloacina::dispatcher::{DefaultDispatcher, RoutingConfig, RoutingRule};

let config = RoutingConfig::new("default")
    .with_rule(RoutingRule::new("ml::*", "gpu"))
    .with_rule(RoutingRule::new("heavy_*", "high_memory"));

let dispatcher = DefaultDispatcher::new(dal, config);
dispatcher.register_executor("default", Arc::new(thread_executor));
dispatcher.register_executor("gpu", Arc::new(gpu_executor));
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `executors` | `RwLock < HashMap < String , Arc < dyn TaskExecutor > > >` | Registered executor backends |
| `router` | `Router` | Routing logic |
| `dal` | `DAL` | Data access layer for state updates |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL , routing : RoutingConfig) -> Self
```

Creates a new DefaultDispatcher with the given DAL and routing configuration.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL, routing: RoutingConfig) -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
            router: Router::new(routing),
            dal,
        }
    }
```

</details>



##### `with_defaults` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_defaults (dal : DAL) -> Self
```

Creates a dispatcher with default routing (all tasks go to "default" executor).

<details>
<summary>Source</summary>

```rust
    pub fn with_defaults(dal: DAL) -> Self {
        Self::new(dal, RoutingConfig::default())
    }
```

</details>



##### `router` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn router (& self) -> & Router
```

Gets a reference to the router for inspection.

<details>
<summary>Source</summary>

```rust
    pub fn router(&self) -> &Router {
        &self.router
    }
```

</details>



##### `dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn dal (& self) -> & DAL
```

Gets a reference to the DAL.

<details>
<summary>Source</summary>

```rust
    pub fn dal(&self) -> &DAL {
        &self.dal
    }
```

</details>



##### `handle_result` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn handle_result (& self , event : & TaskReadyEvent , result : super :: types :: ExecutionResult ,) -> Result < () , DispatchError >
```

Handles the execution result by updating database state.

<details>
<summary>Source</summary>

```rust
    async fn handle_result(
        &self,
        event: &TaskReadyEvent,
        result: super::types::ExecutionResult,
    ) -> Result<(), DispatchError> {
        match result.status {
            ExecutionStatus::Completed => {
                self.dal
                    .task_execution()
                    .mark_completed(event.task_execution_id)
                    .await?;
                info!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    duration_ms = result.duration.as_millis(),
                    "Task completed successfully"
                );
            }
            ExecutionStatus::Failed => {
                let error_msg = result.error.as_deref().unwrap_or("Unknown error");
                self.dal
                    .task_execution()
                    .mark_failed(event.task_execution_id, error_msg)
                    .await?;
                warn!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    error = error_msg,
                    duration_ms = result.duration.as_millis(),
                    "Task failed"
                );
            }
            ExecutionStatus::Retry => {
                // Retry handling is done by the executor - it schedules the retry
                debug!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    "Task will be retried"
                );
            }
            ExecutionStatus::Skipped => {
                // Task was claimed by another runner — no action needed
                debug!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    "Task skipped (claimed by another runner)"
                );
            }
        }

        Ok(())
    }
```

</details>
