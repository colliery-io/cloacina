# cloacina::workflow <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina::workflow::Workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Main Workflow structure for representing and managing task graphs.

A Workflow contains a collection of tasks with their dependency relationships,
ensuring that the graph remains acyclic and provides methods for execution
planning and analysis.

**Examples:**

```rust,ignore
use cloacina::*;

# struct TestTask { id: String, deps: Vec<String> }
# impl TestTask { fn new(id: &str, deps: Vec<&str>) -> Self { Self { id: id.to_string(), deps: deps.into_iter().map(|s| s.to_string()).collect() } } }
# use async_trait::async_trait;
# #[async_trait]
# impl Task for TestTask {
#     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
#     fn id(&self) -> &str { &self.id }
#     fn dependencies(&self) -> &[String] { &self.deps }
# }
let workflow = Workflow::builder("test-workflow")
    .description("Test workflow")
    .add_task(TestTask::new("task1", vec![]))?
    .add_task(TestTask::new("task2", vec!["task1"]))?
    .build()?;

assert_eq!(workflow.name(), "test-workflow");
assert!(!workflow.metadata().version.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `tenant` | `String` |  |
| `package` | `String` |  |
| `tasks` | `HashMap < TaskNamespace , Arc < dyn Task > >` |  |
| `dependency_graph` | `DependencyGraph` |  |
| `metadata` | `WorkflowMetadata` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (name : & str) -> Self
```

Create a new Workflow with the given name

Most users should use the `workflow!` macro or builder pattern instead.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | Unique name for the workflow |


**Examples:**

```rust,ignore
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



##### `builder` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn builder (name : & str) -> WorkflowBuilder
```

Create a Workflow builder for programmatic construction

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | Unique name for the workflow |


**Examples:**

```rust,ignore
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



##### `name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn name (& self) -> & str
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



##### `tenant` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn tenant (& self) -> & str
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



##### `set_tenant` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set_tenant (& mut self , tenant : & str)
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



##### `package` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn package (& self) -> & str
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



##### `set_package` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set_package (& mut self , package : & str)
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



##### `metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn metadata (& self) -> & WorkflowMetadata
```

Get the Workflow metadata

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `set_version` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set_version (& mut self , version : & str)
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



##### `set_description` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set_description (& mut self , description : & str)
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



##### `add_tag` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_tag (& mut self , key : & str , value : & str)
```

Add a metadata tag

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `-` | Tag key |
| `value` | `-` | Tag value |


**Examples:**

```rust,ignore
# use cloacina::*;
# let mut workflow = Workflow::new("test");
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



##### `remove_tag` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn remove_tag (& mut self , key : & str) -> Option < String >
```

Remove a tag from the workflow metadata

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `-` | Tag key to remove |


**Returns:**

* `Some(String)` - The removed tag value if it existed * `None` - If no tag with that key existed

**Examples:**

```
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



##### `add_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_task (& mut self , task : Arc < dyn Task >) -> Result < () , WorkflowError >
```

Add a task to the Workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `-` | Task to add |


**Returns:**

* `Ok(())` - If the task was added successfully * `Err(WorkflowError)` - If the task ID is duplicate

**Examples:**

```rust,ignore
# use cloacina::*;
# use async_trait::async_trait;
# struct MyTask;
# #[async_trait]
# impl Task for MyTask {
#     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
#     fn id(&self) -> &str { "my_task" }
#     fn dependencies(&self) -> &[String] { &[] }
# }
let mut workflow = Workflow::new("test_workflow");
let task = MyTask;

workflow.add_task(task)?;
assert!(workflow.get_task("my_task").is_some());
# Ok::<(), WorkflowError>(())
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



##### `remove_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn remove_task (& mut self , namespace : & TaskNamespace) -> Option < Arc < dyn Task > >
```

Remove a task from the workflow

This removes the task and all its dependencies from the workflow.
Returns the removed task if it existed.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `-` | ID of the task to remove |


**Returns:**

* `Some(Arc<dyn Task>)` - The removed task if it existed * `None` - If no task with that ID existed

**Examples:**

```ignore
use cloacina::*;
use std::sync::Arc;

