---
title: "cloacina::workflow::graph"
description: "Dependency graph for workflow task relationships."
editUrl: false
sidebar:
  order: 204
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Dependency graph for workflow task relationships.

This module provides the `DependencyGraph` struct for managing
task dependencies, cycle detection, and topological sorting.

## Structs

### <span id="struct.DependencyGraph"></span><code style="font-size: 0.85em">struct DependencyGraph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Low-level representation of task dependencies.

The DependencyGraph manages the relationships between tasks as a directed graph,
providing cycle detection, topological sorting, and dependency analysis.

**Fields**

* `nodes`: HashSet<TaskNamespace> - Set of all task namespaces in the graph
* `edges`: HashMap<TaskNamespace, Vec<TaskNamespace>> - Map from task namespace to its dependencies

**Implementation Details**

The graph is implemented as a directed graph where:
- Nodes represent tasks
- Edges represent dependencies (from dependent to dependency)
- Cycles are detected using depth-first search
- Topological sorting uses Kahn's algorithm

**Examples:**

```rust
use cloacina::DependencyGraph;

let mut graph = DependencyGraph::new();
graph.add_node("task1".to_string());
graph.add_node("task2".to_string());
graph.add_edge("task2".to_string(), "task1".to_string());

assert!(!graph.has_cycles());
assert_eq!(graph.get_dependencies("task2"), Some(&vec!["task1".to_string()]));
```

#### Methods

##### <span id="DependencyGraph.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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

##### <span id="DependencyGraph.add_node"></span><code>add_​node</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn add_node(&mut self, node_id: TaskNamespace)
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

##### <span id="DependencyGraph.add_edge"></span><code>add_​edge</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn add_edge(&mut self, from: TaskNamespace, to: TaskNamespace)
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

##### <span id="DependencyGraph.remove_node"></span><code>remove_​node</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove_node(&mut self, node_id: &TaskNamespace)
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

##### <span id="DependencyGraph.remove_edge"></span><code>remove_​edge</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove_edge(&mut self, from: &TaskNamespace, to: &TaskNamespace)
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

##### <span id="DependencyGraph.get_dependencies"></span><code>get_​dependencies</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_dependencies(
    &self,
    node_id: &TaskNamespace,
) -> Option<&Vec<TaskNamespace>>
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

##### <span id="DependencyGraph.get_dependents"></span><code>get_​dependents</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_dependents(&self, node_id: &TaskNamespace) -> Vec<TaskNamespace>
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

##### <span id="DependencyGraph.has_cycles"></span><code>has_​cycles</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_cycles(&self) -> bool
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

##### <span id="DependencyGraph.topological_sort"></span><code>topological_​sort</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn topological_sort(
    &self,
) -> Result<Vec<TaskNamespace>, ValidationError>
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
