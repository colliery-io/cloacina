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
//! ```rust,ignore
//! pub struct Workflow {
//!     name: String,
//!     tasks: HashMap<String, Arc<dyn Task>>,
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

mod builder;
mod graph;
mod metadata;
mod registry;

// Re-export public types
pub use builder::WorkflowBuilder;
pub use graph::DependencyGraph;
pub use metadata::WorkflowMetadata;
pub use registry::{
    get_all_workflows, global_workflow_registry, register_workflow_constructor,
    GlobalWorkflowRegistry, WorkflowConstructor, GLOBAL_WORKFLOW_REGISTRY,
};

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::{SubgraphError, ValidationError, WorkflowError};
use crate::task::{Task, TaskNamespace};

/// Main Workflow structure for representing and managing task graphs.
///
/// A Workflow contains a collection of tasks with their dependency relationships,
/// ensuring that the graph remains acyclic and provides methods for execution
/// planning and analysis.
///
/// # Fields
///
/// * `name`: String - Unique identifier for the workflow
/// * `tenant`: String - Unique identifier for the tenant
/// * `tasks`: HashMap<TaskNamespace, Arc<dyn Task>> - Map of task namespaces to task implementations
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
/// ```rust,ignore
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
#[derive(Clone)]
pub struct Workflow {
    name: String,
    pub(crate) tenant: String,
    package: String,
    tasks: HashMap<TaskNamespace, Arc<dyn Task>>,
    dependency_graph: DependencyGraph,
    metadata: WorkflowMetadata,
}

