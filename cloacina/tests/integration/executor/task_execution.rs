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

use async_trait::async_trait;
use cloacina::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::fixtures::get_or_init_fixture;

// Helper for getting database for tests
async fn get_test_database() -> Database {
    let fixture = get_or_init_fixture().await;
    let mut locked_fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    locked_fixture.initialize().await;
    locked_fixture.get_database()
}

// Simple task for workflow construction
#[derive(Debug)]
struct WorkflowTask {
    id: String,
    dependencies: Vec<String>,
}

impl WorkflowTask {
    fn new(id: &str, deps: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait]
impl Task for WorkflowTask {
    async fn execute(&self, context: Context<Value>) -> Result<Context<Value>, TaskError> {
        Ok(context) // No-op for workflow building
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

#[derive(Debug)]
struct SimpleExecutorTask {
    id: String,
    dependencies: Vec<String>,
    output_key: String,
    output_value: Value,
}

impl SimpleExecutorTask {
    fn new(id: &str, deps: Vec<&str>, output_key: &str, output_value: Value) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            output_key: output_key.to_string(),
            output_value,
        }
    }
}

#[async_trait]
impl Task for SimpleExecutorTask {
    async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
        // Add our output to the context
        context
            .insert(&self.output_key, self.output_value.clone())
            .map_err(|e| TaskError::Unknown {
                task_id: self.id.clone(),
                message: format!("Failed to insert output: {}", e),
            })?;

        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

#[derive(Debug)]
struct DependencyConsumerTask {
    id: String,
    dependencies: Vec<String>,
    dependency_key: String,
}

impl DependencyConsumerTask {
    fn new(id: &str, deps: Vec<&str>, dependency_key: &str) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            dependency_key: dependency_key.to_string(),
        }
    }
}

#[async_trait]
impl Task for DependencyConsumerTask {
    async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
        // Try to load dependency value using lazy loading
        match context
            .load_from_dependencies_and_cache(&self.dependency_key)
            .await
        {
            Ok(Some(value)) => {
                // Add a derived value to show dependency was loaded
                context
                    .insert(
                        &format!("derived_from_{}", self.dependency_key),
                        Value::String(format!("Processed: {}", value)),
                    )
                    .map_err(|e| TaskError::Unknown {
                        task_id: self.id.clone(),
                        message: format!("Failed to insert derived value: {}", e),
                    })?;
            }
            Ok(None) => {
                return Err(TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: format!("Dependency key '{}' not found", self.dependency_key),
                });
            }
            Err(e) => {
                return Err(TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: format!("Failed to load dependency: {}", e),
                });
            }
        }

        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

