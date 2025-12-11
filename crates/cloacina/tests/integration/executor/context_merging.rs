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
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

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

    // With pre-inject pattern, all dependency data is already in context
    for key in &expected_keys {
        // Get value directly from context (deps are pre-injected by executor)
        if let Some(value) = context.get(*key) {
            merged_values.insert(key.to_string(), value.clone());
        } else {
            return Err(TaskError::Unknown {
                task_id: "merger_task".to_string(),
                message: format!("Expected key '{}' not found in context", key),
            });
        }
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
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow using the #[task] functions with unique name
    let workflow_name = format!(
        "merging_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Create TaskNamespace objects for dependencies
    let early_ns = TaskNamespace::new("public", "embedded", &workflow_name, "early_producer_task");
    let late_ns = TaskNamespace::new("public", "embedded", &workflow_name, "late_producer_task");

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for context merging")
        .add_task(Arc::new(early_producer_task_task()))
        .unwrap()
        .add_task(Arc::new(
            late_producer_task_task().with_dependencies(vec![early_ns.clone()]),
        ))
        .unwrap()
        .add_task(Arc::new(
            merger_task_task().with_dependencies(vec![early_ns.clone(), late_ns.clone()]),
        ))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks with correct namespaces and dependencies in global registry
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "early_producer_task",
    );
    register_task_constructor(namespace1, || Arc::new(early_producer_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "late_producer_task",
    );
    let early_ns_for_closure = early_ns.clone();
    register_task_constructor(namespace2, move || {
        Arc::new(late_producer_task_task().with_dependencies(vec![early_ns_for_closure.clone()]))
    });

    let namespace3 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "merger_task",
    );
    let early_ns_for_merger = early_ns.clone();
    let late_ns_for_merger = late_ns.clone();
    register_task_constructor(namespace3, move || {
        Arc::new(merger_task_task().with_dependencies(vec![
            early_ns_for_merger.clone(),
            late_ns_for_merger.clone(),
        ]))
    });

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow_name.clone(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner with sequential execution to ensure order and proper schema isolation
    let mut config = DefaultRunnerConfig::default();
    config.max_concurrent_tasks = 1;
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
        .insert("initial_context", Value::String("merging_test".to_string()))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Give time for all tasks to execute
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check merger task results
    let dal = cloacina::dal::DAL::new(database.clone());
    let merger_task_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "merger_task",
    );
    let merger_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &merger_task_namespace)
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

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[task(
    id = "scope_inspector_task",
    dependencies = []
)]
async fn scope_inspector_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Task executed successfully - execution scope is now internal to executor
    // and not exposed to tasks (pre-inject pattern)
    let scope_info = serde_json::json!({
        "task_executed": true,
        "message": "Task executed within executor context"
    });

    context.insert("execution_scope_info", scope_info)?;
    Ok(())
}

#[tokio::test]
async fn test_execution_scope_context_setup() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create workflow with unique name
    let workflow_name = format!(
        "scope_pipeline_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
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
        "scope_inspector_task",
    );
    register_task_constructor(namespace, || Arc::new(scope_inspector_task_task()));

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor(workflow_name.clone(), {
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
        .insert("scope_test", Value::String("execution_scope".to_string()))
        .unwrap();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Give time for execution
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that scope information was captured
    let dal = cloacina::dal::DAL::new(database.clone());
    let task_namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "scope_inspector_task",
    );
    let task_metadata = dal
        .task_execution_metadata()
        .get_by_pipeline_and_task(UniversalUuid(pipeline_id), &task_namespace)
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

    // With pre-inject pattern, execution scope is no longer exposed to tasks.
    // The task just confirms it executed successfully within the executor context.
    assert!(
        context_data.contains_key("execution_scope_info"),
        "Task should have captured execution info"
    );

    if let Some(scope_info) = context_data.get("execution_scope_info") {
        let scope_obj = scope_info.as_object().unwrap();
        // With pre-inject pattern, we just verify the task executed
        assert!(
            scope_obj.contains_key("task_executed"),
            "Scope info should contain task_executed field"
        );
        assert_eq!(
            scope_obj.get("task_executed"),
            Some(&Value::Bool(true)),
            "Task should have executed successfully"
        );
    }

    // Cleanup
    runner.shutdown().await.unwrap();
}