let mut workflow = Workflow::new("test-workflow");
let task = Arc::new(MockTask::new("task1", vec![]));
workflow.add_task(task.clone())?;

let removed = workflow.remove_task("task1");
assert!(removed.is_some());
assert!(workflow.get_task("task1").is_none());
# Ok::<(), Box<dyn std::error::Error>>(())
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



##### `remove_dependency` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn remove_dependency (& mut self , from_task : & TaskNamespace , to_task : & TaskNamespace)
```

Remove a dependency between two tasks

This removes the dependency edge but keeps both tasks in the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `from_task` | `-` | Task that currently depends on `to_task` |
| `to_task` | `-` | Task that `from_task` currently depends on |


**Examples:**

```ignore
use cloacina::*;
use std::sync::Arc;

let mut workflow = Workflow::new("test-workflow");
// Add tasks with dependency: task2 depends on task1
workflow.add_task(Arc::new(MockTask::new("task1", vec![])))?;
workflow.add_task(Arc::new(MockTask::new("task2", vec!["task1"])))?;

// Remove the dependency (task2 no longer depends on task1)
workflow.remove_dependency("task2", "task1");
# Ok::<(), Box<dyn std::error::Error>>(())
```

<details>
<summary>Source</summary>

```rust
    pub fn remove_dependency(&mut self, from_task: &TaskNamespace, to_task: &TaskNamespace) {
        self.dependency_graph.remove_edge(from_task, to_task);
    }
```

</details>



##### `validate` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate (& self) -> Result < () , ValidationError >
```

Validate the Workflow structure

Checks for:
- Empty workflows
- Missing dependencies
- Circular dependencies

**Returns:**

* `Ok(())` - If validation passes * `Err(ValidationError)` - If validation fails

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `topological_sort` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn topological_sort (& self) -> Result < Vec < TaskNamespace > , ValidationError >
```

Get topological ordering of tasks

Returns tasks in dependency-safe execution order.

**Returns:**

* `Ok(Vec<String>)` - Task IDs in execution order * `Err(ValidationError)` - If the workflow is invalid

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
let execution_order = workflow.topological_sort()?;
println!("Execute tasks in order: {:?}", execution_order);
# Ok::<(), ValidationError>(())
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



##### `get_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (& self , namespace : & TaskNamespace) -> Result < Arc < dyn Task > , WorkflowError >
```

Get a task by namespace

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `-` | Task namespace to look up |


**Returns:**

* `Ok(Arc<dyn Task>)` - If the task exists * `Err(WorkflowError)` - If no task with that namespace exists

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



##### `get_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependencies (& self , namespace : & TaskNamespace ,) -> Result < & [TaskNamespace] , WorkflowError >
```

Get dependencies for a task

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `-` | Task namespace to get dependencies for |


**Returns:**

* `Ok(&[TaskNamespace])` - Array of dependency task namespaces * `Err(WorkflowError)` - If the task doesn't exist

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



##### `get_dependents` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependents (& self , namespace : & TaskNamespace ,) -> Result < Vec < TaskNamespace > , WorkflowError >
```

Get dependents of a task

Returns tasks that depend on the given task.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `-` | Task namespace to get dependents for |


**Returns:**

* `Ok(Vec<TaskNamespace>)` - Vector of task namespaces that depend on the given task * `Err(WorkflowError)` - If the task doesn't exist

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
let namespace = TaskNamespace::new("public", "embedded", "test", "extract_data");
let dependents = workflow.get_dependents(&namespace)?;
println!("Tasks depending on extract_data: {:?}", dependents);
# Ok::<(), WorkflowError>(())
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



##### `subgraph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn subgraph (& self , task_namespaces : & [& TaskNamespace]) -> Result < Workflow , SubgraphError >
```

Create a subgraph containing only specified tasks and their dependencies

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_ids` | `-` | Tasks to include in the subgraph |


**Returns:**

* `Ok(Workflow)` - New workflow containing only specified tasks * `Err(SubgraphError)` - If any tasks don't exist or other errors

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



