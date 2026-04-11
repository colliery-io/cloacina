# cloacina-workflow::error <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Enums

### `cloacina-workflow::error::ContextError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during context operations.

This minimal version only includes errors that can occur without database
or runtime dependencies.

#### Variants

- **`Serialization`** - JSON serialization/deserialization error
- **`KeyNotFound`** - Key not found in context
- **`TypeMismatch`** - Type mismatch when retrieving a value
- **`KeyExists`** - Key already exists when inserting
- **`Database`** - Database operation failed (infrastructure error, not a key issue)
- **`ConnectionPool`** - Connection pool exhausted or unavailable



### `cloacina-workflow::error::TaskError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during task execution.

Task errors encompass execution failures, context issues, and
any other problems that prevent a task from completing successfully.

#### Variants

- **`ExecutionFailed`** - Task execution failed with a message
- **`DependencyNotSatisfied`** - Task dependency not satisfied
- **`Timeout`** - Task exceeded timeout
- **`ContextError`** - Context operation error within a task
- **`ValidationFailed`** - Task validation failed
- **`Unknown`** - Unknown error
- **`ReadinessCheckFailed`** - Task readiness check failed
- **`TriggerRuleFailed`** - Trigger rule evaluation failed



### `cloacina-workflow::error::CheckpointError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during task checkpointing.

Checkpoint errors occur when tasks attempt to save intermediate state
for recovery purposes.

#### Variants

- **`SaveFailed`** - Failed to save checkpoint
- **`LoadFailed`** - Failed to load checkpoint
- **`Serialization`** - Checkpoint serialization error
- **`StorageError`** - Checkpoint storage error
- **`ValidationFailed`** - Checkpoint validation failed
