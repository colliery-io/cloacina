---
title: "cloacina::workflow"
description: "API reference of the Rust module cloacina::workflow."
editUrl: false
sidebar:
  order: 202
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Workflow Management**

This module provides the core functionality for creating and managing task workflows in Cloacina.
It implements a directed acyclic graph (DAG) of tasks with dependency management, validation,
and content-based versioning.

**Core Components**

- `Workflow`: Main structure for managing task graphs and execution
- `WorkflowMetadata`: Versioning and metadata management
- `DependencyGraph`: Low-level dependency tracking and cycle detection
- `WorkflowBuilder`: Fluent interface for workflow construction

**Key Features**

- Directed acyclic graph (DAG) task dependencies
- Automatic cycle detection and validation
- Content-based versioning for reliable workflow management
- Parallel execution planning
- Global workflow registry

**Type Definitions**

```rust
pub struct Workflow {
    name: String,
    tasks: HashMap<String, Arc<dyn Task>>,
    dependency_graph: DependencyGraph,
    metadata: WorkflowMetadata,
}

pub struct WorkflowMetadata {
    pub created_at: DateTime<Utc>,
    pub version: String,
    pub description: Option<String>,
    pub tags: HashMap<String, String>,
}

pub struct DependencyGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<String>>,
}
```

**Error Types**

- `WorkflowError`: Errors during workflow construction and management
- `ValidationError`: Errors during workflow validation
- `SubgraphError`: Errors during subgraph operations

