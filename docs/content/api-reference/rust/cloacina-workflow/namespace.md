# cloacina-workflow::namespace <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task namespace management for isolated task execution.

This module provides hierarchical namespace support for tasks, enabling:
- Multi-tenant task isolation
- Workflow package separation
- Conflict resolution between workflows with same task IDs

**Examples:**

```rust
use cloacina_workflow::TaskNamespace;

// Embedded workflow (most common)
let ns = TaskNamespace::new("public", "embedded", "customer_etl", "extract_data");
assert_eq!(ns.to_string(), "public::embedded::customer_etl::extract_data");

// Workflow package
let ns = TaskNamespace::new("public", "analytics.so", "data_pipeline", "extract_data");
assert_eq!(ns.to_string(), "public::analytics.so::data_pipeline::extract_data");

// Multi-tenant scenario
let ns = TaskNamespace::new("tenant_123", "embedded", "customer_etl", "extract_data");
assert_eq!(ns.to_string(), "tenant_123::embedded::customer_etl::extract_data");
```

## Structs

### `cloacina-workflow::namespace::TaskNamespace`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`

Hierarchical namespace for task identification and isolation.

Provides a structured way to identify tasks across different contexts:
multi-tenant environments, workflow packages, and embedded workflows.
The namespace components form a hierarchy from most general (tenant) to
most specific (task), enabling precise task resolution while supporting
fallback strategies for compatibility.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` | Tenant identifier for multi-tenancy support.
Default: "public" for single-tenant or public access |
| `package_name` | `String` | Package or deployment context identifier.
Default: "embedded" for tasks compiled into the binary
For workflow packages: name from .dylib/.so file metadata |
| `workflow_id` | `String` | Workflow identifier from workflow macro.
Groups related tasks together within a package/tenant |
| `task_id` | `String` | Individual task identifier from task macro.
Unique within the workflow context |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (tenant_id : & str , package_name : & str , workflow_id : & str , task_id : & str) -> Self
```

Create a complete namespace from all components.

This is the most general constructor, useful when all namespace
components are known and need to be specified explicitly.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `-` | Tenant identifier |
| `package_name` | `-` | Package identifier |
| `workflow_id` | `-` | Workflow identifier |
| `task_id` | `-` | Task identifier |


<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
        }
    }
```

</details>



##### `from_string` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_string (namespace_str : & str) -> Result < Self , String >
```

Create a TaskNamespace from a string representation.

Parses a namespace string in the format "tenant::package::workflow::task"
into a TaskNamespace struct.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace_str` | `-` | String in format "tenant::package::workflow::task" |


**Returns:**

* `Result<TaskNamespace, String>` - Successfully parsed namespace or error message

**Examples:**

```rust
use cloacina_workflow::TaskNamespace;

let ns = TaskNamespace::from_string("public::embedded::etl::extract").unwrap();
assert_eq!(ns.tenant_id, "public");
assert_eq!(ns.task_id, "extract");

// Invalid format
assert!(TaskNamespace::from_string("invalid_format").is_err());
```

<details>
<summary>Source</summary>

```rust
    pub fn from_string(namespace_str: &str) -> Result<Self, String> {
        parse_namespace(namespace_str)
    }
```

</details>



##### `is_public` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_public (& self) -> bool
```

Check if this is a public (non-tenant-specific) namespace.

**Returns:**

`true` if this namespace uses the default "public" tenant

<details>
<summary>Source</summary>

```rust
    pub fn is_public(&self) -> bool {
        self.tenant_id == "public"
    }
```

</details>



##### `is_embedded` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_embedded (& self) -> bool
```

Check if this is an embedded (non-packaged) namespace.

**Returns:**

`true` if this namespace uses the default "embedded" package

<details>
<summary>Source</summary>

```rust
    pub fn is_embedded(&self) -> bool {
        self.package_name == "embedded"
    }
```

</details>





## Functions

### `cloacina-workflow::namespace::parse_namespace`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn parse_namespace (namespace_str : & str) -> Result < TaskNamespace , String >
```

Parse a namespace string back into a TaskNamespace.

Supports parsing namespace strings in the standard format back into
structured TaskNamespace objects.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace_str` | `-` | String in format "tenant::package::workflow::task" |


**Returns:**

* `Ok(TaskNamespace)` - Successfully parsed namespace * `Err(String)` - Parse error message

**Examples:**

```rust
use cloacina_workflow::parse_namespace;

let ns = parse_namespace("public::embedded::etl::extract").unwrap();
assert_eq!(ns.tenant_id, "public");
assert_eq!(ns.task_id, "extract");

// Invalid format
assert!(parse_namespace("invalid_format").is_err());
```

<details>
<summary>Source</summary>

```rust
pub fn parse_namespace(namespace_str: &str) -> Result<TaskNamespace, String> {
    let parts: Vec<&str> = namespace_str.split("::").collect();

    if parts.len() != 4 {
        return Err(format!(
            "Invalid namespace format '{}'. Expected 'tenant::package::workflow::task'",
            namespace_str
        ));
    }

    Ok(TaskNamespace::new(parts[0], parts[1], parts[2], parts[3]))
}
```

</details>
