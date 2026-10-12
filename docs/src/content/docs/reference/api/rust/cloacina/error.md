---
title: "cloacina::error"
description: "API reference of the Rust module cloacina::error."
sidebar:
  order: 127
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>




## Enums

### <span id="enum.ContextError"></span>`enum ContextError`

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



### <span id="enum.RegistrationError"></span>`enum RegistrationError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during task registration.

Registration errors prevent tasks from being added to a registry
due to validation failures or conflicts.

#### Variants

- **`DuplicateTaskId`**
- **`InvalidTaskId`**
- **`RegistrationFailed`**



### <span id="enum.ValidationError"></span>`enum ValidationError`

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



### <span id="enum.ExecutorError"></span>`enum ExecutorError`

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



### <span id="enum.WorkflowError"></span>`enum WorkflowError`

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



### <span id="enum.SubgraphError"></span>`enum SubgraphError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur when creating Workflow subgraphs.

Subgraph errors occur when extracting portions of a Workflow for
partial execution or analysis.

#### Variants

- **`TaskNotFound`**
- **`UnsupportedOperation`**