The process-global workflow registry was removed in CLOACI-T-0509. Workflow
constructors are owned by [`crate::Runtime`](../../../rust/cloacina/runtime/#struct.Runtime) and seeded from `inventory`.

## Re-exports

- [`WorkflowBuilder`](../../../rust/cloacina/workflow/builder/#struct.WorkflowBuilder)
- [`DependencyGraph`](../../../rust/cloacina/workflow/graph/#struct.DependencyGraph)
- [`WorkflowMetadata`](../../../rust/cloacina/workflow/metadata/#struct.WorkflowMetadata)
- [`WorkflowConstructor`](../../../rust/cloacina/workflow/registry/#type.WorkflowConstructor)

## Structs

### <span id="struct.Workflow"></span><code style="font-size: 0.85em">struct Workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Main Workflow structure for representing and managing task graphs.

A Workflow contains a collection of tasks with their dependency relationships,
ensuring that the graph remains acyclic and provides methods for execution
planning and analysis.

**Fields**

* `name`: String - Unique identifier for the workflow
* `tenant`: String - Unique identifier for the tenant
* `tasks`: HashMap<TaskNamespace, Arc<dyn Task>> - Map of task namespaces to task implementations
* `dependency_graph`: DependencyGraph - Internal representation of task dependencies
* `metadata`: WorkflowMetadata - Versioning and metadata information

**Implementation Details**

The Workflow structure provides:
- Task dependency management
- Cycle detection and validation
- Content-based versioning
- Parallel execution planning
- Subgraph operations

**Examples:**

```rust
use cloacina::*;

let workflow = Workflow::builder("test-workflow")
    .description("Test workflow")
    .add_task(TestTask::new("task1", vec![]))?
    .add_task(TestTask::new("task2", vec!["task1"]))?
    .build()?;

assert_eq!(workflow.name(), "test-workflow");
assert!(!workflow.metadata().version.is_empty());
```

#### Methods

##### <span id="Workflow.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(name: &str) -> Self
```

Create a new Workflow with the given name

Most users should use the `workflow!` macro or builder pattern instead.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `&str` | Unique name for the workflow |

**Examples:**

```rust
use cloacina::Workflow;

let workflow = Workflow::new("my_workflow");
assert_eq!(workflow.name(), "my_workflow");
```

<details>
<summary>Source</summary>

```rust
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tenant: "public".to_string(),
            package: "embedded".to_string(),
            tasks: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            metadata: WorkflowMetadata::default(),
        }
    }
```

</details>

##### <span id="Workflow.builder"></span><code>builder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn builder(name: &str) -> WorkflowBuilder
```

Create a Workflow builder for programmatic construction

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `&str` | Unique name for the workflow |

**Examples:**

```rust
use cloacina::*;

let builder = Workflow::builder("my_workflow")
    .description("Example workflow");
```

<details>
<summary>Source</summary>

```rust
    pub fn builder(name: &str) -> WorkflowBuilder {
        WorkflowBuilder::new(name)
    }
```

</details>

##### <span id="Workflow.name"></span><code>name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn name(&self) -> &str
```

Get the Workflow name

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        &self.name
    }
```

</details>

##### <span id="Workflow.tenant"></span><code>tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant(&self) -> &str
```

Get the Workflow tenant

<details>
<summary>Source</summary>

```rust
    pub fn tenant(&self) -> &str {
        &self.tenant
    }
```

</details>

##### <span id="Workflow.set_tenant"></span><code>set_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_tenant(&mut self, tenant: &str)
```

Set the Workflow tenant

<details>
<summary>Source</summary>

```rust
    pub fn set_tenant(&mut self, tenant: &str) {
        self.tenant = tenant.to_string();
    }
```

</details>

##### <span id="Workflow.package"></span><code>package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn package(&self) -> &str
```

Get the Workflow package

<details>
<summary>Source</summary>

```rust
    pub fn package(&self) -> &str {
        &self.package
    }
```

</details>

##### <span id="Workflow.set_package"></span><code>set_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_package(&mut self, package: &str)
```

Set the Workflow package

<details>
<summary>Source</summary>

```rust
    pub fn set_package(&mut self, package: &str) {
        self.package = package.to_string();
    }
```

</details>

##### <span id="Workflow.metadata"></span><code>metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn metadata(&self) -> &WorkflowMetadata
```

Get the Workflow metadata

**Examples:**

```rust
let metadata = workflow.metadata();
println!("Version: {}", metadata.version);
println!("Created: {}", metadata.created_at);
```

<details>
<summary>Source</summary>

```rust
    pub fn metadata(&self) -> &WorkflowMetadata {
        &self.metadata
    }
```

</details>

##### <span id="Workflow.set_version"></span><code>set_​version</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_version(&mut self, version: &str)
```

Set the Workflow version manually

Note: Workflows built with the `workflow!` macro or builder automatically
calculate content-based versions.

<details>
<summary>Source</summary>

```rust
    pub fn set_version(&mut self, version: &str) {
        self.metadata.version = version.to_string();
    }
```

</details>

##### <span id="Workflow.set_description"></span><code>set_​description</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_description(&mut self, description: &str)
```

Set the Workflow description

<details>
<summary>Source</summary>

```rust
    pub fn set_description(&mut self, description: &str) {
        self.metadata.description = Some(description.to_string());
    }
```

</details>

##### <span id="Workflow.add_tag"></span><code>add_​tag</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn add_tag(&mut self, key: &str, value: &str)
```

Add a metadata tag

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | Tag key |
| `value` | `&str` | Tag value |

**Examples:**

```rust
workflow.add_tag("environment", "production");
workflow.add_tag("team", "data-engineering");
```

<details>
<summary>Source</summary>

```rust
    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.metadata
            .tags
            .insert(key.to_string(), value.to_string());
    }
```

</details>

##### <span id="Workflow.remove_tag"></span><code>remove_​tag</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove_tag(&mut self, key: &str) -> Option<String>
```

Remove a tag from the workflow metadata

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | Tag key to remove |

**Returns:**

* `Some(String)` - The removed tag value if it existed
* `None` - If no tag with that key existed

**Examples:**

```rust
use cloacina::Workflow;

let mut workflow = Workflow::new("test-workflow");
workflow.add_tag("environment", "staging");

let removed = workflow.remove_tag("environment");
assert_eq!(removed, Some("staging".to_string()));
```

<details>
<summary>Source</summary>

```rust
    pub fn remove_tag(&mut self, key: &str) -> Option<String> {
        self.metadata.tags.remove(key)
    }
```

</details>

##### <span id="Workflow.add_task"></span><code>add_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn add_task(&mut self, task: Arc<dyn Task>) -> Result<(), WorkflowError>
```

Add a task to the Workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `Arc<dyn Task>` | Task to add |

**Returns:**

* `Ok(())` - If the task was added successfully
* `Err(WorkflowError)` - If the task ID is duplicate

**Examples:**

```rust
let mut workflow = Workflow::new("test_workflow");
let task = MyTask;

workflow.add_task(task)?;
assert!(workflow.get_task("my_task").is_some());
```

<details>
<summary>Source</summary>

```rust
    pub fn add_task(&mut self, task: Arc<dyn Task>) -> Result<(), WorkflowError> {
        let task_namespace = TaskNamespace::new(&self.tenant, &self.package, &self.name, task.id());

        // Check for duplicate task namespace
        if self.tasks.contains_key(&task_namespace) {
            return Err(WorkflowError::DuplicateTask(task_namespace.to_string()));
        }

        // Add task to dependency graph
        self.dependency_graph.add_node(task_namespace.clone());

        // Add dependencies
        for dep in task.dependencies() {
            self.dependency_graph
                .add_edge(task_namespace.clone(), dep.clone());
        }

        // Store the task
        self.tasks.insert(task_namespace, task);

        Ok(())
    }
```

</details>

##### <span id="Workflow.remove_task"></span><code>remove_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove_task(
    &mut self,
    namespace: &TaskNamespace,
) -> Option<Arc<dyn Task>>
```

Remove a task from the workflow

This removes the task and all its dependencies from the workflow.
Returns the removed task if it existed.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `-` | ID of the task to remove |

**Returns:**

* `Some(Arc<dyn Task>)` - The removed task if it existed
* `None` - If no task with that ID existed

**Examples:**

```rust
use cloacina::*;
use std::sync::Arc;

let mut workflow = Workflow::new("test-workflow");
let task = Arc::new(MockTask::new("task1", vec![]));
workflow.add_task(task.clone())?;

let removed = workflow.remove_task("task1");
assert!(removed.is_some());
assert!(workflow.get_task("task1").is_none());
```

<details>
<summary>Source</summary>

```rust
    pub fn remove_task(&mut self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        // Remove from dependency graph first
        self.dependency_graph.remove_node(namespace);

        // Remove and return the task
        self.tasks.remove(namespace)
    }
```

</details>

##### <span id="Workflow.remove_dependency"></span><code>remove_​dependency</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove_dependency(
    &mut self,
    from_task: &TaskNamespace,
    to_task: &TaskNamespace,
)
```

Remove a dependency between two tasks

This removes the dependency edge but keeps both tasks in the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `from_task` | `&TaskNamespace` | Task that currently depends on `to_task` |
| `to_task` | `&TaskNamespace` | Task that `from_task` currently depends on |

**Examples:**

```rust
use cloacina::*;
use std::sync::Arc;

let mut workflow = Workflow::new("test-workflow");
// Add tasks with dependency: task2 depends on task1
workflow.add_task(Arc::new(MockTask::new("task1", vec![])))?;
workflow.add_task(Arc::new(MockTask::new("task2", vec!["task1"])))?;

// Remove the dependency (task2 no longer depends on task1)
workflow.remove_dependency("task2", "task1");
```

<details>
<summary>Source</summary>

```rust
    pub fn remove_dependency(&mut self, from_task: &TaskNamespace, to_task: &TaskNamespace) {
        self.dependency_graph.remove_edge(from_task, to_task);
    }
```

</details>

##### <span id="Workflow.validate"></span><code>validate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn validate(&self) -> Result<(), ValidationError>
```

Validate the Workflow structure

Checks for:
- Empty workflows
- Missing dependencies
- Circular dependencies

**Returns:**

* `Ok(())` - If validation passes
* `Err(ValidationError)` - If validation fails

**Examples:**

```rust
match workflow.validate() {
    Ok(()) => println!("Workflow is valid"),
    Err(e) => println!("Validation error: {:?}", e),
}
```

<details>
<summary>Source</summary>

```rust
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check for empty Workflow
        if self.tasks.is_empty() {
            return Err(ValidationError::EmptyWorkflow);
        }

        // Check for missing dependencies
        for (task_namespace, task) in &self.tasks {
            for dependency in task.dependencies() {
                if !self.tasks.contains_key(dependency) {
                    return Err(ValidationError::MissingDependency {
                        task: task_namespace.to_string(),
                        dependency: dependency.to_string(),
                    });
                }
            }
        }

        // Check for cycles
        if self.dependency_graph.has_cycles() {
            let cycle = self
                .dependency_graph
                .find_cycle()
                .unwrap_or_default()
                .into_iter()
                .map(|ns| ns.to_string())
                .collect();
            return Err(ValidationError::CyclicDependency { cycle });
        }

        Ok(())
    }
```

</details>

##### <span id="Workflow.topological_sort"></span><code>topological_​sort</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn topological_sort(
    &self,
) -> Result<Vec<TaskNamespace>, ValidationError>
```

Get topological ordering of tasks

Returns tasks in dependency-safe execution order.

**Returns:**

* `Ok(Vec<String>)` - Task IDs in execution order
* `Err(ValidationError)` - If the workflow is invalid

**Examples:**

```rust
let execution_order = workflow.topological_sort()?;
println!("Execute tasks in order: {:?}", execution_order);
```

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> Result<Vec<TaskNamespace>, ValidationError> {
        self.validate()?;
        self.dependency_graph.topological_sort()
    }
```

</details>

##### <span id="Workflow.get_task"></span><code>get_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_task(
    &self,
    namespace: &TaskNamespace,
) -> Result<Arc<dyn Task>, WorkflowError>
```

Get a task by namespace

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `&TaskNamespace` | Task namespace to look up |

**Returns:**

* `Ok(Arc<dyn Task>)` - If the task exists
* `Err(WorkflowError)` - If no task with that namespace exists

<details>
<summary>Source</summary>

```rust
    pub fn get_task(&self, namespace: &TaskNamespace) -> Result<Arc<dyn Task>, WorkflowError> {
        self.tasks
            .get(namespace)
            .cloned()
            .ok_or_else(|| WorkflowError::TaskNotFound(namespace.to_string()))
    }
```

</details>

##### <span id="Workflow.get_dependencies"></span><code>get_​dependencies</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_dependencies(
    &self,
    namespace: &TaskNamespace,
) -> Result<&[TaskNamespace], WorkflowError>
```

Get dependencies for a task

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `&TaskNamespace` | Task namespace to get dependencies for |

**Returns:**

* `Ok(&[TaskNamespace])` - Array of dependency task namespaces
* `Err(WorkflowError)` - If the task doesn't exist

<details>
<summary>Source</summary>

```rust
    pub fn get_dependencies(
        &self,
        namespace: &TaskNamespace,
    ) -> Result<&[TaskNamespace], WorkflowError> {
        self.tasks
            .get(namespace)
            .map(|task| task.dependencies())
            .ok_or_else(|| WorkflowError::TaskNotFound(namespace.to_string()))
    }
