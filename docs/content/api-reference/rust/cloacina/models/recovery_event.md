# cloacina::models::recovery_event <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Recovery Event Model

This module defines domain structures and types for tracking recovery events.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::recovery_event::RecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a recovery event record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `recovered_at` | `UniversalTimestamp` |  |
| `details` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::models::recovery_event::NewRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new recovery event records (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `details` | `Option < String >` |  |



## Enums

### `cloacina::models::recovery_event::RecoveryType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Enumeration of possible recovery types in the system.

#### Variants

- **`TaskReset`**
- **`TaskAbandoned`**
- **`PipelineFailed`**
- **`WorkflowUnavailable`**
