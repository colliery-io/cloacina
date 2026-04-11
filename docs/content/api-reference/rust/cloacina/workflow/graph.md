# cloacina::workflow::graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Dependency graph for workflow task relationships.

This module provides the `DependencyGraph` struct for managing
task dependencies, cycle detection, and topological sorting.

## Structs

### `cloacina::workflow::graph::DependencyGraph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Low-level representation of task dependencies.

The DependencyGraph manages the relationships between tasks as a directed graph,
providing cycle detection, topological sorting, and dependency analysis.

**Examples:**

```rust,ignore
use cloacina::DependencyGraph;

let mut graph = DependencyGraph::new();
graph.add_node("task1".to_string());
graph.add_node("task2".to_string());
graph.add_edge("task2".to_string(), "task1".to_string());

assert!(!graph.has_cycles());
assert_eq!(graph.get_dependencies("task2"), Some(&vec!["task1".to_string()]));
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `nodes` | `HashSet < TaskNamespace >` |  |
| `edges` | `HashMap < TaskNamespace , Vec < TaskNamespace > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Create a new empty dependency graph

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }
```

</details>



##### `add_node` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_node (& mut self , node_id : TaskNamespace)
```

Add a node (task) to the graph

<details>
<summary>Source</summary>

```rust
    pub fn add_node(&mut self, node_id: TaskNamespace) {
        self.nodes.insert(node_id.clone());
        self.edges.entry(node_id).or_default();
    }
```

</details>



##### `add_edge` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_edge (& mut self , from : TaskNamespace , to : TaskNamespace)
```

Add an edge (dependency) to the graph

<details>
<summary>Source</summary>

```rust
    pub fn add_edge(&mut self, from: TaskNamespace, to: TaskNamespace) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.entry(from).or_default().push(to);
    }
```

</details>



##### `remove_node` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn remove_node (& mut self , node_id : & TaskNamespace)
```

Remove a node (task) from the graph This also removes all edges involving this node

<details>
<summary>Source</summary>

```rust
    pub fn remove_node(&mut self, node_id: &TaskNamespace) {
        self.nodes.remove(node_id);
        self.edges.remove(node_id);

        // Remove all edges pointing to this node
        for deps in self.edges.values_mut() {
            deps.retain(|dep| dep != node_id);
        }
    }
```

</details>



##### `remove_edge` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn remove_edge (& mut self , from : & TaskNamespace , to : & TaskNamespace)
```

Remove a specific edge (dependency) from the graph

<details>
<summary>Source</summary>

```rust
    pub fn remove_edge(&mut self, from: &TaskNamespace, to: &TaskNamespace) {
        if let Some(deps) = self.edges.get_mut(from) {
            deps.retain(|dep| dep != to);
        }
    }
```

</details>



##### `get_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependencies (& self , node_id : & TaskNamespace) -> Option < & Vec < TaskNamespace > >
```

Get dependencies for a task

<details>
<summary>Source</summary>

```rust
    pub fn get_dependencies(&self, node_id: &TaskNamespace) -> Option<&Vec<TaskNamespace>> {
        self.edges.get(node_id)
    }
```

</details>



##### `get_dependents` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependents (& self , node_id : & TaskNamespace) -> Vec < TaskNamespace >
```

Get tasks that depend on the given task

<details>
<summary>Source</summary>

```rust
    pub fn get_dependents(&self, node_id: &TaskNamespace) -> Vec<TaskNamespace> {
        self.edges
            .iter()
            .filter_map(|(k, v)| {
                if v.contains(node_id) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }
```

</details>



##### `has_cycles` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_cycles (& self) -> bool
```

Check if the graph contains cycles

<details>
<summary>Source</summary>

```rust
    pub fn has_cycles(&self) -> bool {
        let mut graph = Graph::<TaskNamespace, (), Directed>::new();
        let mut node_indices = HashMap::new();

        // Add nodes
        for node in &self.nodes {
            let index = graph.add_node(node.clone());
            node_indices.insert(node.clone(), index);
        }

        // Add edges
        for (from, deps) in &self.edges {
            if let Some(&from_index) = node_indices.get(from) {
                for dep in deps {
                    if let Some(&dep_index) = node_indices.get(dep) {
                        graph.add_edge(dep_index, from_index, ());
                    }
                }
            }
        }

        is_cyclic_directed(&graph)
    }
```

</details>



##### `topological_sort` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn topological_sort (& self) -> Result < Vec < TaskNamespace > , ValidationError >
```

Get tasks in topological order

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> Result<Vec<TaskNamespace>, ValidationError> {
        if self.has_cycles() {
            return Err(ValidationError::CyclicDependency {
                cycle: self
                    .find_cycle()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|ns| ns.to_string())
                    .collect(),
            });
        }

        let mut graph = Graph::<TaskNamespace, (), Directed>::new();
        let mut node_indices = HashMap::new();

        // Add nodes
        for node in &self.nodes {
            let index = graph.add_node(node.clone());
            node_indices.insert(node.clone(), index);
        }

        // Add edges (dependency -> dependent)
        for (from, deps) in &self.edges {
            if let Some(&from_index) = node_indices.get(from) {
                for dep in deps {
                    if let Some(&dep_index) = node_indices.get(dep) {
                        graph.add_edge(dep_index, from_index, ());
                    }
                }
            }
        }

        match toposort(&graph, None) {
            Ok(sorted) => {
                let result = sorted.into_iter().map(|idx| graph[idx].clone()).collect();
                Ok(result)
            }
            Err(_) => Err(ValidationError::CyclicDependency {
                cycle: self
                    .find_cycle()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|ns| ns.to_string())
                    .collect(),
            }),
        }
    }
```

</details>



##### `find_cycle` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn find_cycle (& self) -> Option < Vec < TaskNamespace > >
```

<details>
<summary>Source</summary>

```rust
    pub(crate) fn find_cycle(&self) -> Option<Vec<TaskNamespace>> {
        // Simple DFS-based cycle detection
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }
```

</details>



##### `dfs_cycle` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn dfs_cycle (& self , node : & TaskNamespace , visited : & mut HashSet < TaskNamespace > , rec_stack : & mut HashSet < TaskNamespace > , path : & mut Vec < TaskNamespace > ,) -> Option < Vec < TaskNamespace > >
```

<details>
<summary>Source</summary>

```rust
    fn dfs_cycle(
        &self,
        node: &TaskNamespace,
        visited: &mut HashSet<TaskNamespace>,
        rec_stack: &mut HashSet<TaskNamespace>,
        path: &mut Vec<TaskNamespace>,
    ) -> Option<Vec<TaskNamespace>> {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found cycle
                    let cycle_start = path.iter().position(|x| x == dep).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }
```

</details>
