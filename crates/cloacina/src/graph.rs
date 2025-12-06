/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! # Workflow Graph Data Structures
//!
//! This module provides comprehensive graph representations for workflow DAGs,
//! including serialization support for embedding in package metadata.
//!
//! ## Key Components
//!
//! - `WorkflowGraph`: Main graph structure using petgraph
//! - `TaskNode`: Node data containing task information
//! - `DependencyEdge`: Edge data for dependencies
//! - `WorkflowGraphData`: Serializable representation for metadata
//! - Graph algorithms for analysis and optimization

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node data for tasks in the workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskNode {
    /// Unique task identifier
    pub id: String,
    /// Human-readable task name
    pub name: String,
    /// Task description
    pub description: Option<String>,
    /// Source location (file:line)
    pub source_location: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Edge data representing dependencies between tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyEdge {
    /// Type of dependency (default: "data")
    pub dependency_type: String,
    /// Optional weight for scheduling priorities
    pub weight: Option<f64>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for DependencyEdge {
    fn default() -> Self {
        Self {
            dependency_type: "data".to_string(),
            weight: None,
            metadata: HashMap::new(),
        }
    }
}

/// Main workflow graph structure using petgraph
#[derive(Debug, Clone)]
pub struct WorkflowGraph {
    /// The underlying directed graph
    graph: DiGraph<TaskNode, DependencyEdge>,
    /// Map from task ID to node index for quick lookup
    task_index: HashMap<String, NodeIndex>,
}

impl WorkflowGraph {
    /// Create a new empty workflow graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            task_index: HashMap::new(),
        }
    }

    /// Add a task node to the graph
    pub fn add_task(&mut self, node: TaskNode) -> NodeIndex {
        let task_id = node.id.clone();
        let index = self.graph.add_node(node);
        self.task_index.insert(task_id, index);
        index
    }

    /// Add a dependency edge between tasks
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

    /// Get a task node by ID
    pub fn get_task(&self, task_id: &str) -> Option<&TaskNode> {
        self.task_index
            .get(task_id)
            .and_then(|&index| self.graph.node_weight(index))
    }

    /// Get an iterator over task IDs without allocation
    pub fn task_ids(&self) -> impl Iterator<Item = &str> {
        self.task_index.keys().map(|s| s.as_str())
    }

    /// Get the number of tasks in the graph (O(1))
    pub fn task_count(&self) -> usize {
        self.task_index.len()
    }

    /// Check if the graph has cycles
    pub fn has_cycles(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }

    /// Get topological ordering of tasks
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        match toposort(&self.graph, None) {
            Ok(indices) => Ok(indices
                .into_iter()
                .filter_map(|idx| self.graph.node_weight(idx).map(|n| n.id.clone()))
                .collect()),
            Err(_) => Err("Graph contains cycles".to_string()),
        }
    }

    /// Get an iterator over direct dependencies of a task
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

    /// Get an iterator over tasks that depend on the given task
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

    /// Get an iterator over root tasks (tasks with no dependencies)
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

    /// Get an iterator over leaf tasks (tasks with no dependents)
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

    /// Calculate the depth of each task (longest path from root)
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

    /// Find parallel execution groups (tasks that can run simultaneously)
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

    /// Convert to serializable format
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

    /// Create from serializable format
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
}

impl Default for WorkflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable representation of the workflow graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphData {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// All edges in the graph
    pub edges: Vec<GraphEdge>,
    /// Graph metadata and statistics
    pub metadata: GraphMetadata,
}

/// Serializable node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Task ID (matches TaskNode.id)
    pub id: String,
    /// Task node data
    pub data: TaskNode,
}

/// Serializable edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source task ID
    pub from: String,
    /// Target task ID
    pub to: String,
    /// Edge data
    pub data: DependencyEdge,
}

/// Graph metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadata {
    /// Total number of tasks
    pub task_count: usize,
    /// Total number of dependencies
    pub edge_count: usize,
    /// Whether the graph contains cycles
    pub has_cycles: bool,
    /// Number of depth levels in the graph
    pub depth_levels: usize,
    /// Root tasks (no dependencies)
    pub root_tasks: Vec<String>,
    /// Leaf tasks (no dependents)
    pub leaf_tasks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_graph_creation() {
        let mut graph = WorkflowGraph::new();

        // Add tasks
        let task1 = TaskNode {
            id: "task1".to_string(),
            name: "Task 1".to_string(),
            description: None,
            source_location: None,
            metadata: HashMap::new(),
        };
        let task2 = TaskNode {
            id: "task2".to_string(),
            name: "Task 2".to_string(),
            description: None,
            source_location: None,
            metadata: HashMap::new(),
        };

        graph.add_task(task1);
        graph.add_task(task2);

        // Add dependency
        graph
            .add_dependency("task2", "task1", DependencyEdge::default())
            .unwrap();

        assert_eq!(graph.task_count(), 2);
        assert!(!graph.has_cycles());
        assert_eq!(
            graph.get_dependencies("task2").collect::<Vec<_>>(),
            vec!["task1"]
        );
        assert_eq!(
            graph.get_dependents("task1").collect::<Vec<_>>(),
            vec!["task2"]
        );
    }

    #[test]
    fn test_parallel_groups() {
        let mut graph = WorkflowGraph::new();

        // Create diamond pattern: root -> (a, b) -> end
        for id in ["root", "a", "b", "end"] {
            graph.add_task(TaskNode {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                source_location: None,
                metadata: HashMap::new(),
            });
        }

        graph
            .add_dependency("root", "a", DependencyEdge::default())
            .unwrap();
        graph
            .add_dependency("root", "b", DependencyEdge::default())
            .unwrap();
        graph
            .add_dependency("a", "end", DependencyEdge::default())
            .unwrap();
        graph
            .add_dependency("b", "end", DependencyEdge::default())
            .unwrap();

        let groups = graph.find_parallel_groups();
        assert_eq!(groups.len(), 3); // root, (a,b), end
    }

    #[test]
    fn test_serialization() {
        let mut graph = WorkflowGraph::new();

        graph.add_task(TaskNode {
            id: "test".to_string(),
            name: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            source_location: Some("test.rs:42".to_string()),
            metadata: HashMap::new(),
        });

        let serializable = graph.to_serializable();
        let json = serde_json::to_string(&serializable).unwrap();
        let deserialized: WorkflowGraphData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.nodes.len(), 1);
        assert_eq!(deserialized.metadata.task_count, 1);
    }

    #[test]
    fn test_task_count() {
        let mut graph = WorkflowGraph::new();
        assert_eq!(graph.task_count(), 0);

        graph.add_task(TaskNode {
            id: "task1".to_string(),
            name: "Task 1".to_string(),
            description: None,
            source_location: None,
            metadata: HashMap::new(),
        });
        assert_eq!(graph.task_count(), 1);

        graph.add_task(TaskNode {
            id: "task2".to_string(),
            name: "Task 2".to_string(),
            description: None,
            source_location: None,
            metadata: HashMap::new(),
        });
        assert_eq!(graph.task_count(), 2);
    }

    #[test]
    fn test_task_ids_iterator() {
        let mut graph = WorkflowGraph::new();

        for id in ["a", "b", "c"] {
            graph.add_task(TaskNode {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                source_location: None,
                metadata: HashMap::new(),
            });
        }

        let ids: Vec<&str> = graph.task_ids().collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }
}
