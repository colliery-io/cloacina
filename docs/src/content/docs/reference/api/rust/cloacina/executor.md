---
title: "cloacina::executor"
description: "API reference of the Rust module cloacina::executor."
editUrl: false
sidebar:
  order: 117
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Task Executor**

The Task Executor provides task execution via the dispatcher architecture.
The scheduler pushes task-ready events to executors, which handle execution
with automatic context loading and state management.

**Key Features**

- **Push-Based Execution**: Scheduler dispatches tasks directly to executors
- **Atomic Task Claiming**: Thread-safe task claiming with database locking
- **Lazy Context Loading**: Automatic dependency context loading at execution time
- **Simple Conflict Resolution**: Latest task wins for key conflicts
- **Concurrency Management**: Configurable limits with semaphore-based control
- **Timeout Handling**: Per-task execution timeout with cancellation

**Components**

- `ThreadTaskExecutor`: Default executor implementing the `TaskExecutor` trait
- `WorkflowExecutor`: Handles workflow execution results and status tracking

**Configuration**

The executor can be configured using `ExecutorConfig`:
- Concurrency limits (`max_concurrent_tasks`)
- Execution timeouts (`task_timeout`)

**Thread Safety**

All components are thread-safe and can be used in concurrent environments.

## Re-exports

- [`TaskContextBuilder`](../../../rust/cloacina/executor/context_builder/#struct.TaskContextBuilder)
- [`TaskResultHandler`](../../../rust/cloacina/executor/result_handler/#struct.TaskResultHandler)
- [`EngineHost`](../../../rust/cloacina/executor/host_impl/#struct.EngineHost)
- [`SlotToken`](../../../rust/cloacina/executor/slot_token/#struct.SlotToken)
- [`TaskHandle`](../../../rust/cloacina/executor/task_handle/#struct.TaskHandle)
- [`current_task_execution_id`](../../../rust/cloacina/executor/task_handle/#fn.current_task_execution_id)
- [`return_task_handle`](../../../rust/cloacina/executor/task_handle/#fn.return_task_handle)
- [`take_task_handle`](../../../rust/cloacina/executor/task_handle/#fn.take_task_handle)
- [`with_task_execution_id`](../../../rust/cloacina/executor/task_handle/#fn.with_task_execution_id)
- [`with_task_handle`](../../../rust/cloacina/executor/task_handle/#fn.with_task_handle)
- [`ThreadTaskExecutor`](../../../rust/cloacina/executor/thread_task_executor/#struct.ThreadTaskExecutor)
- [`ClaimedTask`](../../../rust/cloacina/executor/types/#struct.ClaimedTask)
- [`DependencyLoader`](../../../rust/cloacina/executor/types/#struct.DependencyLoader)
- [`ExecutorConfig`](../../../rust/cloacina/executor/types/#struct.ExecutorConfig)
- [`StatusCallback`](../../../rust/cloacina/executor/workflow_executor/#trait.StatusCallback)
- [`TaskResult`](../../../rust/cloacina/executor/workflow_executor/#struct.TaskResult)
- [`WorkflowExecution`](../../../rust/cloacina/executor/workflow_executor/#struct.WorkflowExecution)
- [`WorkflowExecutionError`](../../../rust/cloacina/executor/workflow_executor/#enum.WorkflowExecutionError)
- [`WorkflowExecutionResult`](../../../rust/cloacina/executor/workflow_executor/#struct.WorkflowExecutionResult)
- [`WorkflowExecutor`](../../../rust/cloacina/executor/workflow_executor/#trait.WorkflowExecutor)
- [`WorkflowStatus`](../../../rust/cloacina/executor/workflow_executor/#enum.WorkflowStatus)
