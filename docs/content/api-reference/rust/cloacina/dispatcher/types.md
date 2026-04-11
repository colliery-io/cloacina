# cloacina::dispatcher::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Core types for the dispatcher system.

This module defines the fundamental data structures used for dispatching
tasks from the scheduler to executors.

## Structs

### `cloacina::dispatcher::types::TaskReadyEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Event emitted when a task becomes ready for execution.

This event contains all the information needed to identify and route a task.
The actual context loading is deferred to execution time.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` | Unique identifier for this task execution |
| `pipeline_execution_id` | `UniversalUuid` | Parent pipeline execution ID |
| `task_name` | `String` | Fully qualified task name (namespace::task) |
| `attempt` | `i32` | Current attempt number (starts at 1) |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (task_execution_id : UniversalUuid , pipeline_execution_id : UniversalUuid , task_name : String , attempt : i32 ,) -> Self
```

Creates a new TaskReadyEvent.

<details>
<summary>Source</summary>

```rust
    pub fn new(
        task_execution_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
        task_name: String,
        attempt: i32,
    ) -> Self {
        Self {
            task_execution_id,
            pipeline_execution_id,
            task_name,
            attempt,
        }
    }
```

</details>





### `cloacina::dispatcher::types::ExecutionResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

Result of task execution from an executor.

This structure contains the outcome of a task execution, including
the final status, any error message, and execution metrics.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` | The task execution ID |
| `status` | `ExecutionStatus` | Final execution status |
| `error` | `Option < String >` | Error message (if failed) |
| `duration` | `Duration` | Time taken to execute the task |

#### Methods

##### `success` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn success (task_execution_id : UniversalUuid , duration : Duration) -> Self
```

Creates a successful execution result.

<details>
<summary>Source</summary>

```rust
    pub fn success(task_execution_id: UniversalUuid, duration: Duration) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Completed,
            error: None,
            duration,
        }
    }
```

</details>



##### `failure` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn failure (task_execution_id : UniversalUuid , error : impl Into < String > , duration : Duration ,) -> Self
```

Creates a failed execution result.

<details>
<summary>Source</summary>

```rust
    pub fn failure(
        task_execution_id: UniversalUuid,
        error: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Failed,
            error: Some(error.into()),
            duration,
        }
    }
```

</details>



##### `skipped` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn skipped (task_execution_id : UniversalUuid) -> Self
```

Creates a skipped execution result (task claimed by another runner).

<details>
<summary>Source</summary>

```rust
    pub fn skipped(task_execution_id: UniversalUuid) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Skipped,
            error: None,
            duration: Duration::ZERO,
        }
    }
```

</details>



##### `retry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn retry (task_execution_id : UniversalUuid , error : impl Into < String > , duration : Duration ,) -> Self
```

Creates a retry execution result.

<details>
<summary>Source</summary>

```rust
    pub fn retry(
        task_execution_id: UniversalUuid,
        error: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Retry,
            error: Some(error.into()),
            duration,
        }
    }
```

</details>





### `cloacina::dispatcher::types::ExecutorMetrics`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Metrics for monitoring executor performance.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `active_tasks` | `usize` | Number of tasks currently executing |
| `max_concurrent` | `usize` | Maximum concurrent tasks allowed |
| `total_executed` | `u64` | Total tasks executed since startup |
| `total_failed` | `u64` | Total tasks that failed |
| `avg_duration_ms` | `u64` | Average task duration in milliseconds |

#### Methods

##### `available_capacity` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn available_capacity (& self) -> usize
```

Returns the current capacity (available slots).

<details>
<summary>Source</summary>

```rust
    pub fn available_capacity(&self) -> usize {
        self.max_concurrent.saturating_sub(self.active_tasks)
    }
```

</details>





### `cloacina::dispatcher::types::RoutingConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration for task routing.

Defines how tasks are routed to different executor backends based on
pattern matching rules.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `default_executor` | `String` | Default executor key when no rules match |
| `rules` | `Vec < RoutingRule >` | Routing rules evaluated in order |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (default_executor : impl Into < String >) -> Self
```

Creates a new routing configuration with a default executor.

<details>
<summary>Source</summary>

```rust
    pub fn new(default_executor: impl Into<String>) -> Self {
        Self {
            default_executor: default_executor.into(),
            rules: Vec::new(),
        }
    }
```

</details>



##### `with_rule` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_rule (mut self , rule : RoutingRule) -> Self
```

Adds a routing rule.

<details>
<summary>Source</summary>

```rust
    pub fn with_rule(mut self, rule: RoutingRule) -> Self {
        self.rules.push(rule);
        self
    }
```

</details>



##### `with_rules` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_rules (mut self , rules : impl IntoIterator < Item = RoutingRule >) -> Self
```

Adds multiple routing rules.

<details>
<summary>Source</summary>

```rust
    pub fn with_rules(mut self, rules: impl IntoIterator<Item = RoutingRule>) -> Self {
        self.rules.extend(rules);
        self
    }
```

</details>





### `cloacina::dispatcher::types::RoutingRule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A routing rule for directing tasks to specific executors.

Rules are evaluated in order, and the first matching rule determines
which executor handles the task.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_pattern` | `String` | Glob pattern to match task names (e.g., "ml::*", "heavy::*") |
| `executor` | `String` | Executor key to route matching tasks to |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (task_pattern : impl Into < String > , executor : impl Into < String >) -> Self
```

Creates a new routing rule.

<details>
<summary>Source</summary>

```rust
    pub fn new(task_pattern: impl Into<String>, executor: impl Into<String>) -> Self {
        Self {
            task_pattern: task_pattern.into(),
            executor: executor.into(),
        }
    }
```

</details>





## Enums

### `cloacina::dispatcher::types::ExecutionStatus` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Simplified status for execution results.

#### Variants

- **`Completed`** - Task completed successfully
- **`Failed`** - Task failed
- **`Retry`** - Task should be retried
- **`Skipped`** - Task was skipped (e.g., claimed by another runner)



### `cloacina::dispatcher::types::DispatchError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during dispatch operations.

#### Variants

- **`ExecutorNotFound`** - The specified executor was not found
- **`ExecutionFailed`** - Task execution failed
- **`DatabaseError`** - Database operation failed
- **`ContextError`** - Context operation failed
- **`ValidationError`** - Validation error
- **`NoCapacity`** - The executor has no capacity
- **`TaskNotFound`** - Task not found for dispatch
