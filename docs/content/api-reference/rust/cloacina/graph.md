# cloacina::graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina::graph::TaskNode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

Node data for tasks in the workflow graph

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Unique task identifier |
| `name` | `String` | Human-readable task name |
| `description` | `Option < String >` | Task description |
| `source_location` | `Option < String >` | Source location (file:line) |
| `metadata` | `HashMap < String , serde_json :: Value >` | Additional metadata |



### `cloacina::graph::DependencyEdge`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

Edge data representing dependencies between tasks

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dependency_type` | `String` | Type of dependency (default: "data") |
| `weight` | `Option < f64 >` | Optional weight for scheduling priorities |
| `metadata` | `HashMap < String , serde_json :: Value >` | Additional metadata |



### `cloacina::graph::WorkflowGraph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Main workflow graph structure using petgraph

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph` | `DiGraph < TaskNode , DependencyEdge >` | The underlying directed graph |
| `task_index` | `HashMap < String , NodeIndex >` | Map from task ID to node index for quick lookup |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Create a new empty workflow graph

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            task_index: HashMap::new(),
        }
    }
```

</details>



##### `add_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_task (& mut self , node : TaskNode) -> NodeIndex
```

Add a task node to the graph

<details>
<summary>Source</summary>

```rust
    pub fn add_task(&mut self, node: TaskNode) -> NodeIndex {
        let task_id = node.id.clone();
        let index = self.graph.add_node(node);
        self.task_index.insert(task_id, index);
        index
    }
```

</details>



##### `add_dependency` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_dependency (& mut self , from_task_id : & str , to_task_id : & str , edge : DependencyEdge ,) -> Result < () , String >
```

Add a dependency edge between tasks

<details>
<summary>Source</summary>

```rust
    pub fn add_dependency(
        &mut self,
        from_task_id: &str,
        to_task_id: &str,
        edge: DependencyEdge,
    ) -> Result<(), String> {
        let from_index = self
            .task_index
            .get(from_task_id)
            .ok_or_else(|| format!("Task '{}' not found in graph", from_task_id))?;
        let to_index = self
            .task_index
            .get(to_task_id)
            .ok_or_else(|| format!("Task '{}' not found in graph", to_task_id))?;

        self.graph.add_edge(*from_index, *to_index, edge);
        Ok(())
    }
```

</details>



##### `get_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (& self , task_id : & str) -> Option < & TaskNode >
```

Get a task node by ID

<details>
<summary>Source</summary>

```rust
    pub fn get_task(&self, task_id: &str) -> Option<&TaskNode> {
        self.task_index
            .get(task_id)
            .and_then(|&index| self.graph.node_weight(index))
    }
```

</details>



##### `task_ids` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_ids (& self) -> impl Iterator < Item = & str >
```

Get an iterator over task IDs without allocation

<details>
<summary>Source</summary>

```rust
    pub fn task_ids(&self) -> impl Iterator<Item = &str> {
        self.task_index.keys().map(|s| s.as_str())
    }
```

</details>



##### `task_count` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_count (& self) -> usize
```

Get the number of tasks in the graph (O(1))

<details>
<summary>Source</summary>

```rust
    pub fn task_count(&self) -> usize {
        self.task_index.len()
    }
```

</details>



##### `has_cycles` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_cycles (& self) -> bool
```

Check if the graph has cycles

<details>
<summary>Source</summary>

```rust
    pub fn has_cycles(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }
```

</details>



##### `topological_sort` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn topological_sort (& self) -> Result < Vec < String > , String >
```

Get topological ordering of tasks

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        match toposort(&self.graph, None) {
            Ok(indices) => Ok(indices
                .into_iter()
                .filter_map(|idx| self.graph.node_weight(idx).map(|n| n.id.clone()))
                .collect()),
            Err(_) => Err("Graph contains cycles".to_string()),
        }
    }
```

</details>



##### `get_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependencies (& self , task_id : & str) -> impl Iterator < Item = & str >
```

Get an iterator over direct dependencies of a task

<details>
<summary>Source</summary>

```rust
    pub fn get_dependencies(&self, task_id: &str) -> impl Iterator<Item = &str> {
        self.task_index
            .get(task_id)
            .into_iter()
            .flat_map(|&node_idx| {
                self.graph
                    .edges_directed(node_idx, petgraph::Direction::Outgoing)
                    .filter_map(|edge| self.graph.node_weight(edge.target()).map(|n| n.id.as_str()))
            })
    }
```

</details>



##### `get_dependents` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_dependents (& self , task_id : & str) -> impl Iterator < Item = & str >
```

Get an iterator over tasks that depend on the given task

<details>
<summary>Source</summary>

```rust
    pub fn get_dependents(&self, task_id: &str) -> impl Iterator<Item = &str> {
        self.task_index
            .get(task_id)
            .into_iter()
            .flat_map(|&node_idx| {
                self.graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                    .filter_map(|edge| self.graph.node_weight(edge.source()).map(|n| n.id.as_str()))
            })
    }
```

</details>



##### `find_roots` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn find_roots (& self) -> impl Iterator < Item = & str >
```

Get an iterator over root tasks (tasks with no dependencies)

<details>
<summary>Source</summary>

```rust
    pub fn find_roots(&self) -> impl Iterator<Item = &str> {
        self.graph.node_indices().filter_map(|idx| {
            let has_no_deps = self
                .graph
                .edges_directed(idx, petgraph::Direction::Outgoing)
                .next()
                .is_none();
            if has_no_deps {
                self.graph.node_weight(idx).map(|n| n.id.as_str())
            } else {
                None
            }
        })
    }
```

</details>



##### `find_leaves` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn find_leaves (& self) -> impl Iterator < Item = & str >
```

