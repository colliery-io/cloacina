# cloacina::trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


**Examples:**

```rust,ignore
use cloacina::*;

#[trigger(
    name = "file_watcher",
    poll_interval = "5s",
    allow_concurrent = false,
)]
async fn file_watcher() -> TriggerResult {
    if let Some(path) = check_for_new_file("/inbox/").await {
        let mut ctx = Context::new();
        ctx.insert("file_path", serde_json::json!(path))?;
        TriggerResult::Fire(Some(ctx))
    } else {
        TriggerResult::Skip
    }
}
```

## Structs

### `cloacina::trigger::TriggerConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Configuration for a trigger.

This is typically set via macro attributes and stored in the database
for persistence across restarts.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Unique name identifying this trigger |
| `workflow_name` | `String` | Name of the workflow to fire when the trigger activates |
| `poll_interval` | `Duration` | How often to poll the trigger function |
| `allow_concurrent` | `bool` | Whether to allow concurrent executions with the same context |
| `enabled` | `bool` | Whether this trigger is enabled |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (name : & str , workflow_name : & str , poll_interval : Duration) -> Self
```

Creates a new trigger configuration.

<details>
<summary>Source</summary>

```rust
    pub fn new(name: &str, workflow_name: &str, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            workflow_name: workflow_name.to_string(),
            poll_interval,
            allow_concurrent: false,
            enabled: true,
        }
    }
```

</details>



##### `with_allow_concurrent` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_allow_concurrent (mut self , allow : bool) -> Self
```

Sets whether concurrent executions are allowed.

<details>
<summary>Source</summary>

```rust
    pub fn with_allow_concurrent(mut self, allow: bool) -> Self {
        self.allow_concurrent = allow;
        self
    }
```

</details>



##### `with_enabled` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_enabled (mut self , enabled : bool) -> Self
```

Sets whether the trigger is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
```

</details>





## Enums

### `cloacina::trigger::TriggerError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during trigger operations.

#### Variants

- **`PollError`** - Error during trigger polling
- **`ContextError`** - Error creating context for workflow
- **`TriggerNotFound`** - Trigger not found in registry
- **`Database`** - Database error during trigger operations
- **`ConnectionPool`** - Connection pool error
- **`WorkflowSchedulingFailed`** - Workflow scheduling failed



### `cloacina::trigger::TriggerResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of a trigger poll operation.

When a trigger's `poll()` function is called, it returns this enum
to indicate whether the associated workflow should be fired.

#### Variants

- **`Skip`** - Do not fire the workflow, continue polling on the next interval.
- **`Fire`** - Fire the workflow with an optional context.

If context is provided, it will be used to initialize the workflow execution.
The context is also used for deduplication when `allow_concurrent = false`.
