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

use async_trait::async_trait;
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::fixtures::get_or_init_fixture;

// Simple task for workflow construction
#[derive(Debug)]
struct WorkflowTask {
    id: String,
    dependencies: Vec<TaskNamespace>,
}

impl WorkflowTask {
    fn new(id: &str, deps: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps
                .into_iter()
                .map(|s| TaskNamespace::from_string(s).unwrap())
                .collect(),
        }
    }
}

#[async_trait]
impl Task for WorkflowTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        Ok(context) // No-op for workflow building
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

#[task(
    id = "test_task",
    dependencies = []
)]
async fn test_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Add test output to the context
    context.insert("result", Value::String("success".to_string()))?;
    Ok(())
}

#[task(
    id = "producer_task",
    dependencies = []
)]
async fn producer_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Add shared data to the context
    context.insert("shared_data", Value::String("important_value".to_string()))?;
    Ok(())
}

#[task(
    id = "consumer_task",
    dependencies = ["producer_task"]
)]
async fn consumer_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // With pre-inject pattern, dependency data is already in context
    if let Some(value) = context.get("shared_data") {
        // Add a derived value to show dependency was loaded
        context.insert(
            "derived_from_shared_data",
            Value::String(format!("Processed: {}", value)),
        )?;
    } else {
        return Err(TaskError::Unknown {
            task_id: "consumer_task".to_string(),
            message: "Dependency key 'shared_data' not found".to_string(),
        });
    }

    Ok(())
}

#[task(
    id = "timeout_task_test",
    dependencies = [],
    retry_attempts = 1
)]
async fn timeout_task_test(_context: &mut Context<Value>) -> Result<(), TaskError> {
    // Sleep longer than the timeout
    time::sleep(Duration::from_secs(10)).await;
    Ok(())
}

