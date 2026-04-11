# cloacina::task <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina::task::TaskRegistry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Registry for managing collections of tasks and validating their dependencies.

The TaskRegistry provides a centralized container for tasks with built-in
validation of dependency relationships, cycle detection, and topological sorting.
Most users won't interact with this directly as the `#[task]` macro and
`workflow!` macro handle registration automatically.
Now supports namespaced task registration for isolation and conflict resolution.

**Examples:**

```rust,ignore
use cloacina::*;

let mut registry = TaskRegistry::new();

// Register namespaced tasks
let ns1 = TaskNamespace::embedded("customer_etl", "extract");
let ns2 = TaskNamespace::embedded("customer_etl", "transform");

registry.register_with_namespace(ns1, TestTask::new("extract", vec![]))?;
registry.register_with_namespace(ns2, TestTask::new("transform", vec!["extract"]))?;

// Look up tasks by namespace
let task = registry.get_task_by_namespace(&TaskNamespace::embedded("customer_etl", "extract"));
assert!(task.is_some());
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tasks` | `HashMap < TaskNamespace , Arc < dyn Task > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Create a new empty task registry

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }
```

</details>



##### `register` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register < T : Task + 'static > (& mut self , namespace : TaskNamespace , task : T ,) -> Result < () , RegistrationError >
```

Register a task in the registry

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `-` | The namespace for the task |
| `task` | `-` | The task to register |


**Returns:**

* `Ok(())` - If registration succeeds * `Err(RegistrationError)` - If the namespace is already taken

<details>
<summary>Source</summary>

```rust
    pub fn register<T: Task + 'static>(
        &mut self,
        namespace: TaskNamespace,
        task: T,
    ) -> Result<(), RegistrationError> {
        // Validate task ID is not empty
        if namespace.task_id.is_empty() {
            return Err(RegistrationError::InvalidTaskId {
                message: "Task ID cannot be empty".to_string(),
            });
        }

        // Check for duplicate namespace
        if self.tasks.contains_key(&namespace) {
            return Err(RegistrationError::DuplicateTaskId {
                id: namespace.to_string(),
            });
        }

        self.tasks.insert(namespace, Arc::new(task));
        Ok(())
    }
```

</details>



##### `register_arc` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_arc (& mut self , namespace : TaskNamespace , task : Arc < dyn Task > ,) -> Result < () , RegistrationError >
```

Register a boxed task in the registry (used internally)

<details>
<summary>Source</summary>

```rust
    pub fn register_arc(
        &mut self,
        namespace: TaskNamespace,
        task: Arc<dyn Task>,
    ) -> Result<(), RegistrationError> {
        // Validate task ID is not empty
        if namespace.task_id.is_empty() {
            return Err(RegistrationError::InvalidTaskId {
                message: "Task ID cannot be empty".to_string(),
            });
        }

        // Check for duplicate namespace
        if self.tasks.contains_key(&namespace) {
            return Err(RegistrationError::DuplicateTaskId {
                id: namespace.to_string(),
            });
        }

        self.tasks.insert(namespace, task);
        Ok(())
    }
```

</details>



##### `get_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (& self , namespace : & TaskNamespace) -> Option < Arc < dyn Task > >
```

Get a task by namespace

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `-` | The namespace to look up |


**Returns:**

* `Some(Arc<dyn Task>)` - If the task exists * `None` - If no task with that namespace is registered

<details>
<summary>Source</summary>

```rust
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        self.tasks.get(namespace).cloned()
    }
```

</details>



##### `task_ids` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_ids (& self) -> Vec < TaskNamespace >
```

Get all registered task namespaces

**Returns:**

A vector of all task namespaces currently registered

<details>
<summary>Source</summary>

```rust
    pub fn task_ids(&self) -> Vec<TaskNamespace> {
        self.tasks.keys().cloned().collect()
    }
```

</details>



##### `task_count` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_count (& self) -> usize
```

Get the number of registered tasks (O(1))

<details>
<summary>Source</summary>

```rust
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
```

</details>



##### `validate_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_dependencies (& self) -> Result < () , ValidationError >
```

Validate all task dependencies

Checks that:
- All dependencies exist as registered tasks
- No circular dependencies exist

**Returns:**

* `Ok(())` - If all dependencies are valid * `Err(ValidationError)` - If validation fails

<details>
<summary>Source</summary>

```rust
    pub fn validate_dependencies(&self) -> Result<(), ValidationError> {
        // Check for missing dependencies
        for (namespace, task) in &self.tasks {
            for dependency_namespace in task.dependencies() {
                if !self.tasks.contains_key(dependency_namespace) {
                    return Err(ValidationError::MissingDependency {
                        task: namespace.to_string(),
                        dependency: dependency_namespace.to_string(),
                    });
                }
            }
        }

        // Check for circular dependencies using DFS
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();

        for namespace in self.tasks.keys() {
            if !visited.get(namespace).unwrap_or(&false) {
                if let Err(cycle) = self.check_cycles(namespace, &mut visited, &mut rec_stack) {
                    return Err(ValidationError::CyclicDependency { cycle: vec![cycle] });
                }
            }
        }

        Ok(())
    }
