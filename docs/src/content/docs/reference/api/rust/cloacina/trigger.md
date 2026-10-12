---
title: "cloacina::trigger"
description: "API reference of the Rust module cloacina::trigger."
sidebar:
  order: 218
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


**Examples:**

```rust
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

### <span id="struct.TriggerConfig"></span>`struct TriggerConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

##### <span id="TriggerConfig.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="TriggerConfig.with_allow_concurrent"></span>`with_allow_concurrent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="TriggerConfig.with_enabled"></span>`with_enabled`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

### <span id="enum.TriggerError"></span>`enum TriggerError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during trigger operations.

#### Variants

- **`PollError`** - Error during trigger polling
- **`ContextError`** - Error creating context for workflow
- **`TriggerNotFound`** - Trigger not found in registry
- **`Database`** - Database error during trigger operations
- **`ConnectionPool`** - Connection pool error
- **`WorkflowSchedulingFailed`** - Workflow scheduling failed



