# cloacina::registry::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Data types for the workflow registry system.

This module defines the core data structures used throughout the registry,
including workflow metadata, package information, and identifiers.

## Structs

### `cloacina::registry::types::WorkflowMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

Metadata for a registered workflow package.

This structure contains all the descriptive information about a workflow
package, stored in the `workflow_packages` table. It includes both
user-provided metadata and system-generated information.

**Examples:**

```rust
use cloacina::registry::WorkflowMetadata;
use uuid::Uuid;
use chrono::Utc;

let metadata = WorkflowMetadata {
    id: Uuid::new_v4(),
    registry_id: Uuid::new_v4(),
    package_name: "analytics_pipeline".to_string(),
    workflow_name: "analytics_workflow".to_string(),
    version: "1.0.0".to_string(),
    description: Some("Customer analytics workflow".to_string()),
    author: Some("Data Team".to_string()),
    tasks: vec!["extract_data".to_string(), "transform_data".to_string()],
    schedules: vec!["daily_analytics".to_string()],
    created_at: Utc::now(),
    updated_at: Utc::now(),
    paused: false,
};
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `WorkflowPackageId` | Unique identifier for this workflow package |
| `registry_id` | `Uuid` | Foreign key to the workflow_registry table |
| `package_name` | `String` | Name of the workflow package (e.g., "analytics_pipeline") |
| `workflow_name` | `String` | Name of the executable workflow inside the package (e.g.,
"analytics_workflow"). This is the identifier the runner registry is
keyed by and is what callers must use to execute the workflow. It often
differs from `package_name` (package `demo-slow-rust` →
workflow `demo_slow_workflow`). (CLOACI-T-0671) |
| `version` | `String` | Semantic version of the package (e.g., "1.0.0") |
| `description` | `Option < String >` | Optional human-readable description |
| `author` | `Option < String >` | Optional author information |
| `tasks` | `Vec < String >` | List of task IDs included in this package |
| `task_graph` | `Vec < WorkflowTaskNode >` | The task dependency graph: one node per task with its upstream
dependencies, so consumers (the UI) can render the full DAG. Derived
from the persisted package metadata's task list. Empty for packages
predating task-graph persistence. (CLOACI-T-0663) |
| `schedules` | `Vec < String >` | List of schedule names defined in this package |
| `created_at` | `DateTime < Utc >` | When this package was registered |
| `updated_at` | `DateTime < Utc >` | When this package metadata was last updated |
| `paused` | `bool` | Whether this workflow is paused (CLOACI-T-0749). When true, new
executions are refused at the execute chokepoint. Defaults to false for
older serialized data and non-DB registry backends. |
| `declared_params` | `Vec < cloacina_api_types :: InputSlot >` | CLOACI-I-0128: declared input params (named, JSON-Schema-typed slots) the
workflow accepts at execute time. Empty when the workflow declares none
or predates the input-interface entrypoint. |
| `declared_surfaces` | `Vec < cloacina_api_types :: DeclaredSurface >` | CLOACI-I-0128 (T-0758): declared input interfaces of the package's
non-workflow injectable surfaces (graphs/reactors/accumulators), used to
validate operator injections. Empty when none are declared. |
| `workflow_triggers` | `Vec < String >` | CLOACI-T-0777: trigger names this workflow subscribes to, from
`#[workflow(triggers = […])]`. A manual trigger fire fans out to every
workflow whose list includes the fired trigger — this is the runtime
source of truth for that fan-out (schedules carry only the trigger's
primary `on` workflow). Empty when the workflow subscribes to none. |



### `cloacina::registry::types::WorkflowSourceFile`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

A single source file extracted from a package's retained `.cloacina` archive for read-only display (CLOACI-T-0750).

`path` is relative to the package source root (e.g. `"src/lib.rs"`,
`"package.toml"`) and always uses forward slashes. Only UTF-8 text files
are represented; binary and oversized files are omitted by the extractor.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path relative to the package source root, using forward slashes. |
| `contents` | `String` | UTF-8 file contents. |



### `cloacina::registry::types::WorkflowTaskNode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