##### `collect_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn collect_dependencies (& self , task_namespace : & TaskNamespace , collected : & mut HashSet < TaskNamespace > ,)
```

<details>
<summary>Source</summary>

```rust
    fn collect_dependencies(
        &self,
        task_namespace: &TaskNamespace,
        collected: &mut HashSet<TaskNamespace>,
    ) {
        if collected.contains(task_namespace) {
            return;
        }

        collected.insert(task_namespace.clone());

        if let Some(task) = self.tasks.get(task_namespace) {
            for dep in task.dependencies() {
                self.collect_dependencies(dep, collected);
            }
        }
    }
```

</details>



##### `get_execution_levels` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_execution_levels (& self) -> Result < Vec < Vec < TaskNamespace > > , ValidationError >
```

Get execution levels (tasks that can run in parallel)

Returns tasks grouped by execution level, where all tasks in a level
can run in parallel with each other.

**Returns:**

* `Ok(Vec<Vec<String>>)` - Tasks grouped by execution level * `Err(ValidationError)` - If the workflow is invalid

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
let levels = workflow.get_execution_levels()?;
for (level, tasks) in levels.iter().enumerate() {
    println!("Level {}: {} tasks can run in parallel", level, tasks.len());
    for task in tasks {
        println!("  - {}", task);
    }
}
# Ok::<(), ValidationError>(())
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



##### `get_roots` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_roots (& self) -> Vec < TaskNamespace >
```

Get root tasks (tasks with no dependencies)

**Returns:**

Vector of task IDs that have no dependencies

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `get_leaves` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_leaves (& self) -> Vec < TaskNamespace >
```

Get leaf tasks (tasks with no dependents)

**Returns:**

Vector of task IDs that no other tasks depend on

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `can_run_parallel` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn can_run_parallel (& self , task_a : & TaskNamespace , task_b : & TaskNamespace) -> bool
```

Check if two tasks can run in parallel

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_a` | `-` | First task ID |
| `task_b` | `-` | Second task ID |


**Returns:**

`true` if the tasks have no dependency relationship, `false` otherwise

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `has_path` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn has_path (& self , from : & TaskNamespace , to : & TaskNamespace) -> bool
```

<details>
<summary>Source</summary>

```rust
    fn has_path(&self, from: &TaskNamespace, to: &TaskNamespace) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if visited.contains(current) {
                continue;
            }
            visited.insert(current);

            if let Some(task) = self.tasks.get(current) {
                for dep in task.dependencies() {
                    if dep == to {
                        return true;
                    }
                    stack.push(dep);
                }
            }
        }

        false
    }
```

</details>



##### `calculate_version` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn calculate_version (& self) -> String
```

Calculate content-based version hash from Workflow structure and tasks.

The version is calculated by hashing:
1. Workflow topology (task IDs and dependencies)
2. Task definitions (code fingerprints if available)
3. Workflow configuration (name, description, tags)

**Returns:**

A hexadecimal string representing the content hash.

**Examples:**

```rust,ignore
# use cloacina::*;
# let mut workflow = Workflow::new("my-workflow");
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



##### `hash_topology` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn hash_topology (& self , hasher : & mut DefaultHasher)
```

<details>
<summary>Source</summary>

```rust
    fn hash_topology(&self, hasher: &mut DefaultHasher) {
        // Get tasks in deterministic order
        let mut task_ids: Vec<_> = self.tasks.keys().collect();
        task_ids.sort();

        for task_id in task_ids {
            task_id.hash(hasher);

            // Include dependencies in deterministic order
            let mut deps: Vec<_> = self.tasks[task_id].dependencies().to_vec();
            deps.sort();
            deps.hash(hasher);
        }
    }
```

</details>



##### `hash_task_definitions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn hash_task_definitions (& self , hasher : & mut DefaultHasher)
```

<details>
<summary>Source</summary>

```rust
    fn hash_task_definitions(&self, hasher: &mut DefaultHasher) {
        // Get tasks in deterministic order
        let mut task_ids: Vec<_> = self.tasks.keys().collect();
        task_ids.sort();

        for task_id in task_ids {
            let task = &self.tasks[task_id];

            // Hash task metadata
            task.id().hash(hasher);
            task.dependencies().hash(hasher);

            // Hash task code fingerprint (if available)
            if let Some(code_hash) = self.get_task_code_hash(task_id) {
                code_hash.hash(hasher);
            }
        }
    }
