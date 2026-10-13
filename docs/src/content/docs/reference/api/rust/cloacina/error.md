---
title: "cloacina::error"
description: "API reference of the Rust module cloacina::error."
editUrl: false
sidebar:
  order: 110
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Error Types**

This module defines comprehensive error types for all components of the Cloacina framework.
The error types follow a hierarchical structure that provides detailed context for debugging
and error handling.

**Error Categories**

- [`ContextError`](../../../rust/cloacina/error/#enum.ContextError): Errors related to context operations (insert, update, serialization)
- [`TaskError`](../../../rust/cloacina-workflow/error/#enum.TaskError): Errors that occur during task execution
- [`WorkflowError`](../../../rust/cloacina/error/#enum.WorkflowError): Errors in workflow construction and validation
- [`ValidationError`](../../../rust/cloacina/error/#enum.ValidationError): Graph validation and dependency resolution errors
- [`CheckpointError`](../../../rust/cloacina-workflow/error/#enum.CheckpointError): Errors in task checkpointing
- [`RegistrationError`](../../../rust/cloacina/error/#enum.RegistrationError): Task registration and ID validation errors
- [`SubgraphError`](../../../rust/cloacina/error/#enum.SubgraphError): Errors when creating Workflow subgraphs
- [`ExecutorError`](../../../rust/cloacina/error/#enum.ExecutorError): Errors during task execution and workflow management

**Error Handling Patterns**

```rust
use cloacina::*;

match context.insert("key", value) {
    Ok(()) => println!("Success"),
    Err(ContextError::KeyExists(key)) => {
        println!("Key '{}' already exists", key);
    }
    Err(ContextError::Serialization(e)) => {
        eprintln!("Failed to serialize: {}", e);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

**Error Type Details**

**ContextError**

Handles errors related to the execution context, including:
- Serialization failures
- Key management issues (not found, exists, type mismatch)
- Database and connection pool errors
- Invalid execution scope errors

**TaskError**

Covers task execution failures, including:
- Execution failures with detailed context
- Dependency satisfaction issues
- Timeout conditions
- Context-related errors
- Validation and readiness check failures
- Trigger rule evaluation issues

**ValidationError**

Encompasses workflow and graph validation issues:
- Circular dependencies
- Missing dependencies
- Duplicate task IDs
- Empty workflow conditions
- Invalid graph structures
- Database and connection issues
- Recovery-specific errors

**ExecutorError**

Manages errors during task execution and workflow management:
- Database and connection pool issues
- Task registry problems
- Execution timeouts
- Semaphore acquisition failures
- Workflow execution issues
- Serialization errors
- Scope and validation errors

**CheckpointError**

Handles task state persistence issues:
- Save and load failures
- Serialization errors
- Storage and validation problems

**RegistrationError**

Manages task registration issues:
- Duplicate task IDs
- Invalid task IDs
- Registration failures

**WorkflowError**

Covers workflow construction problems:
- Duplicate tasks
- Missing tasks
- Invalid dependencies
- Cyclic dependencies
- Unreachable tasks

**SubgraphError**

Handles subgraph creation issues:
- Missing tasks
- Unsupported operations

All error types implement the standard `Error` trait and provide detailed
error messages for debugging and logging purposes. Each error variant includes
relevant context information to aid in troubleshooting and recovery.

## Re-exports

- [`CheckpointError`](../../../rust/cloacina-workflow/error/#enum.CheckpointError)
- [`TaskError`](../../../rust/cloacina-workflow/error/#enum.TaskError)

## Enums

### <span id="enum.ContextError"></span><code style="font-size: 0.85em">enum ContextError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during context operations.

Context errors cover data manipulation, serialization, and key management
within the execution context.

#### Variants

- **`Serialization`**
- **`KeyNotFound`**
- **`TypeMismatch`**
- **`KeyExists`**
- **`Database`**
- **`ConnectionPool`**
- **`InvalidScope`**

### <span id="enum.RegistrationError"></span><code style="font-size: 0.85em">enum RegistrationError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during task registration.

Registration errors prevent tasks from being added to a registry
due to validation failures or conflicts.

#### Variants

- **`DuplicateTaskId`**
- **`InvalidTaskId`**
- **`RegistrationFailed`**

### <span id="enum.ValidationError"></span><code style="font-size: 0.85em">enum ValidationError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during Workflow and dependency validation.

Validation errors indicate structural problems with the task graph
that prevent safe execution.

#### Variants

- **`CyclicDependency`**
- **`MissingDependency`**
- **`DuplicateTaskId`**
- **`EmptyWorkflow`**
- **`InvalidGraph`**
- **`WorkflowNotFound`**
- **`ExecutionFailed`**
- **`TaskSchedulingFailed`**
- **`InvalidTriggerRule`**
- **`InvalidPredicate`** - CLOACI-T-0602 — caller passed a CEL expression to
  `subscribe_workflow_to_reactor` that fails to compile.
- **`InvalidTaskName`**
- **`ContextEvaluationFailed`**
- **`RecoveryFailed`**
- **`TaskRecoveryAbandoned`**
- **`WorkflowRecoveryFailed`**
- **`DatabaseConnection`**
- **`DatabaseQuery`**
- **`Database`**
- **`ConnectionPool`**
- **`InvalidStateTransition`** - CLOACI-T-0625 — a delivery-outbox row was asked to make a state
  transition its current state does not permit (e.g. acking a pending
  row, or transitioning a terminal `acked` row).
- **`Context`**

### <span id="enum.ExecutorError"></span><code style="font-size: 0.85em">enum ExecutorError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during task execution.

#### Variants

- **`Database`**
- **`ConnectionPool`**
- **`TaskNotFound`**
- **`TaskExecution`**
- **`Context`**
- **`TaskTimeout`**
- **`ClaimLost`**
- **`Semaphore`**
- **`WorkflowExecutionNotFound`**
- **`Serialization`**
- **`InvalidScope`**
- **`Validation`**
- **`ContextLoadFailed`**

### <span id="enum.WorkflowError"></span><code style="font-size: 0.85em">enum WorkflowError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during workflow construction and management.

Workflow errors occur when building or modifying workflows.

#### Variants

- **`DuplicateTask`**
- **`TaskNotFound`**
- **`InvalidDependency`**
- **`CyclicDependency`**
- **`UnreachableTask`**
- **`RegistryError`**
- **`TaskError`**
- **`ValidationError`**

### <span id="enum.SubgraphError"></span><code style="font-size: 0.85em">enum SubgraphError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur when creating Workflow subgraphs.

Subgraph errors occur when extracting portions of a Workflow for
partial execution or analysis.

#### Variants

- **`TaskNotFound`**
- **`UnsupportedOperation`**
