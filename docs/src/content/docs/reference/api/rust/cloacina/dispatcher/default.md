---
title: "cloacina::dispatcher::default"
description: "Default dispatcher implementation."
editUrl: false
sidebar:
  order: 107
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Default dispatcher implementation.

Provides the standard dispatcher that sends every task to a single
server-configured executor. Choosing *which* node/compute a task runs on is
deliberately an executor-internal concern (CLOACI-T-0640) — the scheduler and
dispatcher only know the one configured executor key.

## Structs

### <span id="struct.DefaultDispatcher"></span><code style="font-size: 0.85em">struct DefaultDispatcher</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Default dispatcher implementation.

The DefaultDispatcher maintains a registry of executor backends and sends
every task to the single configured `default_executor_key`. It handles the
full dispatch lifecycle including state transitions and result handling.

**Examples:**

```rust
use cloacina::dispatcher::DefaultDispatcher;

// Every task runs on the "default" (thread) executor.
let dispatcher = DefaultDispatcher::new(dal, "default");
dispatcher.register_executor("default", Arc::new(thread_executor));

// Or send everything to the fleet:
let dispatcher = DefaultDispatcher::new(dal, "fleet");
dispatcher.register_executor("fleet", Arc::new(fleet_executor));
```

#### Methods

##### <span id="DefaultDispatcher.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: DAL, default_executor: impl Into<String>) -> Self
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

##### <span id="DefaultDispatcher.with_defaults"></span><code>with_​defaults</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_defaults(dal: DAL) -> Self
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

##### <span id="DefaultDispatcher.default_executor_key"></span><code>default_​executor_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn default_executor_key(&self) -> &str
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

##### <span id="DefaultDispatcher.dal"></span><code>dal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn dal(&self) -> &DAL
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