```

</details>

##### <span id="Workflow.get_dependents"></span><code>get_​dependents</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_dependents(
    &self,
    namespace: &TaskNamespace,
) -> Result<Vec<TaskNamespace>, WorkflowError>
```

Get dependents of a task

Returns tasks that depend on the given task.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `&TaskNamespace` | Task namespace to get dependents for |

**Returns:**

* `Ok(Vec<TaskNamespace>)` - Vector of task namespaces that depend on the given task
* `Err(WorkflowError)` - If the task doesn't exist

**Examples:**

```rust
let namespace = TaskNamespace::new("public", "embedded", "test", "extract_data");
let dependents = workflow.get_dependents(&namespace)?;
println!("Tasks depending on extract_data: {:?}", dependents);
```

<details>
<summary>Source</summary>

```rust
    pub fn get_dependents(
        &self,
        namespace: &TaskNamespace,
    ) -> Result<Vec<TaskNamespace>, WorkflowError> {
        // First check if the task exists
        if !self.tasks.contains_key(namespace) {
            return Err(WorkflowError::TaskNotFound(namespace.to_string()));
        }

        // Return dependents (may be empty if no tasks depend on this one)
        Ok(self.dependency_graph.get_dependents(namespace))
    }
```

</details>

##### <span id="Workflow.subgraph"></span><code>subgraph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn subgraph(
    &self,
    task_namespaces: &[&TaskNamespace],
) -> Result<Workflow, SubgraphError>
```

Create a subgraph containing only specified tasks and their dependencies

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_ids` | `-` | Tasks to include in the subgraph |

