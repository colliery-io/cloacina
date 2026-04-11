# cloacina::registry::loader::task_registrar::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Types for task metadata exchange with dynamic libraries.

The raw FFI structs have been removed — fidius-host handles all serialization
and FFI safety transparently. Only the owned (post-extraction) types remain.

## Structs

### `cloacina::registry::loader::task_registrar::types::OwnedTaskMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Owned task metadata — safe to use after library is unloaded.

All fields are owned `String` values; no raw pointers are involved.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `local_id` | `String` | Local task ID (e.g., "collect_data") |
| `dependencies_json` | `String` | JSON string of task dependencies |



### `cloacina::registry::loader::task_registrar::types::OwnedTaskMetadataCollection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Owned collection of task metadata — safe to use after library is unloaded.

All fields are owned `String` values; no raw pointers are involved.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` | Name of the workflow (e.g., "data_processing") |
| `package_name` | `String` | Name of the package (e.g., "simple_demo") |
| `tasks` | `Vec < OwnedTaskMetadata >` | Owned task metadata for each task in the package |