#[tokio::test]
async fn test_task_executor_basic_execution() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    // Reset the database to ensure a clean state
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow using the #[task] function
    let workflow = Workflow::builder("test_pipeline_basic")
        .description("Test pipeline for executor")
        .add_task(Arc::new(WorkflowTask::new("test_task", vec![])))
        .unwrap()
        .build()
        .unwrap();

    // Register task with correct namespace in global registry
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "test_task",
    );
    register_task_constructor(namespace, || Arc::new(test_task_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor("test_pipeline_basic".to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with proper schema isolation
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("test_data", Value::String("test_value".to_string()))
        .unwrap();
    let execution = runner
        .execute_async("test_pipeline_basic", input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until task completes (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "task should complete",
        || {
            let dal = dal.clone();
            async move {
                let tasks = dal
                    .task_execution()
                    .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
                    .await
                    .unwrap_or_default();
                tasks.len() == 1 && tasks[0].status == "Completed"
            }
        },
    )
    .await;

    let task_executions = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
        .await
        .unwrap();
    assert_eq!(task_executions.len(), 1);
    let task = &task_executions[0];
    assert_eq!(task.status, "Completed");
    let expected_task_name = format!(
        "{}::{}::{}::test_task",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    assert_eq!(task.task_name, expected_task_name);

    // Clean up
    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_task_executor_dependency_loading() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow with dependencies using the #[task] functions
    let workflow_name = format!(
        "dependency_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Create namespaces for dependencies
    let producer_ns = TaskNamespace::new("public", "embedded", &workflow_name, "producer_task");

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline with dependencies")
        .add_task(Arc::new(producer_task_task()))
        .unwrap()
        .add_task(Arc::new(
            consumer_task_task().with_dependencies(vec![producer_ns.clone()]),
        ))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks with correct namespaces in global registry
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "producer_task",
    );
    register_task_constructor(namespace1, || Arc::new(producer_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "consumer_task",
    );
    let producer_ns_for_closure = producer_ns.clone();
    register_task_constructor(namespace2, move || {
        Arc::new(consumer_task_task().with_dependencies(vec![producer_ns_for_closure.clone()]))
    });

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with proper schema isolation
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("initial_data", Value::String("test_value".to_string()))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until both tasks complete (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    let consumer_namespace_for_poll = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "consumer_task",
    );
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "consumer task should complete",
        || {
            let dal = dal.clone();
            let ns = consumer_namespace_for_poll.clone();
            async move {
                let meta = dal
                    .task_execution_metadata()
                    .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &ns)
                    .await;
                meta.is_ok()
            }
        },
    )
    .await;

    // Check that consumer task successfully loaded dependency data
    let consumer_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "consumer_task",
    );
    let consumer_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &consumer_namespace)
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
    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_task_executor_timeout_handling() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow with timeout task
    let workflow_name = format!(
        "timeout_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline with timeout")
        .add_task(Arc::new(WorkflowTask::new("timeout_task_test", vec![])))
        .unwrap()
        .build()
        .unwrap();

    // Register task with correct namespace in global registry
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "timeout_task_test",
    );
    register_task_constructor(namespace, || Arc::new(timeout_task_test_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with short timeout and proper schema isolation
    let config = DefaultRunnerConfig::builder()
        .task_timeout(Duration::from_millis(500))
        .build();
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .with_config(config)
        .build()
        .await
        .unwrap();

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("test_data", Value::String("timeout_test".to_string()))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until task fails due to timeout (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    let full_task_name = format!(
        "{}::{}::{}::timeout_task_test",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "task should fail due to timeout",
        || {
            let dal = dal.clone();
            let name = full_task_name.clone();
            async move {
                let status = dal
                    .task_execution()
                    .get_task_status(UniversalUuid(pipeline_id), &name)
                    .await
                    .unwrap_or_default();
                status == "Failed"
            }
        },
    )
    .await;

    let task_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), &full_task_name)
        .await
        .unwrap();

    assert_eq!(
        task_status, "Failed",
        "Task should have failed due to timeout"
    );

    // COR-01: Verify PIPELINE status reflects the task failure.
    // Previously, pipelines were always marked "Completed" even with failed tasks.
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "pipeline should be marked as terminal (Failed or Completed)",
        || {
            let dal = dal.clone();
            async move {
                if let Ok(exec) = dal
                    .workflow_execution()
                    .get_by_id(UniversalUuid(pipeline_id))
                    .await
                {
                    exec.status == "Failed" || exec.status == "Completed"
                } else {
                    false
                }
            }
        },
    )
    .await;

    let pipeline_exec = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(pipeline_id))
        .await
        .unwrap();
    assert_eq!(
        pipeline_exec.status, "Failed",
        "Pipeline with failed task(s) must be marked Failed, not Completed"
    );

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[task(
    id = "unified_task_test",
    dependencies = []
)]
async fn unified_task_test(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Add test output to the context
    context.insert("result", Value::String("unified_success".to_string()))?;
    Ok(())
}

#[tokio::test]
async fn test_default_runner_execution() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow using the #[task] function
    let workflow = Workflow::builder("unified_pipeline_test")
        .description("Test pipeline for unified mode")
        .add_task(Arc::new(WorkflowTask::new("unified_task_test", vec![])))
        .unwrap()
        .build()
        .unwrap();

    // Register task with correct namespace in global registry
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "unified_task_test",
    );
    register_task_constructor(namespace, || Arc::new(unified_task_test_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with proper schema isolation
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Schedule a workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("engine_test", Value::String("unified_mode".to_string()))
        .unwrap();
    let execution = runner
        .execute_async("unified_pipeline_test", input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until task is processed (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    let task_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "unified_task_test",
    );
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "unified task should be processed",
        || {
            let dal = dal.clone();
            let ns = task_namespace.clone();
            async move {
                dal.task_execution_metadata()
                    .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &ns)
                    .await
                    .is_ok()
            }
        },
    )
    .await;

    // Check that task was processed
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &task_namespace)
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
            let full_task_name = format!(
                "{}::{}::{}::unified_task_test",
                workflow.tenant(),
                workflow.package(),
                workflow.name()
            );
            let task_status = dal
                .task_execution()
                .get_task_status(UniversalUuid(pipeline_id), &full_task_name)
                .await
                .unwrap();
            assert_ne!(task_status, "Pending", "Task should have been processed");
        }
    }

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[task(
    id = "initial_context_task_test",
    dependencies = []
)]
async fn initial_context_task_test(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Verify we can access the initial context data
    let initial_value = context
        .get("initial_data")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No initial_data found in context".to_string(),
        })?;

    // Add a processed value to show the task ran
    context.insert(
        "processed_initial",
        Value::String(format!("Processed: {}", initial_value)),
    )?;

    Ok(())
}

