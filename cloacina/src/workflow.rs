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

//! # Workflow Management
//!
//! This module provides the core functionality for creating and managing task workflows in Cloacina.
//! It implements a directed acyclic graph (DAG) of tasks with dependency management, validation,
//! and content-based versioning.
//!
//! ## Core Components
//!
//! - `Workflow`: Main structure for managing task graphs and execution
//! - `WorkflowMetadata`: Versioning and metadata management
//! - `DependencyGraph`: Low-level dependency tracking and cycle detection
//! - `WorkflowBuilder`: Fluent interface for workflow construction
//!
//! ## Key Features
//!
//! - Directed acyclic graph (DAG) task dependencies
//! - Automatic cycle detection and validation
//! - Content-based versioning for reliable pipeline management
//! - Parallel execution planning
//! - Global workflow registry
//!
//! ## Type Definitions
//!
//! ```rust
//! pub struct Workflow {
//!     name: String,
//!     tasks: HashMap<String, Box<dyn Task>>,
//!     dependency_graph: DependencyGraph,
//!     metadata: WorkflowMetadata,
//! }
//!
//! pub struct WorkflowMetadata {
//!     pub created_at: DateTime<Utc>,
//!     pub version: String,
//!     pub description: Option<String>,
//!     pub tags: HashMap<String, String>,
//! }
//!
//! pub struct DependencyGraph {
//!     nodes: HashSet<String>,
//!     edges: HashMap<String, Vec<String>>,
//! }
//! ```
//!
//! ## Error Types
//!
//! - `WorkflowError`: Errors during workflow construction and management
//! - `ValidationError`: Errors during workflow validation
//! - `SubgraphError`: Errors during subgraph operations
//!
//! ## Constants
//!
//! - `GLOBAL_WORKFLOW_REGISTRY`: Global registry for workflow constructors
//!
//! ## Public Functions
//!
//! - `register_workflow_constructor`: Register a workflow constructor
//! - `global_workflow_registry`: Access the global workflow registry
//! - `get_all_workflows`: Get all registered workflows

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::{Directed, Graph};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::error::{SubgraphError, ValidationError, WorkflowError};
use crate::task::Task;

/// Metadata information for a Workflow.
///
/// Contains versioning, creation timestamps, and arbitrary tags for
/// organizing and managing workflow instances.
///
/// # Fields
///
/// * `created_at`: DateTime<Utc> - When the workflow was created
/// * `version`: String - Content-based version hash
/// * `description`: Option<String> - Optional human-readable description
/// * `tags`: HashMap<String, String> - Arbitrary key-value tags for organization
///
/// # Implementation Details
///
/// The version field is automatically calculated based on:
/// - Workflow topology (task IDs and dependencies)
/// - Task definitions (code fingerprints)
/// - Workflow configuration (name, description, tags)
///
/// # Examples
///
/// ```rust
/// use cloacina::WorkflowMetadata;
/// use std::collections::HashMap;
///
/// let mut metadata = WorkflowMetadata::default();
/// metadata.version = "a1b2c3d4".to_string();
/// metadata.description = Some("Production ETL pipeline".to_string());
/// metadata.tags.insert("team".to_string(), "data-engineering".to_string());
/// ```
#[derive(Debug, Clone)]
pub struct WorkflowMetadata {
    /// When the workflow was created
    pub created_at: DateTime<Utc>,
    /// Content-based version hash
    pub version: String,
    /// Optional human-readable description
    pub description: Option<String>,
    /// Arbitrary key-value tags for organization
    pub tags: HashMap<String, String>,
}

impl Default for WorkflowMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            version: String::new(), // Will be auto-calculated
            description: None,
            tags: HashMap::new(),
        }
    }
}

