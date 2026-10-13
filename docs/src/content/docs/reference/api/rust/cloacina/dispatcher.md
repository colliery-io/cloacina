---
title: "cloacina::dispatcher"
description: "API reference of the Rust module cloacina::dispatcher."
editUrl: false
sidebar:
  order: 106
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Dispatcher Layer for Executor Decoupling**

The dispatcher module provides a clean abstraction between the scheduler and executor,
enabling pluggable executor backends without tight coupling through database polling.

**Architecture**

```text
Scheduler (mark_ready) --> Dispatcher --> Executor
```

The dispatcher receives `TaskReadyEvent`s from the scheduler and sends every
task to a single server-configured executor (CLOACI-T-0640). Selecting which
node/compute a task lands on is an executor-internal concern, not something
the scheduler/dispatcher decides.

**Key Components**

- [`TaskReadyEvent`](../../../rust/cloacina/dispatcher/types/#struct.TaskReadyEvent): Event emitted when a task becomes ready for execution
- [`Dispatcher`](../../../rust/cloacina/dispatcher/traits/#trait.Dispatcher): Trait for dispatching events to the configured executor
- [`TaskExecutor`](../../../rust/cloacina/dispatcher/traits/#trait.TaskExecutor): Trait for executor backends that receive and execute tasks
- [`DefaultDispatcher`](../../../rust/cloacina/dispatcher/default/#struct.DefaultDispatcher): Standard single-executor implementation

**Usage**

```rust
use cloacina::dispatcher::{DefaultDispatcher, TaskReadyEvent};
use cloacina::dispatcher::TaskExecutor;

// Dispatch every task to the "default" (thread) executor.
let mut dispatcher = DefaultDispatcher::new(dal, "default");

// Register executor backends
dispatcher.register_executor("default", Arc::new(thread_executor));

// Dispatch ready tasks
dispatcher.dispatch(event).await?;
```

## Re-exports

- [`DefaultDispatcher`](../../../rust/cloacina/dispatcher/default/#struct.DefaultDispatcher)
- [`Dispatcher`](../../../rust/cloacina/dispatcher/traits/#trait.Dispatcher)
- [`TaskExecutor`](../../../rust/cloacina/dispatcher/traits/#trait.TaskExecutor)
- [`DispatchError`](../../../rust/cloacina/dispatcher/types/#enum.DispatchError)
- [`ExecutionResult`](../../../rust/cloacina/dispatcher/types/#struct.ExecutionResult)
- [`ExecutionStatus`](../../../rust/cloacina/dispatcher/types/#enum.ExecutionStatus)
- [`ExecutorMetrics`](../../../rust/cloacina/dispatcher/types/#struct.ExecutorMetrics)
- [`TaskReadyEvent`](../../../rust/cloacina/dispatcher/types/#struct.TaskReadyEvent)