```

</details>



##### `check_cycles` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn check_cycles (& self , namespace : & TaskNamespace , visited : & mut HashMap < TaskNamespace , bool > , rec_stack : & mut HashMap < TaskNamespace , bool > ,) -> Result < () , String >
```

Helper method to detect circular dependencies using DFS

<details>
<summary>Source</summary>

```rust
    fn check_cycles(
        &self,
        namespace: &TaskNamespace,
        visited: &mut HashMap<TaskNamespace, bool>,
        rec_stack: &mut HashMap<TaskNamespace, bool>,
    ) -> Result<(), String> {
        visited.insert(namespace.clone(), true);
        rec_stack.insert(namespace.clone(), true);

        if let Some(task) = self.tasks.get(namespace) {
            for dependency_namespace in task.dependencies() {
                if !visited.get(dependency_namespace).unwrap_or(&false) {
                    if let Err(cycle) = self.check_cycles(dependency_namespace, visited, rec_stack)
                    {
                        return Err(format!("{} -> {}", namespace.task_id, cycle));
                    }
                } else if *rec_stack.get(dependency_namespace).unwrap_or(&false) {
                    return Err(format!(
                        "{} -> {}",
                        namespace.task_id, dependency_namespace.task_id
                    ));
                }
            }
        }

        rec_stack.insert(namespace.clone(), false);
        Ok(())
    }
```

</details>



##### `topological_sort` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn topological_sort (& self) -> Result < Vec < TaskNamespace > , ValidationError >
```

Get tasks in topological order (dependencies first)

Returns tasks sorted so that dependencies come before the tasks that depend on them.
This is the safe execution order for the tasks.

**Returns:**

* `Ok(Vec<TaskNamespace>)` - Task namespaces in topological order * `Err(ValidationError)` - If dependencies are invalid or cycles exist

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> Result<Vec<TaskNamespace>, ValidationError> {
        // First validate dependencies
        self.validate_dependencies()?;

        let mut in_degree = HashMap::new();
        let mut adj_list = HashMap::new();

        // Initialize in-degree and adjacency list
        for namespace in self.tasks.keys() {
            in_degree.insert(namespace.clone(), 0);
            adj_list.insert(namespace.clone(), Vec::new());
        }

        // Build adjacency list and calculate in-degrees
        for (namespace, task) in &self.tasks {
            for dependency_namespace in task.dependencies() {
                if let Some(adj_list_entry) = adj_list.get_mut(dependency_namespace) {
                    adj_list_entry.push(namespace.clone());
                    *in_degree.get_mut(namespace).unwrap() += 1;
                }
            }
        }

        // Kahn's algorithm for topological sorting
        let mut queue = Vec::new();
        let mut result = Vec::new();

        // Add nodes with no incoming edges
        for (namespace, &degree) in &in_degree {
            if degree == 0 {
                queue.push(namespace.clone());
            }
        }

        while let Some(current) = queue.pop() {
            result.push(current.clone());

            // Process all neighbors
            for neighbor in &adj_list[&current] {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(neighbor.clone());
                }
            }
        }

        if result.len() != self.tasks.len() {
            return Err(ValidationError::InvalidGraph {
                message: "Graph contains cycles".to_string(),
            });
        }

        Ok(result)
    }
```

</details>





## Functions

### `cloacina::task::register_task_constructor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_task_constructor < F > (namespace : TaskNamespace , constructor : F) where F : Fn () -> Arc < dyn Task > + Send + Sync + 'static ,
```

Register a task constructor function globally with namespace

This is used internally by the `workflow!` macro to automatically register tasks.
Most users won't call this directly.

<details>
<summary>Source</summary>

```rust
pub fn register_task_constructor<F>(namespace: TaskNamespace, constructor: F)
where
    F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
{
    let mut registry = GLOBAL_TASK_REGISTRY.write();
    registry.insert(namespace.clone(), Box::new(constructor));
    tracing::debug!(
        "Successfully registered task constructor for namespace: {}",
        namespace
    );
}
```

</details>



### `cloacina::task::global_task_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn global_task_registry () -> GlobalTaskRegistry
```

Get the global task registry

This provides access to the global task registry used by the macro system.
Most users won't need to call this directly.

<details>
<summary>Source</summary>

```rust
pub fn global_task_registry() -> GlobalTaskRegistry {
    GLOBAL_TASK_REGISTRY.clone()
}
```

</details>



### `cloacina::task::get_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (namespace : & TaskNamespace) -> Option < Arc < dyn Task > >
```

Get a task instance from the global registry by namespace

This is a convenience function for getting task instances without
directly accessing the registry.

<details>
<summary>Source</summary>

```rust
pub fn get_task(namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
    let registry = GLOBAL_TASK_REGISTRY.read();
    registry.get(namespace).map(|constructor| constructor())
}
```

</details>
