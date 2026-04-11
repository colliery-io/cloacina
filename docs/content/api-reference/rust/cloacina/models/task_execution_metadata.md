# cloacina::models::task_execution_metadata <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Execution Metadata Module

This module defines domain structures for task execution metadata.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::task_execution_metadata::TaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a task execution metadata record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::models::task_execution_metadata::NewTaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new task execution metadata (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