impl std::fmt::Debug for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workflow")
            .field("name", &self.name)
            .field("tenant", &self.tenant)
            .field("package", &self.package)
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
    /// ```rust,ignore
    /// use cloacina::Workflow;
    ///
    /// let workflow = Workflow::new("my_workflow");
    /// assert_eq!(workflow.name(), "my_workflow");
    /// ```
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

    /// Create a Workflow builder for programmatic construction
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the workflow
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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

    /// Get the Workflow tenant
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Set the Workflow tenant
    pub fn set_tenant(&mut self, tenant: &str) {
        self.tenant = tenant.to_string();
    }

    /// Get the Workflow package
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Set the Workflow package
    pub fn set_package(&mut self, package: &str) {
        self.package = package.to_string();
    }

    /// Get the Workflow metadata
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// ```rust,ignore
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

    /// Remove a tag from the workflow metadata
    ///
    /// # Arguments
    /// * `key` - Tag key to remove
    ///
    /// # Returns
    /// * `Some(String)` - The removed tag value if it existed
    /// * `None` - If no tag with that key existed
    ///
    /// # Examples
    /// ```
    /// use cloacina::Workflow;
    ///
    /// let mut workflow = Workflow::new("test-workflow");
    /// workflow.add_tag("environment", "staging");
    ///
    /// let removed = workflow.remove_tag("environment");
    /// assert_eq!(removed, Some("staging".to_string()));
    /// ```
    pub fn remove_tag(&mut self, key: &str) -> Option<String> {
        self.metadata.tags.remove(key)
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
    /// ```rust,ignore
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

    /// Remove a task from the workflow
    ///
    /// This removes the task and all its dependencies from the workflow.
    /// Returns the removed task if it existed.
    ///
    /// # Arguments
    /// * `task_id` - ID of the task to remove
    ///
    /// # Returns
    /// * `Some(Arc<dyn Task>)` - The removed task if it existed
    /// * `None` - If no task with that ID existed
    ///
    /// # Examples
    /// ```ignore
    /// use cloacina::*;
    /// use std::sync::Arc;
    ///
    /// let mut workflow = Workflow::new("test-workflow");
    /// let task = Arc::new(MockTask::new("task1", vec![]));
    /// workflow.add_task(task.clone())?;
    ///
    /// let removed = workflow.remove_task("task1");
    /// assert!(removed.is_some());
    /// assert!(workflow.get_task("task1").is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_task(&mut self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        // Remove from dependency graph first
        self.dependency_graph.remove_node(namespace);

        // Remove and return the task
        self.tasks.remove(namespace)
    }

    /// Remove a dependency between two tasks
    ///
    /// This removes the dependency edge but keeps both tasks in the workflow.
    ///
    /// # Arguments
    /// * `from_task` - Task that currently depends on `to_task`
    /// * `to_task` - Task that `from_task` currently depends on
    ///
    /// # Examples
    /// ```ignore
    /// use cloacina::*;
    /// use std::sync::Arc;
    ///
    /// let mut workflow = Workflow::new("test-workflow");
    /// // Add tasks with dependency: task2 depends on task1
    /// workflow.add_task(Arc::new(MockTask::new("task1", vec![])))?;
    /// workflow.add_task(Arc::new(MockTask::new("task2", vec!["task1"])))?;
    ///
    /// // Remove the dependency (task2 no longer depends on task1)
    /// workflow.remove_dependency("task2", "task1");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_dependency(&mut self, from_task: &TaskNamespace, to_task: &TaskNamespace) {
        self.dependency_graph.remove_edge(from_task, to_task);
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
    /// ```rust,ignore
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
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let execution_order = workflow.topological_sort()?;
    /// println!("Execute tasks in order: {:?}", execution_order);
    /// # Ok::<(), ValidationError>(())
    /// ```
    pub fn topological_sort(&self) -> Result<Vec<TaskNamespace>, ValidationError> {
        self.validate()?;
        self.dependency_graph.topological_sort()
    }

    /// Get a task by namespace
    ///
    /// # Arguments
    ///
    /// * `namespace` - Task namespace to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn Task>)` - If the task exists
    /// * `Err(WorkflowError)` - If no task with that namespace exists
    pub fn get_task(&self, namespace: &TaskNamespace) -> Result<Arc<dyn Task>, WorkflowError> {
        self.tasks
            .get(namespace)
            .cloned()
            .ok_or_else(|| WorkflowError::TaskNotFound(namespace.to_string()))
    }

    /// Get dependencies for a task
    ///
    /// # Arguments
    ///
    /// * `namespace` - Task namespace to get dependencies for
    ///
    /// # Returns
    ///
    /// * `Ok(&[TaskNamespace])` - Array of dependency task namespaces
    /// * `Err(WorkflowError)` - If the task doesn't exist
    pub fn get_dependencies(
        &self,
        namespace: &TaskNamespace,
    ) -> Result<&[TaskNamespace], WorkflowError> {
        self.tasks
            .get(namespace)
            .map(|task| task.dependencies())
            .ok_or_else(|| WorkflowError::TaskNotFound(namespace.to_string()))
    }

    /// Get dependents of a task
    ///
    /// Returns tasks that depend on the given task.
    ///
    /// # Arguments
    ///
    /// * `namespace` - Task namespace to get dependents for
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<TaskNamespace>)` - Vector of task namespaces that depend on the given task
    /// * `Err(WorkflowError)` - If the task doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let namespace = TaskNamespace::new("public", "embedded", "test", "extract_data");
    /// let dependents = workflow.get_dependents(&namespace)?;
    /// println!("Tasks depending on extract_data: {:?}", dependents);
    /// # Ok::<(), WorkflowError>(())
    /// ```
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
    /// ```rust,ignore
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

    /// Get root tasks (tasks with no dependencies)
    ///
    /// # Returns
    ///
    /// Vector of task IDs that have no dependencies
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let roots = workflow.get_roots();
    /// println!("Starting tasks: {:?}", roots);
    /// ```
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

    /// Get leaf tasks (tasks with no dependents)
    ///
    /// # Returns
    ///
    /// Vector of task IDs that no other tasks depend on
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let leaves = workflow.get_leaves();
    /// println!("Final tasks: {:?}", leaves);
    /// ```
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
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// if workflow.can_run_parallel("fetch_users", "fetch_orders") {
    ///     println!("These tasks can run simultaneously");
    /// }
    /// ```
    pub fn can_run_parallel(&self, task_a: &TaskNamespace, task_b: &TaskNamespace) -> bool {
        // Tasks can run in parallel if neither depends on the other
        !self.has_path(task_a, task_b) && !self.has_path(task_b, task_a)
    }

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
    /// ```rust,ignore
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
        self.tenant.hash(hasher);
        self.metadata.description.hash(hasher);

        // Hash tags in deterministic order
        let mut tags: Vec<_> = self.metadata.tags.iter().collect();
        tags.sort_by_key(|(k, _)| *k);
        tags.hash(hasher);
    }

    fn get_task_code_hash(&self, task_namespace: &TaskNamespace) -> Option<String> {
        self.tasks
            .get(task_namespace)
            .and_then(|task| task.code_fingerprint())
    }

    /// Get all task namespaces in the workflow
    ///
    /// # Returns
    ///
    /// Vector of all task namespaces currently in the workflow
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::*;
    /// # let workflow = Workflow::new("test");
    /// let task_namespaces = workflow.get_task_ids();
    /// println!("Tasks in workflow: {:?}", task_namespaces);
    /// ```
    pub fn get_task_ids(&self) -> Vec<TaskNamespace> {
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
        dependencies: Vec<TaskNamespace>,
        fingerprint: Option<String>,
    }

    impl TestTask {
        fn new(id: &str, dependencies: Vec<TaskNamespace>) -> Self {
            Self {
                id: id.to_string(),
                dependencies,
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

        fn dependencies(&self) -> &[TaskNamespace] {
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
        let task_namespace = TaskNamespace::new("public", "embedded", "test-workflow", "task1");

        assert!(workflow.add_task(Arc::new(task)).is_ok());
        assert!(workflow.get_task(&task_namespace).is_ok());
    }

    #[test]
    fn test_workflow_validation() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1]);

        workflow.add_task(Arc::new(task1)).unwrap();
        workflow.add_task(Arc::new(task2)).unwrap();

        assert!(workflow.validate().is_ok());
    }

    #[test]
    fn test_workflow_cycle_detection() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test-workflow", "task2");
        let task1 = TestTask::new("task1", vec![ns2]);
        let task2 = TestTask::new("task2", vec![ns1]);

        workflow.add_task(Arc::new(task1)).unwrap();
        workflow.add_task(Arc::new(task2)).unwrap();

        assert!(matches!(
            workflow.validate(),
            Err(ValidationError::CyclicDependency { .. })
        ));
    }

    #[test]
    fn test_workflow_topological_sort() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test-workflow", "task2");
        let _ns3 = TaskNamespace::new("public", "embedded", "test-workflow", "task3");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1.clone()]);
        let task3 = TestTask::new("task3", vec![ns1.clone(), ns2.clone()]);

        workflow.add_task(Arc::new(task1)).unwrap();
        workflow.add_task(Arc::new(task2)).unwrap();
        workflow.add_task(Arc::new(task3)).unwrap();

        let sorted = workflow.topological_sort().unwrap();

        let pos1 = sorted.iter().position(|x| x.task_id == "task1").unwrap();
        let pos2 = sorted.iter().position(|x| x.task_id == "task2").unwrap();
        let pos3 = sorted.iter().position(|x| x.task_id == "task3").unwrap();

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_workflow_builder_auto_versioning() {
        init_test_logging();

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1]);

        let workflow = Workflow::builder("test-workflow")
            .description("Test Workflow with auto-versioning")
            .tag("env", "test")
            .add_task(Arc::new(task1))
            .unwrap()
            .add_task(Arc::new(task2))
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

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test-workflow", "task2");
        let ns3 = TaskNamespace::new("public", "embedded", "test-workflow", "task3");
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![]);
        let task3 = TestTask::new("task3", vec![ns1.clone(), ns2.clone()]);
        let task4 = TestTask::new("task4", vec![ns3]);

        workflow.add_task(Arc::new(task1)).unwrap();
        workflow.add_task(Arc::new(task2)).unwrap();
        workflow.add_task(Arc::new(task3)).unwrap();
        workflow.add_task(Arc::new(task4)).unwrap();

        let levels = workflow.get_execution_levels().unwrap();

        // Level 0: task1, task2 (no dependencies)
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&ns1));
        assert!(levels[0].contains(&ns2));

        // Level 1: task3 (depends on task1, task2)
        assert_eq!(levels[1].len(), 1);
        let expected_ns3 = TaskNamespace::new("public", "embedded", "test-workflow", "task3");
        assert!(levels[1].contains(&expected_ns3));

        // Level 2: task4 (depends on task3)
        assert_eq!(levels[2].len(), 1);
        let expected_ns4 = TaskNamespace::new("public", "embedded", "test-workflow", "task4");
        assert!(levels[2].contains(&expected_ns4));
    }

    #[test]
    fn test_workflow_version_consistency() {
        init_test_logging();

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1]);

        // Build same Workflow twice
        let workflow1 = Workflow::builder("test-workflow")
            .description("Test Workflow")
            .add_task(Arc::new(task1))
            .unwrap()
            .add_task(Arc::new(task2))
            .unwrap()
            .build()
            .unwrap();

        let ns1_copy = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1_copy = TestTask::new("task1", vec![]);
        let task2_copy = TestTask::new("task2", vec![ns1_copy]);

        let workflow2 = Workflow::builder("test-workflow")
            .description("Test Workflow")
            .add_task(Arc::new(task1_copy))
            .unwrap()
            .add_task(Arc::new(task2_copy))
            .unwrap()
            .build()
            .unwrap();

        // Same content should produce same version
        assert_eq!(workflow1.metadata().version, workflow2.metadata().version);
    }

    #[test]
    fn test_workflow_version_changes() {
        init_test_logging();

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1]);

        let workflow1 = Workflow::builder("test-workflow")
            .description("Original description")
            .add_task(Arc::new(task1))
            .unwrap()
            .add_task(Arc::new(task2))
            .unwrap()
            .build()
            .unwrap();

        let ns1_copy = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1_copy = TestTask::new("task1", vec![]);
        let task2_copy = TestTask::new("task2", vec![ns1_copy]);

        let workflow2 = Workflow::builder("test-workflow")
            .description("Changed description") // Different description
            .add_task(Arc::new(task1_copy))
            .unwrap()
            .add_task(Arc::new(task2_copy))
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
        workflow.add_task(Arc::new(task1)).unwrap();

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

        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = TestTask::new("task1", vec![]).with_fingerprint("fingerprint1");
        let task2 = TestTask::new("task2", vec![ns1]).with_fingerprint("fingerprint2");

        let workflow1 = Workflow::builder("test-workflow")
            .description("Test workflow")
            .add_task(Arc::new(task1))
            .unwrap()
            .add_task(Arc::new(task2))
            .unwrap()
            .build()
            .unwrap();

        // Different fingerprint should produce different version
        let ns1_diff = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1_diff = TestTask::new("task1", vec![]).with_fingerprint("different_fingerprint");
        let task2_same = TestTask::new("task2", vec![ns1_diff]).with_fingerprint("fingerprint2");

        let workflow2 = Workflow::builder("test-workflow")
            .description("Test workflow")
            .add_task(Arc::new(task1_diff))
            .unwrap()
            .add_task(Arc::new(task2_same))
            .unwrap()
            .build()
            .unwrap();

        // Versions should be different due to different fingerprints
        assert_ne!(workflow1.metadata().version, workflow2.metadata().version);
    }

    #[test]
    fn test_workflow_removal_methods() {
        init_test_logging();

        let mut workflow = Workflow::new("test-workflow");

        // Add tasks
        let ns1 = TaskNamespace::new("public", "embedded", "test-workflow", "task1");
        let task1 = Arc::new(TestTask::new("task1", vec![]));
        let task2 = Arc::new(TestTask::new("task2", vec![ns1.clone()]));
        workflow.add_task(task1).unwrap();
        workflow.add_task(task2).unwrap();

        // Add tags
        workflow.add_tag("env", "test");
        workflow.add_tag("team", "eng");

        // Test task removal
        assert!(workflow.get_task(&ns1).is_ok());
        let removed_task = workflow.remove_task(&ns1);
        assert!(removed_task.is_some());
        assert!(workflow.get_task(&ns1).is_err());

        // Test tag removal
        assert_eq!(
            workflow.metadata().tags.get("env"),
            Some(&"test".to_string())
        );
        let removed_tag = workflow.remove_tag("env");
        assert_eq!(removed_tag, Some("test".to_string()));
        assert!(workflow.metadata().tags.get("env").is_none());

        // Test dependency removal (task2 should still exist but with no deps)
        let ns2 = TaskNamespace::new("public", "embedded", "test-workflow", "task2");
        workflow.remove_dependency(&ns2, &ns1);
        // We can't easily test this without exposing dependency graph methods
        // but it should not panic
    }
}