/// Low-level representation of task dependencies.
///
/// The DependencyGraph manages the relationships between tasks as a directed graph,
/// providing cycle detection, topological sorting, and dependency analysis.
///
/// # Fields
///
/// * `nodes`: HashSet<String> - Set of all task IDs in the graph
/// * `edges`: HashMap<String, Vec<String>> - Map from task ID to its dependencies
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
/// ```rust
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
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<String>>,
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
    pub fn add_node(&mut self, node_id: String) {
        self.nodes.insert(node_id.clone());
        self.edges.entry(node_id).or_insert_with(Vec::new);
    }

    /// Add an edge (dependency) to the graph
    pub fn add_edge(&mut self, from: String, to: String) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.entry(from).or_insert_with(Vec::new).push(to);
    }

    /// Get dependencies for a task
    pub fn get_dependencies(&self, node_id: &str) -> Option<&Vec<String>> {
        self.edges.get(node_id)
    }

    /// Get tasks that depend on the given task
    pub fn get_dependents(&self, node_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter_map(|(k, v)| {
                if v.contains(&node_id.to_string()) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the graph contains cycles
    pub fn has_cycles(&self) -> bool {
        let mut graph = Graph::<String, (), Directed>::new();
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
    pub fn topological_sort(&self) -> Result<Vec<String>, ValidationError> {
        if self.has_cycles() {
            return Err(ValidationError::CyclicDependency {
                cycle: self.find_cycle().unwrap_or_default(),
            });
        }

        let mut graph = Graph::<String, (), Directed>::new();
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
                cycle: self.find_cycle().unwrap_or_default(),
            }),
        }
    }

    fn find_cycle(&self) -> Option<Vec<String>> {
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
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

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

/// Main Workflow structure for representing and managing task graphs.
///
/// A Workflow contains a collection of tasks with their dependency relationships,
/// ensuring that the graph remains acyclic and provides methods for execution
/// planning and analysis.
///
/// # Fields
///
/// * `name`: String - Unique identifier for the workflow
/// * `tasks`: HashMap<String, Box<dyn Task>> - Map of task IDs to task implementations
/// * `dependency_graph`: DependencyGraph - Internal representation of task dependencies
/// * `metadata`: WorkflowMetadata - Versioning and metadata information
///
/// # Implementation Details
///
/// The Workflow structure provides:
/// - Task dependency management
/// - Cycle detection and validation
/// - Content-based versioning
/// - Parallel execution planning
/// - Subgraph operations
///
/// # Examples
///
/// ```rust
/// use cloacina::*;
///
/// # struct TestTask { id: String, deps: Vec<String> }
/// # impl TestTask { fn new(id: &str, deps: Vec<&str>) -> Self { Self { id: id.to_string(), deps: deps.into_iter().map(|s| s.to_string()).collect() } } }
/// # use async_trait::async_trait;
/// # #[async_trait]
/// # impl Task for TestTask {
/// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
/// #     fn id(&self) -> &str { &self.id }
/// #     fn dependencies(&self) -> &[String] { &self.deps }
/// # }
/// let workflow = Workflow::builder("test-workflow")
///     .description("Test workflow")
///     .add_task(TestTask::new("task1", vec![]))?
///     .add_task(TestTask::new("task2", vec!["task1"]))?
///     .build()?;
///
/// assert_eq!(workflow.name(), "test-workflow");
/// assert!(!workflow.metadata().version.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Workflow {
    name: String,
    tasks: HashMap<String, Box<dyn Task>>,
    dependency_graph: DependencyGraph,
    metadata: WorkflowMetadata,
}

impl std::fmt::Debug for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workflow")
            .field("name", &self.name)
            .field("task_count", &self.tasks.len())
            .field("dependency_graph", &self.dependency_graph)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl Workflow {
    /// Create a new Workflow with the given name
    ///
    /// Most users should use the `workflow!` macro or builder pattern instead.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the workflow
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::Workflow;
    ///
    /// let workflow = Workflow::new("my_workflow");
    /// assert_eq!(workflow.name(), "my_workflow");
    /// ```
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tasks: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            metadata: WorkflowMetadata::default(),
        }
    }

    /// Create a Workflow builder for programmatic construction
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the workflow
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::*;
    ///
    /// let builder = Workflow::builder("my_workflow")
    ///     .description("Example workflow");
    /// ```
    pub fn builder(name: &str) -> WorkflowBuilder {
        WorkflowBuilder::new(name)
    }

    /// Get the Workflow name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the Workflow metadata
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let metadata = workflow.metadata();
    /// println!("Version: {}", metadata.version);
    /// println!("Created: {}", metadata.created_at);
    /// ```
    pub fn metadata(&self) -> &WorkflowMetadata {
        &self.metadata
    }

    /// Set the Workflow version manually
    ///
    /// Note: Workflows built with the `workflow!` macro or builder automatically
    /// calculate content-based versions.
    pub fn set_version(&mut self, version: &str) {
        self.metadata.version = version.to_string();
    }

    /// Set the Workflow description
    pub fn set_description(&mut self, description: &str) {
        self.metadata.description = Some(description.to_string());
    }

    /// Add a metadata tag
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key
    /// * `value` - Tag value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let mut workflow = Workflow::new("test");
    /// workflow.add_tag("environment", "production");
    /// workflow.add_tag("team", "data-engineering");
    /// ```
    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.metadata
            .tags
            .insert(key.to_string(), value.to_string());
    }

    /// Add a task to the Workflow
    ///
    /// # Arguments
    ///
    /// * `task` - Task to add
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the task was added successfully
    /// * `Err(WorkflowError)` - If the task ID is duplicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # use async_trait::async_trait;
    /// # struct MyTask;
    /// # #[async_trait]
    /// # impl Task for MyTask {
    /// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
    /// #     fn id(&self) -> &str { "my_task" }
    /// #     fn dependencies(&self) -> &[String] { &[] }
    /// # }
    /// let mut workflow = Workflow::new("test_workflow");
    /// let task = MyTask;
    ///
    /// workflow.add_task(task)?;
    /// assert!(workflow.get_task("my_task").is_some());
    /// # Ok::<(), WorkflowError>(())
    /// ```
    pub fn add_task<T: Task + 'static>(&mut self, task: T) -> Result<(), WorkflowError> {
        let task_id = task.id().to_string();

        // Check for duplicate task ID
        if self.tasks.contains_key(&task_id) {
            return Err(WorkflowError::DuplicateTask(task_id));
        }

        // Add task to dependency graph
        self.dependency_graph.add_node(task_id.clone());

        // Add dependencies
        for dep in task.dependencies() {
            self.dependency_graph.add_edge(task_id.clone(), dep.clone());
        }

        // Store the task
        self.tasks.insert(task_id, Box::new(task));

        Ok(())
    }

    /// Add a boxed task to the Workflow
    ///
    /// This method accepts an already-boxed task, which is useful for dynamic
    /// task registration scenarios (like Python bindings) where tasks are
    /// stored as trait objects in registries.
    ///
    /// # Arguments
    ///
    /// * `task` - Boxed task to add
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the task was added successfully
    /// * `Err(WorkflowError)` - If the task ID is duplicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # use async_trait::async_trait;
    /// # struct MyTask;
    /// # #[async_trait]
    /// # impl Task for MyTask {
    /// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
    /// #     fn id(&self) -> &str { "my_task" }
    /// #     fn dependencies(&self) -> &[String] { &[] }
    /// # }
    /// let mut workflow = Workflow::new("test_workflow");
    /// let boxed_task: Box<dyn Task> = Box::new(MyTask);
    ///
    /// workflow.add_boxed_task(boxed_task)?;
    /// assert!(workflow.get_task("my_task").is_some());
    /// # Ok::<(), WorkflowError>(())
    /// ```
    pub fn add_boxed_task(&mut self, task: Box<dyn Task>) -> Result<(), WorkflowError> {
        let task_id = task.id().to_string();

        // Check for duplicate task ID
        if self.tasks.contains_key(&task_id) {
            return Err(WorkflowError::DuplicateTask(task_id));
        }

        // Add task to dependency graph
        self.dependency_graph.add_node(task_id.clone());

        // Add dependencies
        for dep in task.dependencies() {
            self.dependency_graph.add_edge(task_id.clone(), dep.clone());
        }

        // Store the task (already boxed)
        self.tasks.insert(task_id, task);

        Ok(())
    }

    /// Validate the Workflow structure
    ///
    /// Checks for:
    /// - Empty workflows
    /// - Missing dependencies
    /// - Circular dependencies
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If validation passes
    /// * `Err(ValidationError)` - If validation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// match workflow.validate() {
    ///     Ok(()) => println!("Workflow is valid"),
    ///     Err(e) => println!("Validation error: {:?}", e),
    /// }
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check for empty Workflow
        if self.tasks.is_empty() {
            return Err(ValidationError::EmptyWorkflow);
        }

        // Check for missing dependencies
        for (task_id, task) in &self.tasks {
            for dependency in task.dependencies() {
                if !self.tasks.contains_key(dependency) {
                    return Err(ValidationError::MissingDependency {
                        task: task_id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }

        // Check for cycles
        if self.dependency_graph.has_cycles() {
            let cycle = self.dependency_graph.find_cycle().unwrap_or_default();
            return Err(ValidationError::CyclicDependency { cycle });
        }

        Ok(())
    }

    /// Get topological ordering of tasks
    ///
    /// Returns tasks in dependency-safe execution order.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - Task IDs in execution order
    /// * `Err(ValidationError)` - If the workflow is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let execution_order = workflow.topological_sort()?;
    /// println!("Execute tasks in order: {:?}", execution_order);
    /// # Ok::<(), ValidationError>(())
    /// ```
    pub fn topological_sort(&self) -> Result<Vec<String>, ValidationError> {
        self.validate()?;
        self.dependency_graph.topological_sort()
    }

    /// Get a task by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Task ID to look up
    ///
    /// # Returns
    ///
    /// * `Some(&dyn Task)` - If the task exists
    /// * `None` - If no task with that ID exists
    pub fn get_task(&self, id: &str) -> Option<&dyn Task> {
        self.tasks.get(id).map(|task| task.as_ref())
    }

    /// Get dependencies for a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task ID to get dependencies for
    ///
    /// # Returns
    ///
    /// * `Some(&[String])` - Array of dependency task IDs
    /// * `None` - If the task doesn't exist
    pub fn get_dependencies(&self, task_id: &str) -> Option<&[String]> {
        self.tasks.get(task_id).map(|task| task.dependencies())
    }

    /// Get dependents of a task
    ///
    /// Returns tasks that depend on the given task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task ID to get dependents for
    ///
    /// # Returns
    ///
    /// Vector of task IDs that depend on the given task
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let dependents = workflow.get_dependents("extract_data");
    /// println!("Tasks depending on extract_data: {:?}", dependents);
    /// ```
    pub fn get_dependents(&self, task_id: &str) -> Vec<String> {
        self.dependency_graph.get_dependents(task_id)
    }

    /// Create a subgraph containing only specified tasks and their dependencies
    ///
    /// # Arguments
    ///
    /// * `task_ids` - Tasks to include in the subgraph
    ///
    /// # Returns
    ///
    /// * `Ok(Workflow)` - New workflow containing only specified tasks
    /// * `Err(SubgraphError)` - If any tasks don't exist or other errors
    pub fn subgraph(&self, task_ids: &[&str]) -> Result<Workflow, SubgraphError> {
        let mut subgraph_tasks = HashSet::new();

        // Add specified tasks and recursively add their dependencies
        for &task_id in task_ids {
            if !self.tasks.contains_key(task_id) {
                return Err(SubgraphError::TaskNotFound(task_id.to_string()));
            }
            self.collect_dependencies(task_id, &mut subgraph_tasks);
        }

        // Create new Workflow with subset of tasks
        let mut workflow = Workflow::new(&format!("{}-subgraph", self.name));
        workflow.metadata = self.metadata.clone();

        for task_id in subgraph_tasks {
            if let Some(_task) = self.tasks.get(&task_id) {
                // Note: This requires cloning tasks, which may not be possible
                // In practice, we might need a different approach for subgraphs
                return Err(SubgraphError::UnsupportedOperation(
                    "Task cloning not supported".to_string(),
                ));
            }
        }

        Ok(workflow)
    }

    fn collect_dependencies(&self, task_id: &str, collected: &mut HashSet<String>) {
        if collected.contains(task_id) {
            return;
        }

        collected.insert(task_id.to_string());

        if let Some(task) = self.tasks.get(task_id) {
            for dep in task.dependencies() {
                self.collect_dependencies(dep, collected);
            }
        }
    }

    /// Get execution levels (tasks that can run in parallel)
    ///
    /// Returns tasks grouped by execution level, where all tasks in a level
    /// can run in parallel with each other.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Vec<String>>)` - Tasks grouped by execution level
    /// * `Err(ValidationError)` - If the workflow is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let levels = workflow.get_execution_levels()?;
    /// for (level, tasks) in levels.iter().enumerate() {
    ///     println!("Level {}: {} tasks can run in parallel", level, tasks.len());
    ///     for task in tasks {
    ///         println!("  - {}", task);
    ///     }
    /// }
    /// # Ok::<(), ValidationError>(())
    /// ```
    pub fn get_execution_levels(&self) -> Result<Vec<Vec<String>>, ValidationError> {
        let sorted = self.topological_sort()?;
        let mut levels = Vec::new();
        let mut remaining: HashSet<String> = sorted.into_iter().collect();
        let mut completed = HashSet::new();

        while !remaining.is_empty() {
            let mut current_level = Vec::new();

            // Find tasks with all dependencies completed
            for task_id in &remaining {
                if let Some(task) = self.tasks.get(task_id) {
                    let all_deps_done = task
                        .dependencies()
                        .iter()
                        .all(|dep| completed.contains(dep));

                    if all_deps_done {
                        current_level.push(task_id.clone());
                    }
                }
            }

            // Remove current level tasks from remaining
            for task_id in &current_level {
                remaining.remove(task_id);
                completed.insert(task_id.clone());
            }

            levels.push(current_level);
        }

        Ok(levels)
    }

    /// Get root tasks (tasks with no dependencies)
    ///
    /// # Returns
    ///
    /// Vector of task IDs that have no dependencies
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let roots = workflow.get_roots();
    /// println!("Starting tasks: {:?}", roots);
    /// ```
    pub fn get_roots(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter_map(|(id, task)| {
                if task.dependencies().is_empty() {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get leaf tasks (tasks with no dependents)
    ///
    /// # Returns
    ///
    /// Vector of task IDs that no other tasks depend on
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let leaves = workflow.get_leaves();
    /// println!("Final tasks: {:?}", leaves);
    /// ```
    pub fn get_leaves(&self) -> Vec<String> {
        let all_dependencies: HashSet<String> = self
            .tasks
            .values()
            .flat_map(|task| task.dependencies().iter().cloned())
            .collect();

        self.tasks
            .keys()
            .filter(|&id| !all_dependencies.contains(id))
            .cloned()
            .collect()
    }

    /// Check if two tasks can run in parallel
    ///
    /// # Arguments
    ///
    /// * `task_a` - First task ID
    /// * `task_b` - Second task ID
    ///
    /// # Returns
    ///
    /// `true` if the tasks have no dependency relationship, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// if workflow.can_run_parallel("fetch_users", "fetch_orders") {
    ///     println!("These tasks can run simultaneously");
    /// }
    /// ```
    pub fn can_run_parallel(&self, task_a: &str, task_b: &str) -> bool {
        // Tasks can run in parallel if neither depends on the other
        !self.has_path(task_a, task_b) && !self.has_path(task_b, task_a)
    }

    fn has_path(&self, from: &str, to: &str) -> bool {
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

    /// Calculate content-based version hash from Workflow structure and tasks.
    ///
    /// The version is calculated by hashing:
    /// 1. Workflow topology (task IDs and dependencies)
    /// 2. Task definitions (code fingerprints if available)
    /// 3. Workflow configuration (name, description, tags)
    ///
    /// # Returns
    ///
    /// A hexadecimal string representing the content hash.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let mut workflow = Workflow::new("my-workflow");
    /// let version = workflow.calculate_version();
    /// assert_eq!(version.len(), 16); // 16-character hex string
    /// ```
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

    fn hash_configuration(&self, hasher: &mut DefaultHasher) {
        // Hash Workflow-level configuration (excluding version and timestamps)
        self.name.hash(hasher);
        self.metadata.description.hash(hasher);

        // Hash tags in deterministic order
        let mut tags: Vec<_> = self.metadata.tags.iter().collect();
        tags.sort_by_key(|(k, _)| *k);
        tags.hash(hasher);
    }

    fn get_task_code_hash(&self, task_id: &str) -> Option<String> {
        self.tasks
            .get(task_id)
            .and_then(|task| task.code_fingerprint())
    }

    /// Get all task IDs in the workflow
    ///
    /// # Returns
    ///
    /// Vector of all task IDs currently in the workflow
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let task_ids = workflow.get_task_ids();
    /// println!("Tasks in workflow: {:?}", task_ids);
    /// ```
    pub fn get_task_ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Create a new workflow instance from the same data as this workflow
    ///
    /// This method recreates a workflow by fetching tasks from the global task registry
    /// and rebuilding the workflow structure. This is useful for workflow registration
    /// scenarios where you need to create a fresh workflow instance.
    ///
    /// # Returns
    ///
    /// A new workflow instance with the same structure and metadata
    ///
    /// # Errors
    ///
    /// Returns an error if any tasks cannot be found in the global registry
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let original_workflow = Workflow::new("test");
    /// let recreated_workflow = original_workflow.recreate_from_registry()?;
    /// assert_eq!(original_workflow.name(), recreated_workflow.name());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn recreate_from_registry(&self) -> Result<Workflow, WorkflowError> {
        let mut new_workflow = Workflow::new(&self.name);
        
        // Copy metadata (except version which will be recalculated)
        new_workflow.metadata.description = self.metadata.description.clone();
        new_workflow.metadata.tags = self.metadata.tags.clone();
        new_workflow.metadata.created_at = self.metadata.created_at;
        
        // Get the task registry
        let registry = crate::task::global_task_registry();
        let guard = registry.lock().map_err(|e| {
            WorkflowError::RegistryError(format!("Failed to access task registry: {}", e))
        })?;
        
        // Recreate all tasks from the registry
        for task_id in self.get_task_ids() {
            let constructor = guard.get(&task_id).ok_or_else(|| {
                WorkflowError::TaskNotFound(format!(
                    "Task '{}' not found in global registry during workflow recreation", 
                    task_id
                ))
            })?;
            
            // Create a new task instance
            let task = constructor();
            
            // Add the task to the new workflow
            new_workflow.add_boxed_task(task).map_err(|e| {
                WorkflowError::TaskError(format!("Failed to add task '{}' during recreation: {}", task_id, e))
            })?;
        }
        
        // Validate the recreated workflow
        new_workflow.validate().map_err(|e| {
            WorkflowError::ValidationError(format!("Recreated workflow validation failed: {}", e))
        })?;
        
        // Finalize and return
        Ok(new_workflow.finalize())
    }

    /// Finalize Workflow and calculate version.
    ///
    /// This method calculates the content-based version hash and sets it
    /// in the Workflow metadata. It should be called after all tasks have been
    /// added and before the Workflow is used for execution.
    ///
    /// # Returns
    ///
    /// The Workflow with the calculated version set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # let mut workflow = Workflow::new("my-workflow");
    /// // Version is empty before finalization
    /// assert!(workflow.metadata().version.is_empty());
    ///
    /// let finalized_workflow = workflow.finalize();
    /// // Version is calculated after finalization
    /// assert!(!finalized_workflow.metadata().version.is_empty());
    /// ```
    pub fn finalize(mut self) -> Self {
        // Calculate content-based version
        let version = self.calculate_version();
        self.metadata.version = version;
        self
    }
}

/// Builder pattern for convenient and fluent Workflow construction.
///
/// The WorkflowBuilder provides a chainable interface for constructing Workflows,
/// making it easy to set metadata, add tasks, and validate the structure
/// before finalizing the Workflow.
///
/// # Fields
///
/// * `workflow`: Workflow - The workflow being constructed
///
/// # Implementation Details
///
/// The builder pattern provides:
/// - Fluent interface for workflow construction
/// - Automatic validation during build
/// - Content-based version calculation
/// - Metadata management
///
/// # Examples
///
/// ```rust
/// use cloacina::*;
///
/// # struct TestTask { id: String, deps: Vec<String> }
/// # impl TestTask { fn new(id: &str, deps: Vec<&str>) -> Self { Self { id: id.to_string(), deps: deps.into_iter().map(|s| s.to_string()).collect() } } }
/// # use async_trait::async_trait;
/// # #[async_trait]
/// # impl Task for TestTask {
/// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
/// #     fn id(&self) -> &str { &self.id }
/// #     fn dependencies(&self) -> &[String] { &self.deps }
/// # }
/// let workflow = Workflow::builder("etl-pipeline")
///     .description("Customer data ETL pipeline")
///     .tag("environment", "staging")
///     .tag("owner", "data-team")
///     .add_task(TestTask::new("extract_customers", vec![]))?
///     .add_task(TestTask::new("validate_data", vec!["extract_customers"]))?
///     .validate()?
///     .build()?;
///
/// assert_eq!(workflow.name(), "etl-pipeline");
/// assert!(!workflow.metadata().version.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct WorkflowBuilder {
    workflow: Workflow,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: &str) -> Self {
        Self {
            workflow: Workflow::new(name),
        }
    }

    /// Set the workflow description
    pub fn description(mut self, description: &str) -> Self {
        self.workflow.set_description(description);
        self
    }

    /// Add a tag to the workflow metadata
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        self.workflow.add_tag(key, value);
        self
    }

    /// Add a task to the workflow
    pub fn add_task<T: Task + 'static>(mut self, task: T) -> Result<Self, WorkflowError> {
        self.workflow.add_task(task)?;
        Ok(self)
    }

    /// Validate the workflow structure
    pub fn validate(self) -> Result<Self, ValidationError> {
        self.workflow.validate()?;
        Ok(self)
    }

    /// Build the final workflow with automatic version calculation
    pub fn build(self) -> Result<Workflow, ValidationError> {
        self.workflow.validate()?;
        // Auto-calculate version when building
        Ok(self.workflow.finalize())
    }
}

/// Global registry for automatically registering workflows created with the `workflow!` macro
static GLOBAL_WORKFLOW_REGISTRY: Lazy<
    Arc<Mutex<HashMap<String, Box<dyn Fn() -> Workflow + Send + Sync>>>>,
> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Register a workflow constructor function globally
///
/// This is used internally by the `workflow!` macro to automatically register workflows.
/// Most users won't call this directly.
pub fn register_workflow_constructor<F>(workflow_name: String, constructor: F)
where
    F: Fn() -> Workflow + Send + Sync + 'static,
{
    let mut registry = GLOBAL_WORKFLOW_REGISTRY.lock().unwrap();
    registry.insert(workflow_name, Box::new(constructor));
}

/// Get the global workflow registry
///
/// This provides access to the global workflow registry used by the macro system.
/// Most users won't need to call this directly.
pub fn global_workflow_registry(
) -> Arc<Mutex<HashMap<String, Box<dyn Fn() -> Workflow + Send + Sync>>>> {
    GLOBAL_WORKFLOW_REGISTRY.clone()
}

/// Get all workflows from the global registry
///
/// Returns instances of all workflows registered with the `workflow!` macro.
///
/// # Examples
///
/// ```rust
/// use cloacina::*;
///
/// let all_workflows = get_all_workflows();
/// for workflow in all_workflows {
///     println!("Found workflow: {}", workflow.name());
/// }
/// ```
pub fn get_all_workflows() -> Vec<Workflow> {
    let registry = GLOBAL_WORKFLOW_REGISTRY.lock().unwrap();
    registry.values().map(|constructor| constructor()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::error::TaskError;
    use crate::init_test_logging;
    use async_trait::async_trait;

    // Test task implementation
    struct TestTask {
        id: String,
        dependencies: Vec<String>,
        fingerprint: Option<String>,
    }

    impl TestTask {
        fn new(id: &str, dependencies: Vec<&str>) -> Self {
            Self {
                id: id.to_string(),
                dependencies: dependencies.into_iter().map(|s| s.to_string()).collect(),
                fingerprint: None,
            }
        }

        fn with_fingerprint(mut self, fingerprint: &str) -> Self {
            self.fingerprint = Some(fingerprint.to_string());
            self
        }
    }

    #[async_trait]
    impl Task for TestTask {
        async fn execute(
            &self,
            context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &self.dependencies
        }

        fn code_fingerprint(&self) -> Option<String> {
            self.fingerprint.clone()
        }
    }

    #[test]
    fn test_workflow_creation() {
        init_test_logging();

        let workflow = Workflow::new("test-workflow");
        assert_eq!(workflow.name(), "test-workflow");
        // Version starts empty until finalized
        assert_eq!(workflow.metadata().version, "");
    }

    #[test]
    fn test_workflow_add_task() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        let task = TestTask::new("task1", vec![]);

        assert!(workflow.add_task(task).is_ok());
        assert!(workflow.get_task("task1").is_some());
    }

    #[test]
    fn test_workflow_validation() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);

        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();

        assert!(workflow.validate().is_ok());
    }

    #[test]
    fn test_workflow_cycle_detection() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let task1 = TestTask::new("task1", vec!["task2"]);
        let task2 = TestTask::new("task2", vec!["task1"]);

        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();

        assert!(matches!(
            workflow.validate(),
            Err(ValidationError::CyclicDependency { .. })
        ));
    }

    #[test]
    fn test_workflow_topological_sort() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);
        let task3 = TestTask::new("task3", vec!["task1", "task2"]);

        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();
        workflow.add_task(task3).unwrap();

        let sorted = workflow.topological_sort().unwrap();

        let pos1 = sorted.iter().position(|x| x == "task1").unwrap();
        let pos2 = sorted.iter().position(|x| x == "task2").unwrap();
        let pos3 = sorted.iter().position(|x| x == "task3").unwrap();

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_workflow_builder_auto_versioning() {
        init_test_logging();

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);

        let workflow = Workflow::builder("test-workflow")
            .description("Test Workflow with auto-versioning")
            .tag("env", "test")
            .add_task(task1)
            .unwrap()
            .add_task(task2)
            .unwrap()
            .validate()
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(workflow.name(), "test-workflow");
        // Version should be auto-calculated
        assert!(!workflow.metadata().version.is_empty());
        assert_ne!(workflow.metadata().version, "1.0"); // Not the old default
        assert_eq!(
            workflow.metadata().description,
            Some("Test Workflow with auto-versioning".to_string())
        );
        assert_eq!(
            workflow.metadata().tags.get("env"),
            Some(&"test".to_string())
        );
    }

    #[test]
    fn test_execution_levels() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![]);
        let task3 = TestTask::new("task3", vec!["task1", "task2"]);
        let task4 = TestTask::new("task4", vec!["task3"]);

        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();
        workflow.add_task(task3).unwrap();
        workflow.add_task(task4).unwrap();

        let levels = workflow.get_execution_levels().unwrap();

        // Level 0: task1, task2 (no dependencies)
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&"task1".to_string()));
        assert!(levels[0].contains(&"task2".to_string()));

        // Level 1: task3 (depends on task1, task2)
        assert_eq!(levels[1].len(), 1);
        assert!(levels[1].contains(&"task3".to_string()));

        // Level 2: task4 (depends on task3)
        assert_eq!(levels[2].len(), 1);
        assert!(levels[2].contains(&"task4".to_string()));
    }

    #[test]
    fn test_workflow_version_consistency() {
        init_test_logging();

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);

        // Build same Workflow twice
        let workflow1 = Workflow::builder("test-workflow")
            .description("Test Workflow")
            .add_task(task1)
            .unwrap()
            .add_task(task2)
            .unwrap()
            .build()
            .unwrap();

        let task1_copy = TestTask::new("task1", vec![]);
        let task2_copy = TestTask::new("task2", vec!["task1"]);

        let workflow2 = Workflow::builder("test-workflow")
            .description("Test Workflow")
            .add_task(task1_copy)
            .unwrap()
            .add_task(task2_copy)
            .unwrap()
            .build()
            .unwrap();

        // Same content should produce same version
        assert_eq!(workflow1.metadata().version, workflow2.metadata().version);
    }

    #[test]
    fn test_workflow_version_changes() {
        init_test_logging();

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);

        let workflow1 = Workflow::builder("test-workflow")
            .description("Original description")
            .add_task(task1)
            .unwrap()
            .add_task(task2)
            .unwrap()
            .build()
            .unwrap();

        let task1_copy = TestTask::new("task1", vec![]);
        let task2_copy = TestTask::new("task2", vec!["task1"]);

        let workflow2 = Workflow::builder("test-workflow")
            .description("Changed description") // Different description
            .add_task(task1_copy)
            .unwrap()
            .add_task(task2_copy)
            .unwrap()
            .build()
            .unwrap();

        // Different content should produce different versions
        assert_ne!(workflow1.metadata().version, workflow2.metadata().version);
    }

    #[test]
    fn test_workflow_finalize() {
        init_test_logging();

        let mut workflow = Workflow::new("my-workflow");
        let task1 = TestTask::new("task1", vec![]);
        workflow.add_task(task1).unwrap();

        // Version is empty before finalization
        assert!(workflow.metadata().version.is_empty());

        let finalized_workflow = workflow.finalize();
        // Version is calculated after finalization
        assert!(!finalized_workflow.metadata().version.is_empty());
        assert_eq!(finalized_workflow.metadata().version.len(), 16); // 16-character hex string
    }

    #[test]
    fn test_workflow_version_with_code_fingerprints() {
        init_test_logging();

        let task1 = TestTask::new("task1", vec![]).with_fingerprint("fingerprint1");
        let task2 = TestTask::new("task2", vec!["task1"]).with_fingerprint("fingerprint2");

        let workflow1 = Workflow::builder("test-workflow")
            .description("Test workflow")
            .add_task(task1)
            .unwrap()
            .add_task(task2)
            .unwrap()
            .build()
            .unwrap();

        // Different fingerprint should produce different version
        let task1_diff = TestTask::new("task1", vec![]).with_fingerprint("different_fingerprint");
        let task2_same = TestTask::new("task2", vec!["task1"]).with_fingerprint("fingerprint2");

        let workflow2 = Workflow::builder("test-workflow")
            .description("Test workflow")
            .add_task(task1_diff)
            .unwrap()
            .add_task(task2_same)
            .unwrap()
            .build()
            .unwrap();

        // Versions should be different due to different fingerprints
        assert_ne!(workflow1.metadata().version, workflow2.metadata().version);
    }

    #[test]
    fn test_workflow_add_boxed_task() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        let task: Box<dyn Task> = Box::new(TestTask::new("boxed_task", vec![]));

        // Test adding a boxed task
        assert!(workflow.add_boxed_task(task).is_ok());
        assert!(workflow.get_task("boxed_task").is_some());
    }

    #[test]
    fn test_workflow_add_boxed_task_with_dependencies() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        
        // Add first task
        let task1: Box<dyn Task> = Box::new(TestTask::new("task1", vec![]));
        workflow.add_boxed_task(task1).unwrap();
        
        // Add second task that depends on first
        let task2: Box<dyn Task> = Box::new(TestTask::new("task2", vec!["task1"]));
        workflow.add_boxed_task(task2).unwrap();

        // Verify both tasks are present
        assert!(workflow.get_task("task1").is_some());
        assert!(workflow.get_task("task2").is_some());
        
        // Verify dependencies
        let deps = workflow.get_dependencies("task2").unwrap();
        assert_eq!(deps, &["task1"]);
        
        // Verify topological sort works
        let sorted = workflow.topological_sort().unwrap();
        let pos1 = sorted.iter().position(|x| x == "task1").unwrap();
        let pos2 = sorted.iter().position(|x| x == "task2").unwrap();
        assert!(pos1 < pos2);
    }

    #[test]
    fn test_workflow_add_boxed_task_duplicate_id() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        
        // Add first task
        let task1: Box<dyn Task> = Box::new(TestTask::new("duplicate_id", vec![]));
        workflow.add_boxed_task(task1).unwrap();
        
        // Try to add second task with same ID
        let task2: Box<dyn Task> = Box::new(TestTask::new("duplicate_id", vec![]));
        let result = workflow.add_boxed_task(task2);
        
        // Should fail with duplicate task error
        assert!(matches!(result, Err(WorkflowError::DuplicateTask(_))));
    }

    #[test]
    fn test_workflow_add_boxed_task_vs_regular_task() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        
        // Add regular task
        let regular_task = TestTask::new("regular", vec![]);
        workflow.add_task(regular_task).unwrap();
        
        // Add boxed task
        let boxed_task: Box<dyn Task> = Box::new(TestTask::new("boxed", vec!["regular"]));
        workflow.add_boxed_task(boxed_task).unwrap();
        
        // Both should work together
        assert!(workflow.get_task("regular").is_some());
        assert!(workflow.get_task("boxed").is_some());
        
        // Verify validation passes
        assert!(workflow.validate().is_ok());
        
        // Verify topological sort includes both
        let sorted = workflow.topological_sort().unwrap();
        assert!(sorted.contains(&"regular".to_string()));
        assert!(sorted.contains(&"boxed".to_string()));
    }

    #[test]
    fn test_workflow_add_boxed_task_missing_dependency() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        
        // Add task with non-existent dependency
        let task: Box<dyn Task> = Box::new(TestTask::new("task_with_missing_dep", vec!["nonexistent"]));
        workflow.add_boxed_task(task).unwrap();
        
        // Should be added successfully (dependency checking happens at validation)
        assert!(workflow.get_task("task_with_missing_dep").is_some());
        
        // But validation should fail
        assert!(matches!(
            workflow.validate(),
            Err(ValidationError::MissingDependency { .. })
        ));
    }

    #[test]
    fn test_workflow_get_task_ids() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");
        
        // Add some tasks
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec!["task1"]);
        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();
        
        // Get task IDs
        let task_ids = workflow.get_task_ids();
        assert_eq!(task_ids.len(), 2);
        assert!(task_ids.contains(&"task1".to_string()));
        assert!(task_ids.contains(&"task2".to_string()));
    }

    #[test]
    fn test_workflow_recreate_from_registry() {
        init_test_logging();

        // First, register some tasks in the global registry
        {
            let registry = crate::task::global_task_registry();
            let mut guard = registry.lock().unwrap();
            
            // Clear any existing tasks
            guard.clear();
            
            // Register test task constructors
            guard.insert("test_task_1".to_string(), Box::new(|| {
                Box::new(TestTask::new("test_task_1", vec![]))
            }));
            guard.insert("test_task_2".to_string(), Box::new(|| {
                Box::new(TestTask::new("test_task_2", vec!["test_task_1"]))
            }));
        }
        
        // Create original workflow using the registered tasks
        let mut original_workflow = Workflow::new("test-recreation");
        original_workflow.set_description("Original workflow for testing recreation");
        original_workflow.add_tag("environment", "test");
        
        // Add tasks from registry
        {
            let registry = crate::task::global_task_registry();
            let guard = registry.lock().unwrap();
            
            let task1 = guard.get("test_task_1").unwrap()();
            let task2 = guard.get("test_task_2").unwrap()();
            
            original_workflow.add_boxed_task(task1).unwrap();
            original_workflow.add_boxed_task(task2).unwrap();
        }
        
        let original_workflow = original_workflow.finalize();
        
        // Test recreation
        let recreated_workflow = original_workflow.recreate_from_registry().unwrap();
        
        // Verify the recreation worked
        assert_eq!(original_workflow.name(), recreated_workflow.name());
        assert_eq!(original_workflow.metadata().description, recreated_workflow.metadata().description);
        assert_eq!(original_workflow.metadata().tags, recreated_workflow.metadata().tags);
        
        // Verify tasks were recreated
        let original_task_ids = original_workflow.get_task_ids();
        let recreated_task_ids = recreated_workflow.get_task_ids();
        assert_eq!(original_task_ids.len(), recreated_task_ids.len());
        
        for task_id in &original_task_ids {
            assert!(recreated_task_ids.contains(task_id));
            assert!(recreated_workflow.get_task(task_id).is_some());
        }
        
        // Verify dependencies are preserved
        assert_eq!(
            original_workflow.get_dependencies("test_task_2"),
            recreated_workflow.get_dependencies("test_task_2")
        );
        
        // Verify topological sort is the same
        assert_eq!(
            original_workflow.topological_sort().unwrap(),
            recreated_workflow.topological_sort().unwrap()
        );
    }
}
