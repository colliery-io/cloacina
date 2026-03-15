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

//! Graph topology for continuous scheduling.
//!
//! Defines `DataSourceGraph`, `GraphEdge`, `ContinuousTaskConfig`, and
//! `JoinMode`. The graph is assembled from registered data sources and
//! continuous task declarations.
//!
//! See CLOACI-S-0008 for the full specification.

use super::accumulator::{SignalAccumulator, SimpleAccumulator};
use super::datasource::DataSource;
use super::trigger_policy::Immediate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// How to combine accumulator readiness for multi-input tasks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinMode {
    /// Fire when any accumulator is ready.
    Any,
    /// Fire when all accumulators are ready.
    All,
}

/// Late arrival policy for boundaries arriving after consumer watermark.
/// Only `AccumulateForward` is supported in the MVP (I-0023).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LateArrivalPolicy {
    /// Buffer the late boundary for the next accumulation cycle.
    AccumulateForward,
}

impl Default for LateArrivalPolicy {
    fn default() -> Self {
        LateArrivalPolicy::AccumulateForward
    }
}

/// An edge in the continuous graph: data source → accumulator → task.
pub struct GraphEdge {
    /// Data source name.
    pub source: String,
    /// Task name.
    pub task: String,
    /// Per-edge accumulator (behind Arc<Mutex> for thread-safe access).
    pub accumulator: Arc<Mutex<Box<dyn SignalAccumulator>>>,
    /// How to handle late boundaries on this edge.
    pub late_arrival_policy: LateArrivalPolicy,
}

impl std::fmt::Debug for GraphEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphEdge")
            .field("source", &self.source)
            .field("task", &self.task)
            .field("late_arrival_policy", &self.late_arrival_policy)
            .finish()
    }
}

/// Configuration for a continuous task within the graph.
#[derive(Debug)]
pub struct ContinuousTaskConfig {
    /// Indices into the graph's edges vector for triggering edges.
    pub triggered_edges: Vec<usize>,
    /// Data source names available but not triggering execution.
    pub referenced_sources: Vec<String>,
    /// How to combine accumulator readiness.
    pub join_mode: JoinMode,
}

/// The continuous reactive graph.
///
/// Contains data sources, task configurations, and edges wiring them together.
/// Assembled at startup from registered data sources and continuous task declarations.
pub struct DataSourceGraph {
    /// Registered data sources by name.
    pub data_sources: HashMap<String, DataSource>,
    /// Continuous task configurations by task ID.
    pub tasks: HashMap<String, ContinuousTaskConfig>,
    /// Edges: data source → accumulator → task.
    pub edges: Vec<GraphEdge>,
}