```

</details>



##### `hash_configuration` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn hash_configuration (& self , hasher : & mut DefaultHasher)
```

<details>
<summary>Source</summary>

```rust
    fn hash_configuration(&self, hasher: &mut DefaultHasher) {
        // Hash Workflow-level configuration (excluding version and timestamps)
        self.name.hash(hasher);
        self.tenant.hash(hasher);
        self.metadata.description.hash(hasher);

        // Hash tags in deterministic order
        let mut tags: Vec<_> = self.metadata.tags.iter().collect();
        tags.sort_by_key(|(k, _)| *k);
        tags.hash(hasher);
    }
```

</details>



##### `get_task_code_hash` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn get_task_code_hash (& self , task_namespace : & TaskNamespace) -> Option < String >
```

<details>
<summary>Source</summary>

```rust
    fn get_task_code_hash(&self, task_namespace: &TaskNamespace) -> Option<String> {
        self.tasks
            .get(task_namespace)
            .and_then(|task| task.code_fingerprint())
    }
```

</details>



##### `get_task_ids` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task_ids (& self) -> Vec < TaskNamespace >
```

Get all task namespaces in the workflow

**Returns:**

Vector of all task namespaces currently in the workflow

**Examples:**

```rust,ignore
# use cloacina::*;
# let workflow = Workflow::new("test");
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



##### `recreate_from_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn recreate_from_registry (& self) -> Result < Workflow , WorkflowError >
```

Create a new workflow instance from the same data as this workflow

This method recreates a workflow by fetching tasks from the global task registry
and rebuilding the workflow structure. This is useful for workflow registration
scenarios where you need to create a fresh workflow instance.

**Returns:**

A new workflow instance with the same structure and metadata

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if any tasks cannot be found in the global registry |


**Examples:**

```rust,ignore
# use cloacina::*;
# let original_workflow = Workflow::new("test");
let recreated_workflow = original_workflow.recreate_from_registry()?;
assert_eq!(original_workflow.name(), recreated_workflow.name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

<details>
<summary>Source</summary>

```rust
    pub fn recreate_from_registry(&self) -> Result<Workflow, WorkflowError> {
        let mut new_workflow = Workflow::new(&self.name);

        // Copy metadata (except version which will be recalculated)
        new_workflow.metadata.description = self.metadata.description.clone();
        new_workflow.metadata.tags = self.metadata.tags.clone();
        new_workflow.metadata.created_at = self.metadata.created_at;

        // Get the task registry
        let registry = crate::task::global_task_registry();
        let guard = registry.write();

        // Recreate all tasks from the registry
        for task_namespace in self.get_task_ids() {
            // Use the existing namespace
            let constructor = guard.get(&task_namespace).ok_or_else(|| {
                WorkflowError::TaskNotFound(format!(
                    "Task '{}' not found in global registry during workflow recreation",
                    task_namespace
                ))
            })?;

            // Create a new task instance
            let task = constructor();

            // Add the task to the new workflow
            new_workflow.add_task(task).map_err(|e| {
                WorkflowError::TaskError(format!(
                    "Failed to add task '{}' during recreation: {}",
                    task_namespace, e
                ))
            })?;
        }

        // Validate the recreated workflow
        new_workflow.validate().map_err(|e| {
            WorkflowError::ValidationError(format!("Recreated workflow validation failed: {}", e))
        })?;

        // Finalize and return
        Ok(new_workflow.finalize())
    }
```

</details>



##### `finalize` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn finalize (mut self) -> Self
```

Finalize Workflow and calculate version.

This method calculates the content-based version hash and sets it
in the Workflow metadata. It should be called after all tasks have been
added and before the Workflow is used for execution.

**Returns:**

The Workflow with the calculated version set.

**Examples:**

```rust,ignore
# use cloacina::*;
# let mut workflow = Workflow::new("my-workflow");
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