#[tokio::test]
async fn test_task_executor_context_loading_no_dependencies() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow using the #[task] function with unique name
    let workflow_name = format!(
        "initial_context_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for initial context loading")
        .add_task(Arc::new(WorkflowTask::new(
            "initial_context_task_test",
            vec![],
        )))
        .unwrap()
        .build()
        .unwrap();

    // Register task with correct namespace in global registry
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "initial_context_task_test",
    );
    register_task_constructor(namespace, || Arc::new(initial_context_task_test_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with proper schema isolation
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Schedule workflow execution with initial context
    let mut input_context = Context::new();
    input_context
        .insert("initial_data", Value::String("hello_world".to_string()))
        .unwrap();
    input_context
        .insert("config_value", Value::Number(serde_json::Number::from(42)))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until task completes (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    let full_task_name = format!(
        "{}::{}::{}::initial_context_task_test",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "initial_context_task should complete",
        || {
            let dal = dal.clone();
            let name = full_task_name.clone();
            async move {
                let status = dal
                    .task_execution()
                    .get_task_status(UniversalUuid(pipeline_id), &name)
                    .await
                    .unwrap_or_default();
                status == "Completed"
            }
        },
    )
    .await;

    // Verify the task successfully processed the initial context
    let task_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), &full_task_name)
        .await
        .unwrap();
    assert_eq!(
        task_status, "Completed",
        "Task should have completed successfully"
    );

    // Check the output context contains processed data
    let task_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "initial_context_task_test",
    );
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &task_namespace)
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
    runner.shutdown().await.unwrap();
}

#[task(
    id = "producer_context_task",
    dependencies = []
)]
async fn producer_context_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Should have access to initial context
    let initial_value = context
        .get("seed_value")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No seed_value found in context".to_string(),
        })?;

    // Produce some data
    context.insert(
        "produced_data",
        Value::String(format!("Produced from: {}", initial_value)),
    )?;

    Ok(())
}

#[task(
    id = "consumer_context_task",
    dependencies = ["producer_context_task"]
)]
async fn consumer_context_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Should have access to dependency context data (not initial context directly)
    let produced_data =
        context
            .get("produced_data")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "No produced_data found in context from dependency".to_string(),
            })?;

    // Should also have initial context merged in
    let seed_value = context
        .get("seed_value")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No seed_value found in context".to_string(),
        })?;

    // Process the data
    context.insert(
        "final_result",
        Value::String(format!("Final: {} + {}", produced_data, seed_value)),
    )?;

    Ok(())
}