impl DataSourceGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            data_sources: HashMap::new(),
            tasks: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Get all edges for a given task.
    pub fn edges_for_task(&self, task_id: &str) -> Vec<&GraphEdge> {
        self.tasks
            .get(task_id)
            .map(|config| {
                config
                    .triggered_edges
                    .iter()
                    .filter_map(|&idx| self.edges.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all edges for a given data source.
    pub fn edges_for_source(&self, source_name: &str) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| e.source == source_name)
            .collect()
    }

    /// Get all task IDs in the graph.
    pub fn task_ids(&self) -> Vec<&str> {
        self.tasks.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for DataSourceGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DataSourceGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataSourceGraph")
            .field(
                "data_sources",
                &self.data_sources.keys().collect::<Vec<_>>(),
            )
            .field("tasks", &self.tasks.keys().collect::<Vec<_>>())
            .field("edges_count", &self.edges.len())
            .finish()
    }
}

/// Registration for a continuous task (used during graph assembly).
#[derive(Debug, Clone)]
pub struct ContinuousTaskRegistration {
    /// Task ID.
    pub id: String,
    /// Triggering data source names.
    pub sources: Vec<String>,
    /// Non-triggering data source names.
    pub referenced: Vec<String>,
}

/// Errors during graph assembly.
#[derive(Debug, thiserror::Error)]
pub enum GraphAssemblyError {
    #[error("unknown data source '{source_name}' referenced by task '{task_id}'")]
    UnknownSource {
        task_id: String,
        source_name: String,
    },
    #[error("unknown detector workflow '{workflow}' for data source '{source_name}'")]
    UnknownDetectorWorkflow {
        source_name: String,
        workflow: String,
    },
    #[error("duplicate task ID: '{0}'")]
    DuplicateTask(String),
}

/// Assemble a `DataSourceGraph` from registered data sources and task declarations.
///
/// For each task's `sources`, creates a `GraphEdge` with a default
/// `SimpleAccumulator` + `Immediate` policy. Referenced sources get no edges.
pub fn assemble_graph(
    data_sources: Vec<DataSource>,
    task_registrations: Vec<ContinuousTaskRegistration>,
) -> Result<DataSourceGraph, GraphAssemblyError> {
    let mut graph = DataSourceGraph::new();

    // Index data sources by name
    for ds in data_sources {
        graph.data_sources.insert(ds.name.clone(), ds);
    }

    // Process task registrations
    for reg in &task_registrations {
        if graph.tasks.contains_key(&reg.id) {
            return Err(GraphAssemblyError::DuplicateTask(reg.id.clone()));
        }

        // Validate all sources exist
        for source_name in &reg.sources {
            if !graph.data_sources.contains_key(source_name) {
                return Err(GraphAssemblyError::UnknownSource {
                    task_id: reg.id.clone(),
                    source_name: source_name.clone(),
                });
            }
        }

        // Validate all referenced sources exist
        for source_name in &reg.referenced {
            if !graph.data_sources.contains_key(source_name) {
                return Err(GraphAssemblyError::UnknownSource {
                    task_id: reg.id.clone(),
                    source_name: source_name.clone(),
                });
            }
        }

        // Create edges for each triggering source
        let mut triggered_edges = Vec::new();
        for source_name in &reg.sources {
            let edge_idx = graph.edges.len();
            graph.edges.push(GraphEdge {
                source: source_name.clone(),
                task: reg.id.clone(),
                accumulator: Arc::new(Mutex::new(Box::new(SimpleAccumulator::new(Box::new(
                    Immediate,
                )))
                    as Box<dyn SignalAccumulator>)),
                late_arrival_policy: LateArrivalPolicy::default(),
            });
            triggered_edges.push(edge_idx);
        }

        graph.tasks.insert(
            reg.id.clone(),
            ContinuousTaskConfig {
                triggered_edges,
                referenced_sources: reg.referenced.clone(),
                join_mode: JoinMode::Any,
            },
        );
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::datasource::{
        ConnectionDescriptor, DataConnection, DataConnectionError, DataSourceMetadata,
    };
    use std::any::Any;

    struct MockConnection;
    impl DataConnection for MockConnection {
        fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
            Ok(Box::new("mock"))
        }
        fn descriptor(&self) -> ConnectionDescriptor {
            ConnectionDescriptor {
                system_type: "mock".into(),
                location: "test".into(),
            }
        }
        fn system_metadata(&self) -> serde_json::Value {
            serde_json::json!({})
        }
    }

    fn make_data_source(name: &str) -> DataSource {
        DataSource {
            name: name.to_string(),
            connection: Box::new(MockConnection),
            detector_workflow: format!("detect_{}", name),
            lineage: DataSourceMetadata::default(),
        }
    }

    #[test]
    fn test_assemble_simple_graph() {
        let sources = vec![make_data_source("events"), make_data_source("config")];
        let tasks = vec![ContinuousTaskRegistration {
            id: "aggregate".to_string(),
            sources: vec!["events".to_string()],
            referenced: vec!["config".to_string()],
        }];

        let graph = assemble_graph(sources, tasks).unwrap();
        assert_eq!(graph.data_sources.len(), 2);
        assert_eq!(graph.tasks.len(), 1);
        assert_eq!(graph.edges.len(), 1);

        let config = &graph.tasks["aggregate"];
        assert_eq!(config.triggered_edges.len(), 1);
        assert_eq!(config.referenced_sources, vec!["config"]);
        assert_eq!(config.join_mode, JoinMode::Any);
    }

    #[test]
    fn test_assemble_multi_source_task() {
        let sources = vec![make_data_source("clicks"), make_data_source("impressions")];
        let tasks = vec![ContinuousTaskRegistration {
            id: "join_task".to_string(),
            sources: vec!["clicks".to_string(), "impressions".to_string()],
            referenced: vec![],
        }];

        let graph = assemble_graph(sources, tasks).unwrap();
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.tasks["join_task"].triggered_edges.len(), 2);
    }

    #[test]
    fn test_assemble_unknown_source() {
        let sources = vec![make_data_source("events")];
        let tasks = vec![ContinuousTaskRegistration {
            id: "task_a".to_string(),
            sources: vec!["nonexistent".to_string()],
            referenced: vec![],
        }];

        let result = assemble_graph(sources, tasks);
        assert!(result.is_err());
        match result.unwrap_err() {
            GraphAssemblyError::UnknownSource { source_name, .. } => {
                assert_eq!(source_name, "nonexistent");
            }
            other => panic!("expected UnknownSource, got: {:?}", other),
        }
    }

    #[test]
    fn test_assemble_unknown_referenced_source() {
        let sources = vec![make_data_source("events")];
        let tasks = vec![ContinuousTaskRegistration {
            id: "task_a".to_string(),
            sources: vec!["events".to_string()],
            referenced: vec!["missing_config".to_string()],
        }];

        let result = assemble_graph(sources, tasks);
        assert!(result.is_err());
    }

    #[test]
    fn test_assemble_duplicate_task() {
        let sources = vec![make_data_source("events")];
        let tasks = vec![
            ContinuousTaskRegistration {
                id: "dup".to_string(),
                sources: vec!["events".to_string()],
                referenced: vec![],
            },
            ContinuousTaskRegistration {
                id: "dup".to_string(),
                sources: vec!["events".to_string()],
                referenced: vec![],
            },
        ];

        let result = assemble_graph(sources, tasks);
        assert!(result.is_err());
        match result.unwrap_err() {
            GraphAssemblyError::DuplicateTask(name) => assert_eq!(name, "dup"),
            other => panic!("expected DuplicateTask, got: {:?}", other),
        }
    }

    #[test]
    fn test_edges_for_task() {
        let sources = vec![make_data_source("a"), make_data_source("b")];
        let tasks = vec![ContinuousTaskRegistration {
            id: "task1".to_string(),
            sources: vec!["a".to_string(), "b".to_string()],
            referenced: vec![],
        }];

        let graph = assemble_graph(sources, tasks).unwrap();
        let edges = graph.edges_for_task("task1");
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_edges_for_source() {
        let sources = vec![make_data_source("events")];
        let tasks = vec![
            ContinuousTaskRegistration {
                id: "task_a".to_string(),
                sources: vec!["events".to_string()],
                referenced: vec![],
            },
            ContinuousTaskRegistration {
                id: "task_b".to_string(),
                sources: vec!["events".to_string()],
                referenced: vec![],
            },
        ];

        let graph = assemble_graph(sources, tasks).unwrap();
        let edges = graph.edges_for_source("events");
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_empty_graph() {
        let graph = assemble_graph(vec![], vec![]).unwrap();
        assert!(graph.data_sources.is_empty());
        assert!(graph.tasks.is_empty());
        assert!(graph.edges.is_empty());
    }
}
