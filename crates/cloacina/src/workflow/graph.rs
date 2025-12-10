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

//! Dependency graph for workflow task relationships.
//!
//! This module provides the `DependencyGraph` struct for managing
//! task dependencies, cycle detection, and topological sorting.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::{Directed, Graph};
use std::collections::{HashMap, HashSet};

use crate::error::ValidationError;
use crate::task::TaskNamespace;

/// Low-level representation of task dependencies.
///
/// The DependencyGraph manages the relationships between tasks as a directed graph,
/// providing cycle detection, topological sorting, and dependency analysis.
///
/// # Fields
///
/// * `nodes`: HashSet<TaskNamespace> - Set of all task namespaces in the graph
/// * `edges`: HashMap<TaskNamespace, Vec<TaskNamespace>> - Map from task namespace to its dependencies
///
/// # Implementation Details
///
/// The graph is implemented as a directed graph where:
/// - Nodes represent tasks
/// - Edges represent dependencies (from dependent to dependency)
/// - Cycles are detected using depth-first search
/// - Topological sorting uses Kahn's algorithm
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::DependencyGraph;
///
/// let mut graph = DependencyGraph::new();
/// graph.add_node("task1".to_string());
/// graph.add_node("task2".to_string());
/// graph.add_edge("task2".to_string(), "task1".to_string());
///
/// assert!(!graph.has_cycles());
/// assert_eq!(graph.get_dependencies("task2"), Some(&vec!["task1".to_string()]));
/// ```
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    nodes: HashSet<TaskNamespace>,
    edges: HashMap<TaskNamespace, Vec<TaskNamespace>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a node (task) to the graph
    pub fn add_node(&mut self, node_id: TaskNamespace) {
        self.nodes.insert(node_id.clone());
        self.edges.entry(node_id).or_default();
    }

    /// Add an edge (dependency) to the graph
    pub fn add_edge(&mut self, from: TaskNamespace, to: TaskNamespace) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.entry(from).or_default().push(to);
    }

    /// Remove a node (task) from the graph
    /// This also removes all edges involving this node
    pub fn remove_node(&mut self, node_id: &TaskNamespace) {
        self.nodes.remove(node_id);
        self.edges.remove(node_id);

        // Remove all edges pointing to this node
        for deps in self.edges.values_mut() {
            deps.retain(|dep| dep != node_id);
        }
    }

    /// Remove a specific edge (dependency) from the graph
    pub fn remove_edge(&mut self, from: &TaskNamespace, to: &TaskNamespace) {
        if let Some(deps) = self.edges.get_mut(from) {
            deps.retain(|dep| dep != to);
        }
    }

    /// Get dependencies for a task
    pub fn get_dependencies(&self, node_id: &TaskNamespace) -> Option<&Vec<TaskNamespace>> {
        self.edges.get(node_id)
    }

    /// Get tasks that depend on the given task
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

    /// Check if the graph contains cycles
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

    /// Get tasks in topological order
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
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}