One node in a workflow's task dependency graph (CLOACI-T-0663).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Local task id (the node id), e.g. `"validate"`. |
| `dependencies` | `Vec < String >` | Local ids of the tasks this task depends on (its incoming edges). |
| `description` | `Option < String >` | Optional human-readable task description. |
| `doc_what` | `Option < String >` | CLOACI-T-0752 "what" — short summary parsed from the task's
doc-comment/docstring. `None` when undocumented. |
| `doc_why` | `Option < String >` | CLOACI-T-0752 "why" — rationale parsed from the doc-comment/docstring.
`None` when undocumented. |



### `cloacina::registry::types::PackageMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Package metadata extracted from a .cloacina file.

This structure represents the metadata embedded in the packaged workflow
file itself, typically extracted during the packaging process by cloacina-ctl.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package` | `String` | Package name from the workflow macro |
| `version` | `String` | Version from Cargo.toml |
| `description` | `Option < String >` | Optional description |
| `author` | `Option < String >` | Optional author from Cargo.toml |
| `build_info` | `BuildInfo` | Build metadata |
| `tasks` | `Vec < TaskInfo >` | Task information |
| `schedules` | `Vec < ScheduleInfo >` | Schedule information |



### `cloacina::registry::types::BuildInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Build information embedded in the package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `rustc_version` | `String` | Rust compiler version used |
| `cloacina_version` | `String` | Cloacina version used |
| `build_timestamp` | `DateTime < Utc >` | Build timestamp |
| `target` | `String` | Target architecture |



### `cloacina::registry::types::TaskInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Basic task information from package metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Task identifier |
| `dependencies` | `Vec < String >` | Task dependencies |
| `description` | `Option < String >` | Optional task description |



### `cloacina::registry::types::ScheduleInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Schedule information from package metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Schedule name |
| `cron` | `String` | Cron expression |
| `workflow` | `String` | Workflow to execute |



### `cloacina::registry::types::WorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

A workflow package ready for registration.

This structure combines the extracted metadata with the raw binary
data of the compiled workflow .so file.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `PackageMetadata` | Metadata extracted from the package |
| `package_data` | `Vec < u8 >` | Raw binary data of the .so file |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (metadata : PackageMetadata , package_data : Vec < u8 >) -> Self
```

Create a new workflow package from metadata and binary data.

<details>
<summary>Source</summary>

```rust
    pub fn new(metadata: PackageMetadata, package_data: Vec<u8>) -> Self {
        Self {
            metadata,
            package_data,
        }
    }
```

</details>



##### `from_file` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_file (_path : impl AsRef < std :: path :: Path >) -> Result < Self , std :: io :: Error >
```

Load a workflow package from a .cloacina file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `-` | Path to the .cloacina package file |


**Returns:**

* `Ok(WorkflowPackage)` - Successfully loaded package * `Err(std::io::Error)` - If file operations fail

**Examples:**

```rust,no_run
use cloacina::registry::WorkflowPackage;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let package = WorkflowPackage::from_file("analytics.cloacina")?;
println!("Loaded package: {} v{}",
    package.metadata.package,
    package.metadata.version
);
# Ok(())
# }
```

<details>
<summary>Source</summary>

```rust
    pub fn from_file(_path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        // This will be implemented to use cloacina-ctl's extraction logic
        todo!("Implement using cloacina-ctl archive extraction")
    }
```

</details>





### `cloacina::registry::types::LoadedWorkflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

A loaded workflow with both metadata and binary data.

This structure is returned when retrieving a workflow from the registry,
containing all the information needed to execute the workflow.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `WorkflowMetadata` | Full metadata from the database |
| `package_data` | `Vec < u8 >` | Source archive (.cloacina bzip2 tar) from registry storage |
| `compiled_data` | `Option < Vec < u8 > >` | Compiled cdylib bytes produced by the compiler service.
`None` for pure-Python packages; `Some` for Rust/mixed packages. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (metadata : WorkflowMetadata , package_data : Vec < u8 > , compiled_data : Option < Vec < u8 > > ,) -> Self
```

Create a new loaded workflow.

<details>
<summary>Source</summary>

```rust
    pub fn new(
        metadata: WorkflowMetadata,
        package_data: Vec<u8>,
        compiled_data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            metadata,
            package_data,
            compiled_data,
        }
    }
```

</details>
