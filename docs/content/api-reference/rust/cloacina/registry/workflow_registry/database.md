# cloacina::registry::workflow_registry::database <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database operations for workflow registry metadata storage.

## Structs

### `cloacina::registry::workflow_registry::database::InspectedPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Result of inspecting a package — full metadata plus the raw build state.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `WorkflowMetadata` |  |
| `build_status` | `String` |  |
| `build_error` | `Option < String >` |  |



### `cloacina::registry::workflow_registry::database::BuildQueueStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `serde :: Serialize`

Snapshot of the build queue for the compiler's status endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pending` | `u64` |  |
| `building` | `u64` |  |
| `last_success_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |
| `last_failure_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |
| `heartbeat_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |



### `cloacina::registry::workflow_registry::database::ClaimedBuild`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A build row claimed by the compiler. Everything the compiler needs to locate the source and write back results.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `registry_id` | `Uuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `metadata` | `String` |  |
