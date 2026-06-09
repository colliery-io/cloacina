# cloacina::dispatcher::default <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Default dispatcher implementation.

Provides the standard dispatcher that sends every task to a single
server-configured executor. Choosing *which* node/compute a task runs on is
deliberately an executor-internal concern (CLOACI-T-0640) — the scheduler and
dispatcher only know the one configured executor key.

## Structs

### `cloacina::dispatcher::default::DefaultDispatcher`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Default dispatcher implementation.

The DefaultDispatcher maintains a registry of executor backends and sends
every task to the single configured `default_executor_key`. It handles the
full dispatch lifecycle including state transitions and result handling.

**Examples:**

```rust,ignore
use cloacina::dispatcher::DefaultDispatcher;

// Every task runs on the "default" (thread) executor.
let dispatcher = DefaultDispatcher::new(dal, "default");
dispatcher.register_executor("default", Arc::new(thread_executor));

// Or send everything to the fleet:
let dispatcher = DefaultDispatcher::new(dal, "fleet");
dispatcher.register_executor("fleet", Arc::new(fleet_executor));
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `executors` | `RwLock < HashMap < String , Arc < dyn TaskExecutor > > >` | Registered executor backends |
| `default_executor_key` | `String` | The single executor key every task is dispatched to. |
| `dal` | `DAL` | Data access layer for state updates |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL , default_executor : impl Into < String >) -> Self
```

Creates a new DefaultDispatcher that dispatches every task to `default_executor` (e.g. `"default"` for the thread executor, `"fleet"` for the execution-agent fleet).

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL, default_executor: impl Into<String>) -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
            default_executor_key: default_executor.into(),
            dal,
        }
    }
```

</details>



##### `with_defaults` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_defaults (dal : DAL) -> Self
```

Creates a dispatcher that sends every task to the `"default"` (thread) executor.

<details>
<summary>Source</summary>

```rust
    pub fn with_defaults(dal: DAL) -> Self {
        Self::new(dal, "default")
    }
```

</details>



##### `default_executor_key` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn default_executor_key (& self) -> & str
```

The executor key every task is dispatched to.

<details>
<summary>Source</summary>

```rust
    pub fn default_executor_key(&self) -> &str {
        &self.default_executor_key
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

Logs the execution result. State transitions are owned by the executor (via `complete_task_transaction` / `mark_task_failed`) — the dispatcher only routes and logs.

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
                info!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    duration_ms = result.duration.as_millis(),
                    "Task completed successfully"
                );
            }
            ExecutionStatus::Failed => {
                let error_msg = result.error.as_deref().unwrap_or("Unknown error");
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
