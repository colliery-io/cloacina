# cloacina::error <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Enums

### `cloacina::error::ContextError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::error::RegistrationError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during task registration.

Registration errors prevent tasks from being added to a registry
due to validation failures or conflicts.

#### Variants

- **`DuplicateTaskId`**
- **`InvalidTaskId`**
- **`RegistrationFailed`**



### `cloacina::error::ValidationError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::error::ExecutorError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::error::WorkflowError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::error::SubgraphError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur when creating Workflow subgraphs.

Subgraph errors occur when extracting portions of a Workflow for
partial execution or analysis.

#### Variants

- **`TaskNotFound`**
- **`UnsupportedOperation`**
