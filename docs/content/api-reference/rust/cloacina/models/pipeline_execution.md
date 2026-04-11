# cloacina::models::pipeline_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow Execution Models

This module defines domain structures for tracking workflow executions.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::pipeline_execution::WorkflowExecutionRecord`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a workflow execution record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `pipeline_name` | `String` |  |
| `pipeline_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `error_details` | `Option < String >` |  |
| `recovery_attempts` | `i32` |  |
| `last_recovery_at` | `Option < UniversalTimestamp >` |  |
| `paused_at` | `Option < UniversalTimestamp >` |  |
| `pause_reason` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::models::pipeline_execution::NewWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new workflow executions (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pipeline_name` | `String` |  |
| `pipeline_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
