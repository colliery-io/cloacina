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

//! In-process test runner for Cloacina tasks.

use crate::result::{TaskOutcome, TestResult};
use cloacina_workflow::Task;
use indexmap::IndexMap;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::{Directed, Graph};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A no-DB, in-process task executor for unit tests.
///
/// Runs tasks sequentially in dependency order without any scheduler,
/// dispatcher, or database. Designed for deterministic task logic testing.
///
/// # Example
///
/// ```rust,ignore
/// use cloacina_testing::TestRunner;
/// use cloacina_workflow::Context;
/// use std::sync::Arc;
///
/// #[tokio::test]
/// async fn test_pipeline() {
///     let result = TestRunner::new()
///         .register(Arc::new(MyTask::new()))
///         .run(Context::new())
///         .await
///         .unwrap();
///
///     result.assert_all_completed();
/// }
/// ```
pub struct TestRunner {
    tasks: IndexMap<String, Arc<dyn Task>>,
}

impl TestRunner {
    /// Create a new empty test runner.
    pub fn new() -> Self {
        Self {
            tasks: IndexMap::new(),
        }
    }

    /// Register a task with the runner. Returns `self` for chaining.
    pub fn register(mut self, task: Arc<dyn Task>) -> Self {
        let id = task.id().to_string();
        self.tasks.insert(id, task);
        self
    }

    /// Execute all registered tasks in topological order.
    ///
    /// Tasks are executed sequentially. Context is propagated from each task
    /// to the next. If a task fails, all its transitive dependents are skipped.
    ///
    /// Returns a [`TestResult`] containing the final context and per-task outcomes.
    ///
    /// # Errors
    ///
    /// Returns an error if the dependency graph contains cycles.
    pub async fn run(
        &self,
        initial_context: cloacina_workflow::Context<serde_json::Value>,
    ) -> Result<TestResult, TestRunnerError> {
        if self.tasks.is_empty() {
            return Ok(TestResult {
                context: initial_context,
                task_outcomes: IndexMap::new(),
            });
        }

        // Build the dependency graph and get topological order
        let execution_order = self.topological_sort()?;

        // Track which tasks have failed (to skip their dependents)
        let mut failed_tasks: HashSet<String> = HashSet::new();
        let mut task_outcomes: IndexMap<String, TaskOutcome> = IndexMap::new();
        let mut context = initial_context;

        for task_id in &execution_order {
            let task = self.tasks.get(task_id).unwrap();

            // Check if any dependency has failed
            let should_skip = task.dependencies().iter().any(|dep| {
                // Match by task_id field of the namespace
                failed_tasks.contains(&dep.task_id)
            });

            if should_skip {
                failed_tasks.insert(task_id.clone());
                task_outcomes.insert(task_id.clone(), TaskOutcome::Skipped);
                continue;
            }

            // Execute the task
            match task.execute(context.clone_data()).await {
                Ok(new_context) => {
                    context = new_context;
                    task_outcomes.insert(task_id.clone(), TaskOutcome::Completed);
                }
                Err(e) => {
                    failed_tasks.insert(task_id.clone());
                    task_outcomes.insert(task_id.clone(), TaskOutcome::Failed(e));
                }
            }
        }

        Ok(TestResult {
            context,
            task_outcomes,
        })
    }

    /// Build a petgraph from registered tasks and return topological order.
    fn topological_sort(&self) -> Result<Vec<String>, TestRunnerError> {
        let mut graph = Graph::<String, (), Directed>::new();
        let mut node_indices = HashMap::new();

        // Add all registered tasks as nodes
        for task_id in self.tasks.keys() {
            let idx = graph.add_node(task_id.clone());
            node_indices.insert(task_id.clone(), idx);
        }

        // Add edges from dependencies
        for (task_id, task) in &self.tasks {
            for dep in task.dependencies() {
                // Match dependency by task_id field of the namespace
                let dep_id = &dep.task_id;
                if let Some(&dep_idx) = node_indices.get(dep_id) {
                    let task_idx = node_indices[task_id];
                    // Edge from dependency to dependent (dep must run first)
                    graph.add_edge(dep_idx, task_idx, ());
                }
                // If dependency is not registered, silently skip it.
                // This allows testing subsets of a workflow.
            }
        }

        // Check for cycles
        if is_cyclic_directed(&graph) {
            let cycle = self.find_cycle();
            return Err(TestRunnerError::CyclicDependency { cycle });
        }

        // Topological sort
        match toposort(&graph, None) {
            Ok(sorted) => Ok(sorted.into_iter().map(|idx| graph[idx].clone()).collect()),
            Err(_) => {
                let cycle = self.find_cycle();
                Err(TestRunnerError::CyclicDependency { cycle })
            }
        }
    }