**Returns:**

* `Ok(Workflow)` - New workflow containing only specified tasks
* `Err(SubgraphError)` - If any tasks don't exist or other errors

<details>
<summary>Source</summary>

```rust
    pub fn subgraph(&self, task_namespaces: &[&TaskNamespace]) -> Result<Workflow, SubgraphError> {
        let mut subgraph_tasks = HashSet::new();

        // Add specified tasks and recursively add their dependencies
        for &task_namespace in task_namespaces {
            if !self.tasks.contains_key(task_namespace) {
                return Err(SubgraphError::TaskNotFound(task_namespace.to_string()));
            }
            self.collect_dependencies(task_namespace, &mut subgraph_tasks);
        }

        // Create new Workflow with subset of tasks
        let mut workflow = Workflow::new(&format!("{}-subgraph", self.name));
        workflow.metadata = self.metadata.clone();

        for task_namespace in &subgraph_tasks {
            if let Some(task) = self.tasks.get(task_namespace) {
                // Clone the Arc<dyn Task> to share between workflows
                workflow.tasks.insert(task_namespace.clone(), task.clone());

                // Copy dependency graph edges for this task
                workflow.dependency_graph.add_node(task_namespace.clone());
                for dep in task.dependencies() {
                    if subgraph_tasks.contains(dep) {
                        workflow
                            .dependency_graph
                            .add_edge(task_namespace.clone(), dep.clone());
                    }
                }
            } else {
                return Err(SubgraphError::TaskNotFound(task_namespace.to_string()));
            }
        }

        Ok(workflow)
    }
```

