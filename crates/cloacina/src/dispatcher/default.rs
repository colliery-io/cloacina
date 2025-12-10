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

//! Default dispatcher implementation.
//!
//! Provides the standard dispatcher that routes tasks to executors based on
//! configurable glob patterns.

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::router::Router;
use super::traits::{Dispatcher, TaskExecutor};
use super::types::{DispatchError, ExecutionStatus, RoutingConfig, TaskReadyEvent};
use crate::dal::DAL;

/// Default dispatcher implementation with glob-based routing.
///
/// The DefaultDispatcher maintains a registry of executor backends and routes
/// tasks based on pattern matching rules. It handles the full dispatch lifecycle
/// including state transitions and result handling.
///
/// # Example
///
/// ```rust,ignore
/// use cloacina::dispatcher::{DefaultDispatcher, RoutingConfig, RoutingRule};
///
/// let config = RoutingConfig::new("default")
///     .with_rule(RoutingRule::new("ml::*", "gpu"))
///     .with_rule(RoutingRule::new("heavy_*", "high_memory"));
///
/// let dispatcher = DefaultDispatcher::new(dal, config);
/// dispatcher.register_executor("default", Arc::new(thread_executor));
/// dispatcher.register_executor("gpu", Arc::new(gpu_executor));
/// ```
pub struct DefaultDispatcher {
    /// Registered executor backends
    executors: RwLock<HashMap<String, Arc<dyn TaskExecutor>>>,
    /// Routing logic
    router: Router,
    /// Data access layer for state updates
    dal: DAL,
}

impl DefaultDispatcher {
    /// Creates a new DefaultDispatcher with the given DAL and routing configuration.
    pub fn new(dal: DAL, routing: RoutingConfig) -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
            router: Router::new(routing),
            dal,
        }
    }

    /// Creates a dispatcher with default routing (all tasks go to "default" executor).
    pub fn with_defaults(dal: DAL) -> Self {
        Self::new(dal, RoutingConfig::default())
    }

    /// Gets a reference to the router for inspection.
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Gets a reference to the DAL.
    pub fn dal(&self) -> &DAL {
        &self.dal
    }

    /// Handles the execution result by updating database state.
    async fn handle_result(
        &self,
        event: &TaskReadyEvent,
        result: super::types::ExecutionResult,
    ) -> Result<(), DispatchError> {
        match result.status {
            ExecutionStatus::Completed => {
                self.dal
                    .task_execution()
                    .mark_completed(event.task_execution_id)
                    .await?;
                info!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    duration_ms = result.duration.as_millis(),
                    "Task completed successfully"
                );
            }
            ExecutionStatus::Failed => {
                let error_msg = result.error.as_deref().unwrap_or("Unknown error");
                self.dal
                    .task_execution()
                    .mark_failed(event.task_execution_id, error_msg)
                    .await?;
                warn!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    error = error_msg,
                    duration_ms = result.duration.as_millis(),
                    "Task failed"
                );
            }
            ExecutionStatus::Retry => {
                // Retry handling is done by the executor - it schedules the retry
                debug!(
                    task_id = %event.task_execution_id,
                    task_name = %event.task_name,
                    "Task will be retried"
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Dispatcher for DefaultDispatcher {
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
        let executor_key = self.router.resolve(&event.task_name);

        let executor = {
            let executors = self.executors.read();
            executors
                .get(executor_key)
                .cloned()
                .ok_or_else(|| DispatchError::ExecutorNotFound(executor_key.to_string()))?
        };

        if !executor.has_capacity() {
            return Err(DispatchError::NoCapacity(executor_key.to_string()));
        }

        debug!(
            task_id = %event.task_execution_id,
            task_name = %event.task_name,
            executor = executor_key,
            "Dispatching task to executor"
        );

        // Execute the task - the executor is responsible for claiming (which marks as Running)
        // and handling execution. We handle the final state transition.
        let result = executor.execute(event.clone()).await?;
        self.handle_result(&event, result).await?;

        Ok(())
    }

    fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>) {
        let mut executors = self.executors.write();
        info!(
            executor_key = key,
            executor_name = executor.name(),
            "Registered executor"
        );
        executors.insert(key.to_string(), executor);
    }

    fn has_capacity(&self) -> bool {
        let executors = self.executors.read();
        executors.values().any(|e| e.has_capacity())
    }

    fn resolve_executor_key(&self, task_name: &str) -> String {
        self.router.resolve(task_name).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::UniversalUuid;
    use crate::dispatcher::types::{ExecutionResult, ExecutorMetrics};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    /// Mock executor for testing
    struct MockExecutor {
        name: String,
        has_capacity: AtomicBool,
        execute_count: AtomicUsize,
    }

    impl MockExecutor {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                has_capacity: AtomicBool::new(true),
                execute_count: AtomicUsize::new(0),
            }
        }

        #[allow(dead_code)]
        fn execution_count(&self) -> usize {
            self.execute_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl TaskExecutor for MockExecutor {
        async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
            self.execute_count.fetch_add(1, Ordering::SeqCst);
            Ok(ExecutionResult::success(
                event.task_execution_id,
                Duration::from_millis(100),
            ))
        }

        fn has_capacity(&self) -> bool {
            self.has_capacity.load(Ordering::SeqCst)
        }

        fn metrics(&self) -> ExecutorMetrics {
            ExecutorMetrics {
                active_tasks: 0,
                max_concurrent: 4,
                total_executed: self.execute_count.load(Ordering::SeqCst) as u64,
                total_failed: 0,
                avg_duration_ms: 100,
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[allow(dead_code)]
    fn create_test_event(task_name: &str) -> TaskReadyEvent {
        TaskReadyEvent::new(
            UniversalUuid::new_v4(),
            UniversalUuid::new_v4(),
            task_name.to_string(),
            1,
        )
    }

    #[test]
    fn test_register_executor() {
        // We can't easily test dispatch without a real DAL, but we can test registration
        let _config = RoutingConfig::default();

        // Just verify the types work together
        let _executor: Arc<dyn TaskExecutor> = Arc::new(MockExecutor::new("test"));
    }

    #[test]
    fn test_resolve_executor_key() {
        // Create a mock DAL-less test by just testing routing
        let config = RoutingConfig::new("default")
            .with_rule(super::super::types::RoutingRule::new("ml::*", "gpu"));
        let router = Router::new(config);

        assert_eq!(router.resolve("ml::train"), "gpu");
        assert_eq!(router.resolve("etl::extract"), "default");
    }
}