    /// Find a cycle in the dependency graph (for error reporting).
    fn find_cycle(&self) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for task_id in self.tasks.keys() {
            if !visited.contains(task_id) {
                if let Some(cycle) =
                    self.dfs_cycle(task_id, &mut visited, &mut rec_stack, &mut path)
                {
                    return cycle;
                }
            }
        }
        vec![]
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

        if let Some(task) = self.tasks.get(node) {
            for dep in task.dependencies() {
                let dep_id = &dep.task_id;
                if !visited.contains(dep_id.as_str()) {
                    if let Some(cycle) = self.dfs_cycle(dep_id, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep_id.as_str()) {
                    let cycle_start = path.iter().position(|x| x == dep_id).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep_id.clone());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when running the test runner.
#[derive(Debug, thiserror::Error)]
pub enum TestRunnerError {
    /// The dependency graph contains a cycle.
    #[error("dependency cycle detected: {cycle:?}")]
    CyclicDependency { cycle: Vec<String> },
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cloacina_workflow::{Context, TaskError, TaskNamespace};
    use serde_json::json;

    // --- Test task implementations ---

    /// A task that inserts a key into the context.
    struct PassTask {
        id: String,
        deps: Vec<TaskNamespace>,
        key: String,
        value: serde_json::Value,
    }

    impl PassTask {
        fn new(id: &str, key: &str, value: serde_json::Value) -> Self {
            Self {
                id: id.to_string(),
                deps: vec![],
                key: key.to_string(),
                value,
            }
        }

        fn with_dep(mut self, dep_id: &str) -> Self {
            self.deps
                .push(TaskNamespace::new("public", "embedded", "test", dep_id));
            self
        }
    }

    #[async_trait]
    impl Task for PassTask {
        async fn execute(
            &self,
            mut context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            let _ = context.insert(&self.key, self.value.clone());
            Ok(context)
        }
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[TaskNamespace] {
            &self.deps
        }
    }

    /// A task that always fails.
    struct FailTask {
        id: String,
        deps: Vec<TaskNamespace>,
        message: String,
    }

    impl FailTask {
        fn new(id: &str, message: &str) -> Self {
            Self {
                id: id.to_string(),
                deps: vec![],
                message: message.to_string(),
            }
        }

        fn with_dep(mut self, dep_id: &str) -> Self {
            self.deps
                .push(TaskNamespace::new("public", "embedded", "test", dep_id));
            self
        }
    }

    #[async_trait]
    impl Task for FailTask {
        async fn execute(
            &self,
            _context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            Err(TaskError::ExecutionFailed {
                message: self.message.clone(),
                task_id: self.id.clone(),
                timestamp: chrono::Utc::now(),
            })
        }
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[TaskNamespace] {
            &self.deps
        }
    }

    /// A task that checks a key exists in context.
    struct ContextCheckTask {
        id: String,
        deps: Vec<TaskNamespace>,
        expected_key: String,
    }

    impl ContextCheckTask {
        fn new(id: &str, expected_key: &str) -> Self {
            Self {
                id: id.to_string(),
                deps: vec![],
                expected_key: expected_key.to_string(),
            }
        }

        fn with_dep(mut self, dep_id: &str) -> Self {
            self.deps
                .push(TaskNamespace::new("public", "embedded", "test", dep_id));
            self
        }
    }

    #[async_trait]
    impl Task for ContextCheckTask {
        async fn execute(
            &self,
            mut context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            if context.get(&self.expected_key).is_none() {
                return Err(TaskError::ExecutionFailed {
                    message: format!("expected key '{}' not found in context", self.expected_key),
                    task_id: self.id.clone(),
                    timestamp: chrono::Utc::now(),
                });
            }
            let _ = context.insert(format!("{}_checked", self.expected_key), json!(true));
            Ok(context)
        }
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[TaskNamespace] {
            &self.deps
        }
    }

    // --- Tests ---

    #[tokio::test]
    async fn test_single_task_completes() {
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("task_a", "result", json!("hello"))))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_all_completed();
        assert_eq!(result.context.get("result"), Some(&json!("hello")));
    }

    #[tokio::test]
    async fn test_multiple_independent_tasks() {
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("a", "key_a", json!(1))))
            .register(Arc::new(PassTask::new("b", "key_b", json!(2))))
            .register(Arc::new(PassTask::new("c", "key_c", json!(3))))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_all_completed();
        assert_eq!(result.task_outcomes.len(), 3);
    }

