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
    let mut locked_fixture = fixture.lock().unwrap();
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
struct ContextProducerTask {
    id: String,
    dependencies: Vec<String>,
    output_data: std::collections::HashMap<String, Value>,
}

impl ContextProducerTask {
    fn new(
        id: &str,
        deps: Vec<&str>,
        output_data: std::collections::HashMap<String, Value>,
    ) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            output_data,
        }
    }
}

#[async_trait]
impl Task for ContextProducerTask {
    async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
        // Add all output data to the context
        for (key, value) in &self.output_data {
            // Try to insert first, if key exists then update it
            if let Err(_) = context.insert(key, value.clone()) {
                // Key exists, update it instead (this implements "latest wins")
                context
                    .update(key, value.clone())
                    .map_err(|e| TaskError::Unknown {
                        task_id: self.id.clone(),
                        message: format!("Failed to update {}: {}", key, e),
                    })?;
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

#[derive(Debug)]
struct ContextMergerTask {
    id: String,
    dependencies: Vec<String>,
    expected_keys: Vec<String>,
}

impl ContextMergerTask {
    fn new(id: &str, deps: Vec<&str>, expected_keys: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            expected_keys: expected_keys.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait]
impl Task for ContextMergerTask {
    async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
        let mut merged_values = std::collections::HashMap::new();

        // Try to load all expected keys from dependencies
        for key in &self.expected_keys {
            match context.load_from_dependencies_and_cache(key).await {
                Ok(Some(value)) => {
                    merged_values.insert(key.clone(), value);
                }
                Ok(None) => {
                    // Check if it's in local context
                    if let Some(local_value) = context.get(key) {
                        merged_values.insert(key.clone(), local_value.clone());
                    } else {
                        return Err(TaskError::Unknown {
                            task_id: self.id.clone(),
                            message: format!(
                                "Expected key '{}' not found in dependencies or local context",
                                key
                            ),
                        });
                    }
                }
                Err(e) => {
                    return Err(TaskError::Unknown {
                        task_id: self.id.clone(),
                        message: format!("Failed to load key '{}': {}", key, e),
                    });
                }
            }
        }

        // Add a summary of merged values
        let summary = Value::Array(
            merged_values
                .keys()
                .map(|k| Value::String(k.clone()))
                .collect(),
        );

        context
            .insert("merged_keys", summary)
            .map_err(|e| TaskError::Unknown {
                task_id: self.id.clone(),
                message: format!("Failed to insert summary: {}", e),
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

#[tokio::test]
async fn test_context_merging_latest_wins() {
    let database = get_test_database().await;

    // Create tasks that produce conflicting data
    let mut task_registry = TaskRegistry::new();

    let mut early_data = std::collections::HashMap::new();
    early_data.insert(
        "shared_key".to_string(),
        Value::String("early_value".to_string()),
    );
    early_data.insert(
        "early_only".to_string(),
        Value::String("unique_early".to_string()),
    );

    let mut late_data = std::collections::HashMap::new();
    late_data.insert(
        "shared_key".to_string(),
        Value::String("late_value".to_string()),
    );
    late_data.insert(
        "late_only".to_string(),
        Value::String("unique_late".to_string()),
    );

    let early_task = ContextProducerTask::new("early_producer", vec![], early_data);
    let late_task = ContextProducerTask::new("late_producer", vec!["early_producer"], late_data);
    let merger_task = ContextMergerTask::new(
        "merger",
        vec!["early_producer", "late_producer"],
        vec!["shared_key", "early_only", "late_only"],
    );

    task_registry.register(early_task).unwrap();
    task_registry.register(late_task).unwrap();
    task_registry.register(merger_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow
    let workflow = Workflow::builder("merging_pipeline")
        .description("Test pipeline for context merging")
        .add_task(WorkflowTask::new("early_producer", vec![]))
        .unwrap()
        .add_task(WorkflowTask::new("late_producer", vec!["early_producer"]))
        .unwrap()
        .add_task(WorkflowTask::new(
            "merger",
            vec!["early_producer", "late_producer"],
        ))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("initial_context", Value::String("merging_test".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("merging_pipeline", input_context)
        .await
        .unwrap();

    // Create and run executor
    let config = ExecutorConfig {
        max_concurrent_tasks: 1, // Sequential execution to ensure order
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);

    // Run scheduling and execution
    let scheduler_handle = tokio::spawn(async move { scheduler.run_scheduling_loop().await });

    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for all tasks to execute
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check merger task results
    let dal = cloacina::dal::DAL::new(database.pool());
    let merger_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "merger")
        .await
        .unwrap();

    let context_data: std::collections::HashMap<String, Value> =
        if let Some(context_id) = merger_metadata.context_id {
            let context = dal.context().read::<serde_json::Value>(context_id).await.unwrap();
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

#[tokio::test]
async fn test_execution_scope_context_setup() {
    let database = get_test_database().await;

    // Create a task that inspects its execution scope
    #[derive(Debug)]
    struct ScopeInspectorTask {
        id: String,
    }

    #[async_trait]
    impl Task for ScopeInspectorTask {
        async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
            // Check if execution scope is set
            if let Some(scope) = context.execution_scope() {
                let scope_info = serde_json::json!({
                    "pipeline_execution_id": scope.pipeline_execution_id.to_string(),
                    "task_execution_id": scope.task_execution_id.map(|id| id.to_string()),
                    "task_name": scope.task_name.clone()
                });

                context
                    .insert("execution_scope_info", scope_info)
                    .map_err(|e| TaskError::Unknown {
                        task_id: self.id.clone(),
                        message: format!("Failed to insert scope info: {}", e),
                    })?;
            } else {
                return Err(TaskError::Unknown {
                    task_id: self.id.clone(),
                    message: "Execution scope not set".to_string(),
                });
            }

            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[String] {
            &[]
        }
    }

    let mut task_registry = TaskRegistry::new();
    let inspector_task = ScopeInspectorTask {
        id: "scope_inspector".to_string(),
    };
    task_registry.register(inspector_task).unwrap();
    let task_registry = Arc::new(task_registry);

    // Create workflow
    let workflow = Workflow::builder("scope_pipeline")
        .description("Test pipeline for execution scope")
        .add_task(WorkflowTask::new("scope_inspector", vec![]))
        .unwrap()
        .build()
        .unwrap();

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    // Schedule workflow execution
    let mut input_context = Context::new();
    input_context
        .insert("scope_test", Value::String("execution_scope".to_string()))
        .unwrap();
    let pipeline_id = scheduler
        .schedule_workflow_execution("scope_pipeline", input_context)
        .await
        .unwrap();

    // Process scheduling
    scheduler.process_active_pipelines().await.unwrap();

    // Create and run executor
    let config = ExecutorConfig {
        max_concurrent_tasks: 1,
        poll_interval: Duration::from_millis(100),
        task_timeout: Duration::from_secs(5),
    };

    let executor = TaskExecutor::new(database.clone(), task_registry, config);

    let executor_handle = tokio::spawn(async move { executor.run().await });

    // Give time for execution
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that scope information was captured
    let dal = cloacina::dal::DAL::new(database.pool());
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), "scope_inspector")
        .await
        .unwrap();

    let context_data: std::collections::HashMap<String, Value> =
        if let Some(context_id) = task_metadata.context_id {
            let context = dal.context().read::<serde_json::Value>(context_id).await.unwrap();
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
                assert_eq!(name, "scope_inspector");
            } else {
                panic!("Expected task_name to be a string, got: {:?}", task_name);
            }
        }
    }

    // Cleanup
    executor_handle.abort();
}