#[tokio::test]
async fn test_task_executor_context_loading_with_dependencies() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow with dependency chain using the #[task] functions
    let workflow_name = format!(
        "dependency_context_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Create namespaces for dependencies
    let producer_ns = TaskNamespace::new(
        "public",
        "embedded",
        &workflow_name,
        "producer_context_task",
    );

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for dependency context loading")
        .add_task(Arc::new(producer_context_task_task()))
        .unwrap()
        .add_task(Arc::new(
            consumer_context_task_task().with_dependencies(vec![producer_ns.clone()]),
        ))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks with correct namespaces and dependencies in global registry
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "producer_context_task",
    );
    register_task_constructor(namespace1, || Arc::new(producer_context_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "consumer_context_task",
    );
    let producer_ns_for_closure = producer_ns.clone();
    register_task_constructor(namespace2, move || {
        Arc::new(
            consumer_context_task_task().with_dependencies(vec![producer_ns_for_closure.clone()]),
        )
    });

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with proper schema isolation
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Schedule workflow execution with initial context
    let mut input_context = Context::new();
    input_context
        .insert("seed_value", Value::String("initial_seed".to_string()))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until both tasks complete (replaces fixed sleep)
    let dal = cloacina::dal::DAL::new(database.clone());
    let producer_full_name = format!(
        "{}::{}::{}::producer_context_task",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    let consumer_full_name = format!(
        "{}::{}::{}::consumer_context_task",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    crate::fixtures::poll_until(
        Duration::from_secs(10),
        Duration::from_millis(100),
        "both producer and consumer tasks should complete",
        || {
            let dal = dal.clone();
            let producer_name = producer_full_name.clone();
            let consumer_name = consumer_full_name.clone();
            async move {
                let p = dal
                    .task_execution()
                    .get_task_status(UniversalUuid(pipeline_id), &producer_name)
                    .await
                    .unwrap_or_default();
                let c = dal
                    .task_execution()
                    .get_task_status(UniversalUuid(pipeline_id), &consumer_name)
                    .await
                    .unwrap_or_default();
                p == "Completed" && c == "Completed"
            }
        },
    )
    .await;

    // Verify both tasks completed
    let producer_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), &producer_full_name)
        .await
        .unwrap();
    let consumer_status = dal
        .task_execution()
        .get_task_status(UniversalUuid(pipeline_id), &consumer_full_name)
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
    let consumer_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "consumer_context_task",
    );
    let consumer_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &consumer_namespace)
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
    runner.shutdown().await.unwrap();
}

// =============================================================================
// COR-01 Regression Tests: Pipeline completion status must reflect task outcomes
// =============================================================================

/// A task that always fails immediately.
#[task(id = "always_fails_task", dependencies = [], retry_attempts = 0)]
async fn always_fails_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    Err(TaskError::Unknown {
        task_id: "always_fails_task".to_string(),
        message: "intentional failure for COR-01 test".to_string(),
    })
}

/// A task that always succeeds.
#[task(id = "always_succeeds_task", dependencies = [])]
async fn always_succeeds_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    context.insert("success", Value::Bool(true))?;
    Ok(())
}

/// A task that depends on always_fails_task (will be skipped when dep fails).
#[task(id = "downstream_of_failure", dependencies = ["always_fails_task"])]
async fn downstream_of_failure(context: &mut Context<Value>) -> Result<(), TaskError> {
    context.insert("downstream_ran", Value::Bool(true))?;
    Ok(())
}