#[tokio::test]
async fn test_task_executor_basic_execution() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    // Reset the database to ensure a clean state
    fixture.reset_database().await;

    let database = fixture.get_database();

    // Create task registry
    let mut task_registry = TaskRegistry::new();
    let task = SimpleExecutorTask::new(
        "test_task",
        vec![],
        "result",
        Value::String("success".to_string()),
    );
    task_registry.register(task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create scheduler and schedule a workflow
    let workflow = Workflow::builder("test_pipeline")
        .description("Test pipeline for executor")
        .add_task(WorkflowTask::new("test_task", vec![]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("test_data", Value::String("test_value".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("test_pipeline", input_context)
        .await
        .unwrap();

    // Process scheduling to mark task as ready
    scheduler.process_active_pipelines().await.unwrap();

    // Create and run executor briefly
    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);

    // Run executor for a short time to process the task
    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give the executor time to process the task
    time::sleep(Duration::from_millis(500)).await;

    // Check that task was executed
    let dal = cloacina::dal::DAL::new(database.pool());
    let task_executions = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
        .await
        .unwrap();

    // Verify task execution
    assert_eq!(task_executions.len(), 1);
    let task = &task_executions[0];
    assert_eq!(task.status, "Completed");
    assert_eq!(task.task_name, "test_task");

    // Clean up
    executor_handle.abort();
}

#[tokio::test]
async fn test_task_executor_dependency_loading() {
    let database = get_test_database().await;

    // Create task registry with dependency chain
    let mut task_registry = TaskRegistry::new();

    let producer_task = SimpleExecutorTask::new(
        "producer",
        vec![],
        "shared_data",
        Value::String("important_value".to_string()),
    );
    let consumer_task = DependencyConsumerTask::new("consumer", vec!["producer"], "shared_data");

    task_registry.register(producer_task).unwrap();
    task_registry.register(consumer_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow with dependencies
    let workflow = Workflow::builder("dependency_pipeline")
        .description("Test pipeline with dependencies")
        .add_task(WorkflowTask::new("producer", vec![]))
        .unwrap()
        .add_task(WorkflowTask::new("consumer", vec!["producer"]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("initial_data", Value::String("test_value".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("dependency_pipeline", input_context)
        .await
        .unwrap();

    // Create and run executor
    let config = ExecutorConfig {
        max_concurrent_tasks: 2,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);

    // Run scheduling and execution loop
    let scheduler_handle = tokio::spawn(async move { scheduler.run_scheduling_loop().await });

    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for both tasks to execute
    time::sleep(Duration::from_secs(2)).await;

    // Check that consumer task successfully loaded dependency data
    let dal = cloacina::dal::DAL::new(database.pool());
    let consumer_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "consumer")
        .await
        .unwrap();

    // Verify the consumer processed the dependency data
    let context_data: std::collections::HashMap<String, Value> =
        if let Some(context_id) = consumer_metadata.context_id {
            let context = dal
                .context()
                .read::<serde_json::Value>(context_id)
                .await
                .unwrap();
            context.data().clone()
        } else {
            std::collections::HashMap::new()
        };

    assert!(
        context_data.contains_key("derived_from_shared_data"),
        "Consumer task should have processed dependency data"
    );

    if let Some(derived_value) = context_data.get("derived_from_shared_data") {
        assert_eq!(
            derived_value,
            &Value::String("Processed: \"important_value\"".to_string()),
            "Derived value should contain processed dependency data"
        );
    }

    // Cleanup
    scheduler_handle.abort();
    executor_handle.abort();
}

#[tokio::test]
async fn test_task_executor_timeout_handling() {
    let database = get_test_database().await;

    // Create a task that will timeout
    #[derive(Debug)]
    struct TimeoutTask {
        id: String,
    }

    #[async_trait]
    impl Task for TimeoutTask {
        async fn execute(&self, context: Context<Value>) -> Result<Context<Value>, TaskError> {
            // Sleep longer than the timeout
            time::sleep(Duration::from_secs(10)).await;
            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &[]
        }

        fn retry_policy(&self) -> cloacina::retry::RetryPolicy {
            // For this test, we want the task to fail immediately without retries
            cloacina::retry::RetryPolicy {
                max_attempts: 1,
                backoff_strategy: cloacina::retry::BackoffStrategy::Fixed,
                initial_delay: std::time::Duration::from_millis(1000),
                max_delay: std::time::Duration::from_millis(1000),
                jitter: false,
                retry_conditions: vec![cloacina::retry::RetryCondition::Never],
            }
        }
    }

    let mut task_registry = TaskRegistry::new();
    let timeout_task = TimeoutTask {
        id: "timeout_task".to_string(),
    };
    task_registry.register(timeout_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow
    let workflow = Workflow::builder("timeout_pipeline")
        .description("Test pipeline with timeout")
        .add_task(WorkflowTask::new("timeout_task", vec![]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("test_data", Value::String("timeout_test".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("timeout_pipeline", input_context)
        .await
        .unwrap();

    // Process scheduling
    scheduler.process_active_pipelines().await.unwrap();

    // Create executor with short timeout
    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_millis(500), // Short timeout
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);

    // Run executor briefly
    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for timeout to occur
    time::sleep(Duration::from_secs(2)).await;

    // Check that task failed due to timeout
    let dal = cloacina::dal::DAL::new(database.pool());
    let task_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), "timeout_task")
        .await
        .unwrap();

    assert_eq!(
        task_status, "Failed",
        "Task should have failed due to timeout"
    );

    // Cleanup
    executor_handle.abort();
}

#[tokio::test]
async fn test_pipeline_engine_unified_mode() {
    let database = get_test_database().await;

    // Create task registry
    let mut task_registry = TaskRegistry::new();
    let task = SimpleExecutorTask::new(
        "unified_task",
        vec![],
        "result",
        Value::String("unified_success".to_string()),
    );
    task_registry.register(task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow
    let workflow = Workflow::builder("unified_pipeline")
        .description("Test pipeline for unified mode")
        .add_task(WorkflowTask::new("unified_task", vec![]))
        .unwrap()
        .build()
        .unwrap();

    // Create pipeline engine
    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let engine = PipelineEngine::new(
        database.clone(),
        task_registry,
        vec![workflow],
        config,
        EngineMode::Unified,
    );

    // Create a separate workflow for scheduling since we can't clone
    let schedule_workflow = Workflow::builder("unified_pipeline")
        .description("Test pipeline for unified mode")
        .add_task(WorkflowTask::new("unified_task", vec![]))
        .unwrap()
        .build()
        .unwrap();

    // Schedule a workflow execution manually (since we're testing the engine, not the API)
    let scheduler = TaskScheduler::new(database.clone(), vec![schedule_workflow]);
    let mut input_context = Context::new();
    input_context
        .insert("engine_test", Value::String("unified_mode".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("unified_pipeline", input_context)
        .await
        .unwrap();

    // Run the engine briefly
    let engine_handle = tokio::spawn(async move { engine.run().await });

    // Give time for execution
    time::sleep(Duration::from_secs(1)).await;

    // Check that task was processed
    let dal = cloacina::dal::DAL::new(database.pool());
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "unified_task")
        .await;

    // If the task was executed, metadata should exist
    match task_metadata {
        Ok(metadata) => {
            if let Some(context_id) = metadata.context_id {
                let context = dal
                    .context()
                    .read::<serde_json::Value>(context_id)
                    .await
                    .unwrap();
                let context_data = context.data();
                assert!(
                    context_data.contains_key("result"),
                    "Task output should be present"
                );
            } else {
                // Task completed but produced no output
                println!("Task completed but produced no output context");
            }
        }
        Err(_) => {
            // Task might still be in progress or failed - check execution status
            let task_status = dal
                .task_execution()
                .get_task_status(UniversalUuid(pipeline_id), "unified_task")
                .await
                .unwrap();
            assert_ne!(task_status, "Pending", "Task should have been processed");
        }
    }

    // Cleanup
    engine_handle.abort();
}

#[tokio::test]
async fn test_task_executor_context_loading_no_dependencies() {
    let database = get_test_database().await;

    // Create a task that validates it received initial context
    #[derive(Debug)]
    struct InitialContextTask {
        id: String,
    }

    #[async_trait]
    impl Task for InitialContextTask {
        async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
            // Verify we can access the initial context data
            let initial_value =
                context
                    .get("initial_data")
                    .ok_or_else(|| TaskError::ValidationFailed {
                        message: "No initial_data found in context".to_string(),
                    })?;

            // Add a processed value to show the task ran
            context
                .insert(
                    "processed_initial",
                    Value::String(format!("Processed: {}", initial_value)),
                )
                .map_err(|e| TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: format!("Failed to insert processed value: {}", e),
                })?;

            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &[] // No dependencies - should get initial context
        }
    }

    let mut task_registry = TaskRegistry::new();
    let initial_context_task = InitialContextTask {
        id: "initial_context_task".to_string(),
    };
    task_registry.register(initial_context_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow
    let workflow = Workflow::builder("initial_context_pipeline")
        .description("Test pipeline for initial context loading")
        .add_task(WorkflowTask::new("initial_context_task", vec![]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution with initial context
    let mut input_context = Context::new();
    input_context
        .insert("initial_data", Value::String("hello_world".to_string()))
        .unwrap();
    input_context
        .insert("config_value", Value::Number(serde_json::Number::from(42)))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("initial_context_pipeline", input_context)
        .await
        .unwrap();

    // Run scheduling and execution
    let scheduler_handle = tokio::spawn(async move { scheduler.run_scheduling_loop().await });

    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);
    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for execution
    time::sleep(Duration::from_secs(1)).await;

    // Verify the task successfully processed the initial context
    let dal = cloacina::dal::DAL::new(database.pool());
    let task_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), "initial_context_task")
        .await
        .unwrap();
    assert_eq!(
        task_status, "Completed",
        "Task should have completed successfully"
    );

    // Check the output context contains processed data
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "initial_context_task")
        .await
        .unwrap();

    if let Some(context_id) = task_metadata.context_id {
        let context = dal
            .context()
            .read::<serde_json::Value>(context_id)
            .await
            .unwrap();
        let context_data = context.data();

        assert!(
            context_data.contains_key("processed_initial"),
            "Task should have processed initial context data"
        );
        assert!(
            context_data.contains_key("config_value"),
            "Initial context should be preserved"
        );

        if let Some(processed) = context_data.get("processed_initial") {
            assert_eq!(
                processed,
                &Value::String("Processed: \"hello_world\"".to_string())
            );
        }
    } else {
        panic!("Task should have produced output context");
    }

    // Cleanup
    scheduler_handle.abort();
    executor_handle.abort();
}

#[tokio::test]
async fn test_task_executor_context_loading_with_dependencies() {
    let database = get_test_database().await;

    // Task that produces data (no dependencies)
    #[derive(Debug)]
    struct ProducerTask {
        id: String,
    }

    #[async_trait]
    impl Task for ProducerTask {
        async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
            // Should have access to initial context
            let initial_value =
                context
                    .get("seed_value")
                    .ok_or_else(|| TaskError::ValidationFailed {
                        message: "No seed_value found in context".to_string(),
                    })?;

            // Produce some data
            context
                .insert(
                    "produced_data",
                    Value::String(format!("Produced from: {}", initial_value)),
                )
                .map_err(|e| TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: format!("Failed to insert produced data: {}", e),
                })?;

            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &[]
        }
    }

    // Task that consumes dependency data
    #[derive(Debug)]
    struct ConsumerTask {
        id: String,
        dependencies: Vec<String>,
    }

    #[async_trait]
    impl Task for ConsumerTask {
        async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
            // Should have access to dependency context data (not initial context directly)
            let produced_data =
                context
                    .get("produced_data")
                    .ok_or_else(|| TaskError::ValidationFailed {
                        message: "No produced_data found in context from dependency".to_string(),
                    })?;

            // Should also have initial context merged in
            let seed_value =
                context
                    .get("seed_value")
                    .ok_or_else(|| TaskError::ValidationFailed {
                        message: "No seed_value found in context".to_string(),
                    })?;

            // Process the data
            context
                .insert(
                    "final_result",
                    Value::String(format!("Final: {} + {}", produced_data, seed_value)),
                )
                .map_err(|e| TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: format!("Failed to insert final result: {}", e),
                })?;

            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &self.dependencies
        }
    }

    let mut task_registry = TaskRegistry::new();
    let producer_task = ProducerTask {
        id: "producer".to_string(),
    };
    let consumer_task = ConsumerTask {
        id: "consumer".to_string(),
        dependencies: vec!["producer".to_string()],
    };

    task_registry.register(producer_task).unwrap();
    task_registry.register(consumer_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow with dependency chain
    let workflow = Workflow::builder("dependency_context_pipeline")
        .description("Test pipeline for dependency context loading")
        .add_task(WorkflowTask::new("producer", vec![]))
        .unwrap()
        .add_task(WorkflowTask::new("consumer", vec!["producer"]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution with initial context
    let mut input_context = Context::new();
    input_context
        .insert("seed_value", Value::String("initial_seed".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("dependency_context_pipeline", input_context)
        .await
        .unwrap();

    // Run scheduling and execution
    let scheduler_handle = tokio::spawn(async move { scheduler.run_scheduling_loop().await });

    let config = ExecutorConfig {
        max_concurrent_tasks: 2,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);
    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for both tasks to execute
    time::sleep(Duration::from_secs(2)).await;

    // Verify both tasks completed
    let dal = cloacina::dal::DAL::new(database.pool());
    let producer_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), "producer")
        .await
        .unwrap();
    let consumer_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), "consumer")
        .await
        .unwrap();

    assert_eq!(
        producer_status, "Completed",
        "Producer task should have completed"
    );
    assert_eq!(
        consumer_status, "Completed",
        "Consumer task should have completed"
    );

    // Check the consumer's output context
    let consumer_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "consumer")
        .await
        .unwrap();

    if let Some(context_id) = consumer_metadata.context_id {
        let context = dal
            .context()
            .read::<serde_json::Value>(context_id)
            .await
            .unwrap();
        let context_data = context.data();

        assert!(
            context_data.contains_key("final_result"),
            "Consumer should have produced final result"
        );
        assert!(
            context_data.contains_key("produced_data"),
            "Consumer should have access to producer data"
        );
        assert!(
            context_data.contains_key("seed_value"),
            "Consumer should have access to initial context"
        );

        if let Some(final_result) = context_data.get("final_result") {
            assert_eq!(
                final_result,
                &Value::String(
                    "Final: \"Produced from: \\\"initial_seed\\\"\" + \"initial_seed\"".to_string()
                )
            );
        }
    } else {
        panic!("Consumer task should have produced output context");
    }

    // Cleanup
    scheduler_handle.abort();
    executor_handle.abort();
}