Get an iterator over leaf tasks (tasks with no dependents)

<details>
<summary>Source</summary>

```rust
    pub fn find_leaves(&self) -> impl Iterator<Item = &str> {
        self.graph.node_indices().filter_map(|idx| {
            let has_no_dependents = self
                .graph
                .edges_directed(idx, petgraph::Direction::Incoming)
                .next()
                .is_none();
            if has_no_dependents {
                self.graph.node_weight(idx).map(|n| n.id.as_str())
            } else {
                None
            }
        })
    }
```

</details>



##### `calculate_depths` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn calculate_depths (& self) -> HashMap < String , usize >
```

Calculate the depth of each task (longest path from root)

<details>
<summary>Source</summary>

```rust
    pub fn calculate_depths(&self) -> HashMap<String, usize> {
        let mut depths = HashMap::new();

        // Initialize roots with depth 0
        for root_id in self.find_roots() {
            depths.insert(root_id.to_string(), 0);
        }

        // Process nodes in topological order
        let topo_order = self.topological_sort().unwrap_or_default();
        for task_id in topo_order {
            if let Some(&node_idx) = self.task_index.get(&task_id) {
                // Calculate depth based on maximum depth of dependencies + 1
                let mut max_dep_depth = 0;
                let mut has_dependencies = false;

                // Look at incoming edges (dependencies)
                for edge in self
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    if let Some(dependency) = self.graph.node_weight(edge.source()) {
                        has_dependencies = true;
                        if let Some(&dep_depth) = depths.get(&dependency.id) {
                            max_dep_depth = max_dep_depth.max(dep_depth);
                        }
                    }
                }

                let task_depth = if has_dependencies {
                    max_dep_depth + 1
                } else {
                    0
                };

                depths.insert(task_id, task_depth);
            }
        }

        depths
    }
```

</details>



##### `find_parallel_groups` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn find_parallel_groups (& self) -> Vec < Vec < String > >
```

Find parallel execution groups (tasks that can run simultaneously)

<details>
<summary>Source</summary>

```rust
    pub fn find_parallel_groups(&self) -> Vec<Vec<String>> {
        let depths = self.calculate_depths();
        let mut groups: HashMap<usize, Vec<String>> = HashMap::new();

        for (task_id, depth) in depths {
            groups.entry(depth).or_default().push(task_id);
        }

        let mut result: Vec<Vec<String>> = groups.into_values().collect();
        result.sort_by_key(|group| group.len());
        result
    }
```

</details>



##### `to_serializable` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_serializable (& self) -> WorkflowGraphData
```

Convert to serializable format

<details>
<summary>Source</summary>

```rust
    pub fn to_serializable(&self) -> WorkflowGraphData {
        let nodes: Vec<GraphNode> = self
            .graph
            .node_indices()
            .filter_map(|idx| {
                self.graph.node_weight(idx).map(|node| GraphNode {
                    id: node.id.clone(),
                    data: node.clone(),
                })
            })
            .collect();

        let edges: Vec<GraphEdge> = self
            .graph
            .edge_indices()
            .filter_map(|idx| {
                let (source, target) = self.graph.edge_endpoints(idx)?;
                let source_node = self.graph.node_weight(source)?;
                let target_node = self.graph.node_weight(target)?;
                let edge_data = self.graph.edge_weight(idx)?;

                Some(GraphEdge {
                    from: source_node.id.clone(),
                    to: target_node.id.clone(),
                    data: edge_data.clone(),
                })
            })
            .collect();

        let metadata = GraphMetadata {
            task_count: nodes.len(),
            edge_count: edges.len(),
            has_cycles: self.has_cycles(),
            depth_levels: self.calculate_depths().values().max().copied().unwrap_or(0) + 1,
            root_tasks: self.find_roots().map(|s| s.to_string()).collect(),
            leaf_tasks: self.find_leaves().map(|s| s.to_string()).collect(),
        };

        WorkflowGraphData {
            nodes,
            edges,
            metadata,
        }
    }
```

</details>



##### `from_serializable` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_serializable (data : & WorkflowGraphData) -> Result < Self , String >
```

Create from serializable format

<details>
<summary>Source</summary>

```rust
    pub fn from_serializable(data: &WorkflowGraphData) -> Result<Self, String> {
        let mut graph = WorkflowGraph::new();

        // Add all nodes first
        for node in &data.nodes {
            graph.add_task(node.data.clone());
        }

        // Add all edges
        for edge in &data.edges {
            graph.add_dependency(&edge.from, &edge.to, edge.data.clone())?;
        }

        Ok(graph)
    }
```

</details>





### `cloacina::graph::WorkflowGraphData`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Serializable representation of the workflow graph

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `nodes` | `Vec < GraphNode >` | All nodes in the graph |
| `edges` | `Vec < GraphEdge >` | All edges in the graph |
| `metadata` | `GraphMetadata` | Graph metadata and statistics |



### `cloacina::graph::GraphNode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Serializable node representation

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Task ID (matches TaskNode.id) |
| `data` | `TaskNode` | Task node data |



### `cloacina::graph::GraphEdge`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Serializable edge representation

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `from` | `String` | Source task ID |
| `to` | `String` | Target task ID |
| `data` | `DependencyEdge` | Edge data |



### `cloacina::graph::GraphMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Graph metadata and statistics

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_count` | `usize` | Total number of tasks |
| `edge_count` | `usize` | Total number of dependencies |
| `has_cycles` | `bool` | Whether the graph contains cycles |
| `depth_levels` | `usize` | Number of depth levels in the graph |
| `root_tasks` | `Vec < String >` | Root tasks (no dependencies) |
| `leaf_tasks` | `Vec < String >` | Leaf tasks (no dependents) |