/// Helper to set up a runner with registered tasks and workflow, execute, and
/// return the final pipeline status string.
async fn run_pipeline_and_get_status(
    workflow_name: &str,
    task_defs: Vec<(&str, Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>)>,
    dep_map: Vec<(&str, Vec<&str>)>,
) -> String {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    let unique_name = format!(
        "{}_{}",
        workflow_name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Workflow::builder defaults to tenant="public", package="embedded"
    let default_tenant = "public";
    let default_package = "embedded";

    let mut builder = Workflow::builder(&unique_name).description("COR-01 regression test");

    for (task_id, deps) in &dep_map {
        // Convert bare dep names to fully-qualified namespaces
        let qualified_deps: Vec<String> = deps
            .iter()
            .map(|d| {
                format!(
                    "{}::{}::{}::{}",
                    default_tenant, default_package, unique_name, d
                )
            })
            .collect();
        let qualified_refs: Vec<&str> = qualified_deps.iter().map(|s| s.as_str()).collect();
        builder = builder
            .add_task(Arc::new(WorkflowTask::new(task_id, qualified_refs)))
            .unwrap();
    }
    let workflow = builder.build().unwrap();

    // Register tasks
    for (task_id, constructor) in &task_defs {
        let namespace = TaskNamespace::new(
            workflow.tenant(),
            workflow.package(),
            workflow.name(),
            task_id,
        );
        let ctor = constructor();
        register_task_constructor(namespace, move || ctor.clone());
    }

    // Register workflow
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    let execution = runner
        .execute_async(&unique_name, Context::new())
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Poll until pipeline reaches a terminal state
    let dal = cloacina::dal::DAL::new(database.clone());
    crate::fixtures::poll_until(
        Duration::from_secs(15),
        Duration::from_millis(100),
        "pipeline should reach terminal state",
        || {
            let dal = dal.clone();
            async move {
                if let Ok(exec) = dal
                    .workflow_execution()
                    .get_by_id(UniversalUuid(pipeline_id))
                    .await
                {
                    exec.status == "Failed" || exec.status == "Completed"
                } else {
                    false
                }
            }
        },
    )
    .await;

    let exec = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(pipeline_id))
        .await
        .unwrap();

    runner.shutdown().await.unwrap();
    exec.status
}

/// COR-01: Pipeline where all tasks succeed must be marked "Completed".
#[tokio::test]
#[serial_test::serial]
async fn test_pipeline_all_tasks_succeed_marked_completed() {
    let status = run_pipeline_and_get_status(
        "cor01_all_succeed",
        vec![(
            "always_succeeds_task",
            Box::new(|| Arc::new(always_succeeds_task_task())),
        )],
        vec![("always_succeeds_task", vec![])],
    )
    .await;

    assert_eq!(
        status, "Completed",
        "Pipeline with all successful tasks must be Completed"
    );
}

/// COR-01: Pipeline where a task fails must be marked "Failed".
#[tokio::test]
#[serial_test::serial]
async fn test_pipeline_task_fails_marked_failed() {
    let status = run_pipeline_and_get_status(
        "cor01_task_fails",
        vec![(
            "always_fails_task",
            Box::new(|| Arc::new(always_fails_task_task())),
        )],
        vec![("always_fails_task", vec![])],
    )
    .await;

    assert_eq!(
        status, "Failed",
        "Pipeline with failed task must be Failed, not Completed"
    );
}

/// COR-01: Pipeline with mixed results (one succeeds, one fails) must be "Failed".
#[tokio::test]
#[serial_test::serial]
async fn test_pipeline_mixed_results_marked_failed() {
    let status = run_pipeline_and_get_status(
        "cor01_mixed",
        vec![
            (
                "always_succeeds_task",
                Box::new(|| Arc::new(always_succeeds_task_task())),
            ),
            (
                "always_fails_task",
                Box::new(|| Arc::new(always_fails_task_task())),
            ),
        ],
        vec![
            ("always_succeeds_task", vec![]),
            ("always_fails_task", vec![]),
        ],
    )
    .await;

    assert_eq!(
        status, "Failed",
        "Pipeline with any failed task must be Failed"
    );
}

/// COR-01: Pipeline where a task fails and downstream tasks are skipped must be "Failed".
#[tokio::test]
#[serial_test::serial]
async fn test_pipeline_skipped_downstream_marked_failed() {
    let status = run_pipeline_and_get_status(
        "cor01_skipped_downstream",
        vec![
            (
                "always_fails_task",
                Box::new(|| Arc::new(always_fails_task_task())),
            ),
            (
                "downstream_of_failure",
                Box::new(|| Arc::new(downstream_of_failure_task())),
            ),
        ],
        vec![
            ("always_fails_task", vec![]),
            ("downstream_of_failure", vec!["always_fails_task"]),
        ],
    )
    .await;

    assert_eq!(
        status, "Failed",
        "Pipeline with failed task + skipped dependents must be Failed"
    );
}
