---
title: "cloacina-workflow::trigger"
description: "Trigger types for workflow authoring."
editUrl: false
sidebar:
  order: 38
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Trigger types for workflow authoring.

T-0552 (I-0102 follow-up) relocated the `Trigger` trait from `cloacina`
(engine-only) into this leaf crate so packaged cdylibs can collect
`TriggerEntry` inventory entries (which hold `Arc<dyn Trigger>`) at
link time, and the unified `cloacina::package!()` shell macro can walk
them at FFI call time. Engine paths re-export `cloacina_workflow::Trigger`.

## Enums

### <span id="enum.TriggerResult"></span><code style="font-size: 0.85em">enum TriggerResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Result of a trigger poll operation.

#### Variants

- **`Skip`** - Do not fire the workflow, continue polling on the next interval.
- **`Fire`** - Fire the workflow with an optional context.

#### Methods

##### <span id="TriggerResult.should_fire"></span><code>should_​fire</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn should_fire(&self) -> bool
```

Returns true if this result indicates the workflow should fire.

<details>
<summary>Source</summary>

```rust
    pub fn should_fire(&self) -> bool {
        matches!(self, TriggerResult::Fire(_))
    }
```

</details>

##### <span id="TriggerResult.into_context"></span><code>into_​context</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_context(self) -> Option<Context<serde_json::Value>>
```

Extracts the context if this is a Fire result.

<details>
<summary>Source</summary>

```rust
    pub fn into_context(self) -> Option<Context<serde_json::Value>> {
        match self {
            TriggerResult::Fire(ctx) => ctx,
            TriggerResult::Skip => None,
        }
    }
```

</details>

##### <span id="TriggerResult.context_hash"></span><code>context_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn context_hash(&self) -> String
```

Computes a hash of the context for deduplication purposes.

If no context is provided, returns a constant hash. This allows
deduplication based on the specific trigger conditions.

<details>
<summary>Source</summary>

```rust
    pub fn context_hash(&self) -> String {
        match self {
            TriggerResult::Skip => "skip".to_string(),
            TriggerResult::Fire(None) => "fire_no_context".to_string(),
            TriggerResult::Fire(Some(ctx)) => {
                let mut hasher = DefaultHasher::new();
                if let Ok(serialized) = serde_json::to_string(ctx.data()) {
                    serialized.hash(&mut hasher);
                }
                format!("{:016x}", hasher.finish())
            }
        }
    }
```

</details>

### <span id="enum.TriggerError"></span><code style="font-size: 0.85em">enum TriggerError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during trigger polling.

#### Variants

- **`PollError`** - Error during trigger polling
- **`ContextError`** - Context creation error

## Traits

### <span id="trait.Trigger"></span><code style="font-size: 0.85em">trait Trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync + fmt::Debug`

Core trait for user-defined triggers.

Triggers are polling functions that determine when a workflow should
execute. Each trigger has a name, poll interval, and a `poll()` method
that returns whether the workflow should fire.

#### Methods

##### <span id="Trigger.name"></span><code>name</code>

```rust
fn name(&self) -> &str
```

Returns the unique name of this trigger.

<details>
<summary>Source</summary>

```rust
    fn name(&self) -> &str;
```

</details>

##### <span id="Trigger.poll_interval"></span><code>poll_​interval</code>

```rust
fn poll_interval(&self) -> Duration
```

Returns how often this trigger should be polled.

<details>
<summary>Source</summary>

```rust
    fn poll_interval(&self) -> Duration;
```

</details>

##### <span id="Trigger.allow_concurrent"></span><code>allow_​concurrent</code>

```rust
fn allow_concurrent(&self) -> bool
```

Returns whether concurrent executions with the same context are allowed. When `false`, if a workflow execution with the same context hash is already running, the trigger will not fire again until it completes.

<details>
<summary>Source</summary>

```rust
    fn allow_concurrent(&self) -> bool;
```

</details>

##### <span id="Trigger.poll"></span><code>poll</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn poll(&self) -> Result<TriggerResult, TriggerError>
```

Polls the trigger condition and returns whether to fire the workflow.

Called at the configured `poll_interval`. Returns `TriggerResult::Skip`
to continue polling, `TriggerResult::Fire(ctx)` to fire the workflow.
Errors are logged and polling continues on the next interval.

<details>
<summary>Source</summary>

```rust
    async fn poll(&self) -> Result<TriggerResult, TriggerError>;
```

</details>

##### <span id="Trigger.cron_expression"></span><code>cron_​expression</code>

```rust
fn cron_expression(&self) -> Option<String>
```

Returns this trigger's cron expression, if any. Cron-shaped triggers override this to return `Some(expr)`; their `poll_interval` is ignored and the reconciler routes them to the cron scheduler instead of the runtime trigger registry. Default `None` covers all custom-poll triggers.

<details>
<summary>Source</summary>

```rust
    fn cron_expression(&self) -> Option<String> {
        None
    }
```

</details>

##### <span id="Trigger.workflow_name"></span><code>workflow_​name</code>

```rust
fn workflow_name(&self) -> &str
```

The target workflow this trigger fires — the `#[trigger(on = "...")]` binding. For cron triggers the reconciler registers the schedule against this workflow (not the trigger's own `name`). Default empty for manual impls that don't bind a workflow; macro-generated triggers always override it (CLOACI-T-0669).

<details>
<summary>Source</summary>

```rust
    fn workflow_name(&self) -> &str {
        ""
    }
```

</details>
