/*
 *  Copyright 2025-2026 Colliery Software
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ns(id: &str) -> TaskNamespace {
        TaskNamespace::new("public", "embedded", "test", id)
    }

    #[test]
    fn test_add_node_and_get_dependencies() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        graph.add_node(a.clone());

        // Node exists with empty dependencies
        let deps = graph.get_dependencies(&a);
        assert!(deps.is_some());
        assert!(deps.unwrap().is_empty());
    }

    #[test]
    fn test_add_edge_and_get_dependencies() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(b.clone(), a.clone());

        let deps = graph.get_dependencies(&b).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], a);
    }

    #[test]
    fn test_get_dependencies_nonexistent_node() {
        let graph = DependencyGraph::new();
        let missing = ns("missing");
        assert!(graph.get_dependencies(&missing).is_none());
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), a.clone());

        let dependents = graph.get_dependents(&a);
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&b));
        assert!(dependents.contains(&c));
    }

    #[test]
    fn test_get_dependents_no_dependents() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        graph.add_node(a.clone());

        let dependents = graph.get_dependents(&a);
        assert!(dependents.is_empty());
    }

    #[test]
    fn test_remove_node() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(b.clone(), a.clone());

        graph.remove_node(&a);
        assert!(graph.get_dependencies(&a).is_none());
        // b's dependency on a should be removed
        let deps = graph.get_dependencies(&b).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_remove_edge() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(b.clone(), a.clone());

        graph.remove_edge(&b, &a);
        let deps = graph.get_dependencies(&b).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_remove_edge_nonexistent() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        // Should not panic
        graph.remove_edge(&a, &b);
    }

    #[test]
    fn test_has_cycles_no_cycle() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        graph.add_edge(b.clone(), a.clone()); // b depends on a
        graph.add_edge(c.clone(), b.clone()); // c depends on b

        assert!(!graph.has_cycles());
    }

    #[test]
    fn test_has_cycles_with_cycle() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(a.clone(), b.clone()); // a depends on b
        graph.add_edge(b.clone(), a.clone()); // b depends on a

        assert!(graph.has_cycles());
    }

    #[test]
    fn test_has_cycles_three_node_cycle() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        graph.add_edge(a.clone(), b.clone());
        graph.add_edge(b.clone(), c.clone());
        graph.add_edge(c.clone(), a.clone());

        assert!(graph.has_cycles());
    }

    #[test]
    fn test_find_cycle_returns_some_when_cyclic() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(a.clone(), b.clone());
        graph.add_edge(b.clone(), a.clone());

        let cycle = graph.find_cycle();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 2);
    }

    #[test]
    fn test_find_cycle_returns_none_when_acyclic() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(b.clone(), a.clone());

        assert!(graph.find_cycle().is_none());
    }

    #[test]
    fn test_topological_sort_linear_chain() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        graph.add_edge(b.clone(), a.clone()); // b depends on a
        graph.add_edge(c.clone(), b.clone()); // c depends on b

        let sorted = graph.topological_sort().unwrap();
        let pos_a = sorted.iter().position(|x| *x == a).unwrap();
        let pos_b = sorted.iter().position(|x| *x == b).unwrap();
        let pos_c = sorted.iter().position(|x| *x == c).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_topological_sort_diamond() {
        // a -> b, a -> c, b -> d, c -> d
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        let d = ns("d");
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), a.clone());
        graph.add_edge(d.clone(), b.clone());
        graph.add_edge(d.clone(), c.clone());

        let sorted = graph.topological_sort().unwrap();
        let pos_a = sorted.iter().position(|x| *x == a).unwrap();
        let pos_b = sorted.iter().position(|x| *x == b).unwrap();
        let pos_c = sorted.iter().position(|x| *x == c).unwrap();
        let pos_d = sorted.iter().position(|x| *x == d).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }

    #[test]
    fn test_topological_sort_single_node() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        graph.add_node(a.clone());

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0], a);
    }

    #[test]
    fn test_topological_sort_independent_nodes() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        let c = ns("c");
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_topological_sort_cyclic_returns_error() {
        let mut graph = DependencyGraph::new();
        let a = ns("a");
        let b = ns("b");
        graph.add_edge(a.clone(), b.clone());
        graph.add_edge(b.clone(), a.clone());

        let result = graph.topological_sort();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::CyclicDependency { .. }
        ));
    }

    #[test]
    fn test_default_creates_empty_graph() {
        let graph = DependencyGraph::default();
        assert!(!graph.has_cycles());
    }
}
