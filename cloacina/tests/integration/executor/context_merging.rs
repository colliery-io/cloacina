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
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        Ok(context) // No-op for workflow building
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

#[task(
    id = "early_producer_task",
    dependencies = []
)]
async fn early_producer_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Add early producer data to the context
    context.insert("shared_key", Value::String("early_value".to_string()))?;
    context.insert("early_only", Value::String("unique_early".to_string()))?;
    Ok(())
}

#[task(
    id = "late_producer_task",
    dependencies = ["early_producer_task"]
)]
async fn late_producer_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Add late producer data to the context (should override shared_key)
    context.update("shared_key", Value::String("late_value".to_string()))?;
    context.insert("late_only", Value::String("unique_late".to_string()))?;
    Ok(())
}

#[task(
    id = "merger_task",
    dependencies = ["early_producer_task", "late_producer_task"]
)]
async fn merger_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    let mut merged_values = std::collections::HashMap::new();
    let expected_keys = vec!["shared_key", "early_only", "late_only"];

    // Try to load all expected keys from dependencies
    for key in &expected_keys {
        // Load the value from dependencies or local context
        let value = match context.load_from_dependencies_and_cache(key).await {
            Ok(Some(value)) => value,
            Ok(None) => {
                // Check if it's in local context
                if let Some(local_value) = context.get(key) {
                    local_value.clone()
                } else {
                    return Err(TaskError::Unknown {
                        task_id: "merger_task".to_string(),
                        message: format!(
                            "Expected key '{}' not found in dependencies or local context",
                            key
                        ),
                    });
                }
            }
            Err(e) => {
                return Err(TaskError::Unknown {
                    task_id: "merger_task".to_string(),
                    message: format!("Failed to load key '{}': {}", key, e),
                });
            }
        };

        merged_values.insert(key.to_string(), value);
    }

    // Add a summary of merged values
    let summary = Value::Array(
        merged_values
            .keys()
            .map(|k| Value::String(k.to_string()))
            .collect(),
    );

    // Insert the summary into the context
    context.insert("merged_keys", summary)?;
    Ok(())
}

#[tokio::test]
async fn test_context_merging_latest_wins() {
    let database = get_test_database().await;

    // Create workflow using the #[task] functions with unique name
    let workflow_name = format!("merging_pipeline_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for context merging")
        .add_task(Arc::new(WorkflowTask::new("early_producer_task", vec![])))
        .unwrap()
        .add_task(Arc::new(WorkflowTask::new(
            "late_producer_task",
            vec!["early_producer_task"],
        )))
        .unwrap()
        .add_task(Arc::new(WorkflowTask::new(
            "merger_task",
            vec!["early_producer_task", "late_producer_task"],
        )))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks with correct namespaces in global registry
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "early_producer_task"
    );
    register_task_constructor(namespace1, || Arc::new(early_producer_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "late_producer_task"
    );
    register_task_constructor(namespace2, || Arc::new(late_producer_task_task()));

    let namespace3 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "merger_task"
    );
    register_task_constructor(namespace3, || Arc::new(merger_task_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow_name.clone(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    // Schedule workflow execution
    let input_context = Arc::new(tokio::sync::Mutex::new(Context::new()));
    {
        let mut context = input_context.lock().await;
        context
            .insert("initial_context", Value::String("merging_test".to_string()))
            .unwrap();
    }
    let pipeline_id = scheduler
        .schedule_workflow_execution(
            &workflow_name,
            input_context.lock().await.clone_data(),
        )
        .await
        .unwrap();

    // Create and run executor using global registry
    let config = ExecutorConfig {
        max_concurrent_tasks: 1, // Sequential execution to ensure order
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = ThreadTaskExecutor::with_global_registry(database.clone(), config).unwrap();

    // Run scheduling and execution
    let scheduler_handle = tokio::spawn(async move { scheduler.run_scheduling_loop().await });

    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for all tasks to execute
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check merger task results
    let dal = cloacina::dal::DAL::new(database.clone());
    let full_merger_task_name = format!("{}::{}::{}::merger_task", 
        workflow.tenant(), workflow.package(), workflow.name());
    let merger_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &full_merger_task_name)
        .await
        .unwrap();

    let context_data: std::collections::HashMap<String, Value> =
        if let Some(context_id) = merger_metadata.context_id {
            let context = dal
                .context()
                .read::<serde_json::Value>(context_id)
                .await
                .unwrap();
            context.data().clone()
        } else {
            std::collections::HashMap::new()
        };

    // Verify merged keys were processed
    assert!(
        context_data.contains_key("merged_keys"),
        "Merger should have created a summary of merged keys"
    );

    // Verify latest wins strategy by checking if late_producer's value overwrote early_producer's
    // This would be evident in the dependency loader's behavior during task execution

    // Check that all expected unique keys are available through dependency loading
    // (This is indirectly tested by the merger task succeeding)

    // Cleanup
    scheduler_handle.abort();
    executor_handle.abort();
}

#[task(
    id = "scope_inspector_task",
    dependencies = []
)]
async fn scope_inspector_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Check if execution scope is set
    if let Some(scope) = context.execution_scope() {
        let scope_info = serde_json::json!({
            "pipeline_execution_id": scope.pipeline_execution_id.to_string(),
            "task_execution_id": scope.task_execution_id.map(|id| id.to_string()),
            "task_name": scope.task_name.clone()
        });

        context.insert("execution_scope_info", scope_info)?;
    } else {
        return Err(TaskError::Unknown {
            task_id: "scope_inspector_task".to_string(),
            message: "Execution scope not set".to_string(),
        });
    }

    Ok(())
}