</details>

##### <span id="Workflow.get_execution_levels"></span><code>get_​execution_​levels</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_execution_levels(
    &self,
) -> Result<Vec<Vec<TaskNamespace>>, ValidationError>
```

Get execution levels (tasks that can run in parallel)

Returns tasks grouped by execution level, where all tasks in a level
can run in parallel with each other.

**Returns:**

* `Ok(Vec<Vec<String>>)` - Tasks grouped by execution level
* `Err(ValidationError)` - If the workflow is invalid

**Examples:**

```rust
let levels = workflow.get_execution_levels()?;
for (level, tasks) in levels.iter().enumerate() {
    println!("Level {}: {} tasks can run in parallel", level, tasks.len());
    for task in tasks {
        println!("  - {}", task);
    }
}
```

<details>
<summary>Source</summary>

```rust
    pub fn get_execution_levels(&self) -> Result<Vec<Vec<TaskNamespace>>, ValidationError> {
        let sorted = self.topological_sort()?;
        let mut levels = Vec::new();
        let mut remaining: HashSet<TaskNamespace> = sorted.into_iter().collect();
        let mut completed = HashSet::new();

        while !remaining.is_empty() {
            let mut current_level = Vec::new();

            // Find tasks with all dependencies completed
            for task_namespace in &remaining {
                if let Some(task) = self.tasks.get(task_namespace) {
                    let all_deps_done = task
                        .dependencies()
                        .iter()
                        .all(|dep| completed.contains(dep));

                    if all_deps_done {
                        current_level.push(task_namespace.clone());
                    }
                }
            }

            // Remove current level tasks from remaining
            for task_namespace in &current_level {
                remaining.remove(task_namespace);
                completed.insert(task_namespace.clone());
            }

            levels.push(current_level);
        }

        Ok(levels)
    }
```

</details>

##### <span id="Workflow.get_roots"></span><code>get_​roots</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_roots(&self) -> Vec<TaskNamespace>
```

Get root tasks (tasks with no dependencies)

**Returns:**

Vector of task IDs that have no dependencies

**Examples:**

```rust
let roots = workflow.get_roots();
println!("Starting tasks: {:?}", roots);
```

<details>
<summary>Source</summary>

```rust
    pub fn get_roots(&self) -> Vec<TaskNamespace> {
        self.tasks
            .iter()
            .filter_map(|(namespace, task)| {
                if task.dependencies().is_empty() {
                    Some(namespace.clone())
                } else {
                    None
                }
            })
            .collect()
    }
```