    #[tokio::test]
    async fn test_linear_dependency_chain() {
        // A -> B -> C
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("a", "step_a", json!("done"))))
            .register(Arc::new(
                PassTask::new("b", "step_b", json!("done")).with_dep("a"),
            ))
            .register(Arc::new(
                PassTask::new("c", "step_c", json!("done")).with_dep("b"),
            ))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_all_completed();
        assert!(result.context.get("step_a").is_some());
        assert!(result.context.get("step_b").is_some());
        assert!(result.context.get("step_c").is_some());
    }

    #[tokio::test]
    async fn test_diamond_dependency() {
        // A -> B, A -> C, B+C -> D
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("a", "val_a", json!(1))))
            .register(Arc::new(
                PassTask::new("b", "val_b", json!(2)).with_dep("a"),
            ))
            .register(Arc::new(
                PassTask::new("c", "val_c", json!(3)).with_dep("a"),
            ))
            .register(Arc::new(
                PassTask::new("d", "val_d", json!(4))
                    .with_dep("b")
                    .with_dep("c"),
            ))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_all_completed();
        assert_eq!(result.task_outcomes.len(), 4);
    }

    #[tokio::test]
    async fn test_task_failure_skips_dependents() {
        // A (fails) -> B -> C
        let result = TestRunner::new()
            .register(Arc::new(FailTask::new("a", "boom")))
            .register(Arc::new(
                PassTask::new("b", "val_b", json!(1)).with_dep("a"),
            ))
            .register(Arc::new(
                PassTask::new("c", "val_c", json!(2)).with_dep("b"),
            ))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_task_failed("a");
        result.assert_task_skipped("b");
        result.assert_task_skipped("c");
    }

    #[tokio::test]
    async fn test_partial_failure_independent_branches_continue() {
        // A (fails) -> B (skipped)
        // C (independent, succeeds)
        let result = TestRunner::new()
            .register(Arc::new(FailTask::new("a", "error")))
            .register(Arc::new(
                PassTask::new("b", "val_b", json!(1)).with_dep("a"),
            ))
            .register(Arc::new(PassTask::new("c", "val_c", json!(2))))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_task_failed("a");
        result.assert_task_skipped("b");
        result.assert_task_completed("c");
    }

    #[tokio::test]
    async fn test_cycle_detection() {
        // A depends on B, B depends on A
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("a", "v", json!(1)).with_dep("b")))
            .register(Arc::new(PassTask::new("b", "v", json!(2)).with_dep("a")))
            .run(Context::new())
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, TestRunnerError::CyclicDependency { .. }),
            "expected CyclicDependency error"
        );
    }

    #[tokio::test]
    async fn test_empty_runner() {
        let result = TestRunner::new().run(Context::new()).await.unwrap();

        assert!(result.task_outcomes.is_empty());
    }

    #[tokio::test]
    async fn test_context_propagation() {
        // A inserts "data", B checks "data" exists
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("producer", "data", json!(42))))
            .register(Arc::new(
                ContextCheckTask::new("consumer", "data").with_dep("producer"),
            ))
            .run(Context::new())
            .await
            .unwrap();

        result.assert_all_completed();
        assert_eq!(result.context.get("data"), Some(&json!(42)));
        assert_eq!(result.context.get("data_checked"), Some(&json!(true)));
    }

    #[tokio::test]
    async fn test_index_access() {
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("my_task", "k", json!(1))))
            .run(Context::new())
            .await
            .unwrap();

        assert!(result["my_task"].is_completed());
    }

    #[tokio::test]
    #[should_panic(expected = "task 'nonexistent' not found")]
    async fn test_index_missing_task_panics() {
        let result = TestRunner::new()
            .register(Arc::new(PassTask::new("a", "k", json!(1))))
            .run(Context::new())
            .await
            .unwrap();

        let _ = &result["nonexistent"];
    }
}