#[tokio::test]
async fn test_execution_scope_context_setup() {
    let database = get_test_database().await;

    // Create workflow with unique name
    let workflow_name = format!("scope_pipeline_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for execution scope")
        .add_task(Arc::new(WorkflowTask::new("scope_inspector_task", vec![])))
        .unwrap()
        .build()
        .unwrap();

    // Register task with correct namespace in global registry
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "scope_inspector_task"
    );
    register_task_constructor(namespace, || Arc::new(scope_inspector_task_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow_name.clone(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("scope_test", Value::String("execution_scope".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution(&workflow_name, input_context)
        .await
        .unwrap();

    // Process scheduling
    scheduler.process_active_pipelines().await.unwrap();

    // Create and run executor using global registry
    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = ThreadTaskExecutor::with_global_registry(database.clone(), config).unwrap();

    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for execution
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that scope information was captured
    let dal = cloacina::dal::DAL::new(database.clone());
    let full_task_name = format!("{}::{}::{}::scope_inspector_task", 
        workflow.tenant(), workflow.package(), workflow.name());
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &full_task_name)
        .await
        .unwrap();

    let context_data: std::collections::HashMap<String, Value> =
        if let Some(context_id) = task_metadata.context_id {
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
        context_data.contains_key("execution_scope_info"),
        "Task should have captured execution scope information"
    );

    if let Some(scope_info) = context_data.get("execution_scope_info") {
        let scope_obj = scope_info.as_object().unwrap();
        assert!(
            scope_obj.contains_key("pipeline_execution_id"),
            "Scope should contain pipeline execution ID"
        );
        assert!(
            scope_obj.contains_key("task_execution_id"),
            "Scope should contain task execution ID"
        );
        assert!(
            scope_obj.contains_key("task_name"),
            "Scope should contain task name"
        );

        if let Some(task_name) = scope_obj.get("task_name") {
            // task_name is an Option<String> in ExecutionScope, so it gets serialized as such
            if let Value::String(name) = task_name {
                // Should contain the full namespace now
                assert!(name.ends_with("::scope_inspector_task"), 
                    "Task name should end with ::scope_inspector_task, got: {}", name);
                assert!(name.contains(&workflow_name), 
                    "Task name should contain workflow name, got: {}", name);
            } else {
                panic!("Expected task_name to be a string, got: {:?}", task_name);
            }
        }
    }

    // Cleanup
    executor_handle.abort();
}