</details>

##### <span id="Workflow.get_leaves"></span><code>get_​leaves</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_leaves(&self) -> Vec<TaskNamespace>
```

Get leaf tasks (tasks with no dependents)

**Returns:**

Vector of task IDs that no other tasks depend on

**Examples:**

```rust
let leaves = workflow.get_leaves();
println!("Final tasks: {:?}", leaves);
```

<details>
<summary>Source</summary>

```rust
    pub fn get_leaves(&self) -> Vec<TaskNamespace> {
        let all_dependencies: HashSet<TaskNamespace> = self
            .tasks
            .values()
            .flat_map(|task| task.dependencies().iter().cloned())
            .collect();

        self.tasks
            .keys()
            .filter(|&namespace| !all_dependencies.contains(namespace))
            .cloned()
            .collect()
    }
```

</details>

##### <span id="Workflow.can_run_parallel"></span><code>can_​run_​parallel</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn can_run_parallel(
    &self,
    task_a: &TaskNamespace,
    task_b: &TaskNamespace,
) -> bool
```

Check if two tasks can run in parallel

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_a` | `&TaskNamespace` | First task ID |
| `task_b` | `&TaskNamespace` | Second task ID |

**Returns:**

`true` if the tasks have no dependency relationship, `false` otherwise

**Examples:**

```rust
if workflow.can_run_parallel("fetch_users", "fetch_orders") {
    println!("These tasks can run simultaneously");
}
```

<details>
<summary>Source</summary>

```rust
    pub fn can_run_parallel(&self, task_a: &TaskNamespace, task_b: &TaskNamespace) -> bool {
        // Tasks can run in parallel if neither depends on the other
        !self.has_path(task_a, task_b) && !self.has_path(task_b, task_a)
    }
```

</details>

##### <span id="Workflow.calculate_version"></span><code>calculate_​version</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn calculate_version(&self) -> String
```

Calculate content-based version hash from Workflow structure and tasks.

The version is calculated by hashing:
1. Workflow topology (task IDs and dependencies)
2. Task definitions (code fingerprints if available)
3. Workflow configuration (name, description, tags)

**Returns:**

A hexadecimal string representing the content hash.

**Examples:**

```rust
let version = workflow.calculate_version();
assert_eq!(version.len(), 16); // 16-character hex string
```

<details>
<summary>Source</summary>

```rust
    pub fn calculate_version(&self) -> String {
        let mut hasher = DefaultHasher::new();

        // 1. Hash Workflow structure (topology)
        self.hash_topology(&mut hasher);

        // 2. Hash task definitions
        self.hash_task_definitions(&mut hasher);

        // 3. Hash Workflow configuration
        self.hash_configuration(&mut hasher);

        // Return hex representation of hash
        format!("{:016x}", hasher.finish())
    }
```

</details>

##### <span id="Workflow.get_task_ids"></span><code>get_​task_​ids</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_task_ids(&self) -> Vec<TaskNamespace>
```

Get all task namespaces in the workflow

**Returns:**

Vector of all task namespaces currently in the workflow

**Examples:**

```rust
let task_namespaces = workflow.get_task_ids();
println!("Tasks in workflow: {:?}", task_namespaces);
```

<details>
<summary>Source</summary>

```rust
    pub fn get_task_ids(&self) -> Vec<TaskNamespace> {
        self.tasks.keys().cloned().collect()
    }
```

</details>

##### <span id="Workflow.finalize"></span><code>finalize</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn finalize(mut self) -> Self
```

Finalize Workflow and calculate version.

This method calculates the content-based version hash and sets it
in the Workflow metadata. It should be called after all tasks have been
added and before the Workflow is used for execution.

**Returns:**

The Workflow with the calculated version set.

**Examples:**

```rust
// Version is empty before finalization
assert!(workflow.metadata().version.is_empty());

let finalized_workflow = workflow.finalize();
// Version is calculated after finalization
assert!(!finalized_workflow.metadata().version.is_empty());
```

<details>
<summary>Source</summary>

```rust
    pub fn finalize(mut self) -> Self {
        // Calculate content-based version
        let version = self.calculate_version();
        self.metadata.version = version;
        self
    }
```

</details>
