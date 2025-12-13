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

//! Integration tests for workflow pause/resume functionality.

use async_trait::async_trait;
use cloacina::executor::pipeline_executor::PipelineStatus;
use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
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
    id = "quick_task",
    dependencies = []
)]
async fn quick_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    context.insert("quick_result", Value::String("done".to_string()))?;
    Ok(())
}

#[task(
    id = "slow_first_task",
    dependencies = []
)]
async fn slow_first_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Simulate a slow task that takes a few seconds
    time::sleep(Duration::from_secs(2)).await;
    context.insert("slow_first_result", Value::String("completed".to_string()))?;
    Ok(())
}

#[task(
    id = "slow_second_task",
    dependencies = ["slow_first_task"]
)]
async fn slow_second_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Simulate another slow task
    time::sleep(Duration::from_secs(2)).await;
    context.insert("slow_second_result", Value::String("completed".to_string()))?;
    Ok(())
}

#[tokio::test]
async fn test_pause_running_pipeline() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create a workflow with slow tasks to give us time to pause
    let workflow_name = format!(
        "pause_test_pipeline_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let first_ns = TaskNamespace::new("public", "embedded", &workflow_name, "slow_first_task");

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for pause/resume")
        .add_task(Arc::new(slow_first_task_task()))
        .unwrap()
        .add_task(Arc::new(
            slow_second_task_task().with_dependencies(vec![first_ns.clone()]),
        ))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_first_task",
    );
    register_task_constructor(namespace1, || Arc::new(slow_first_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_second_task",
    );
    let first_ns_clone = first_ns.clone();
    register_task_constructor(namespace2, move || {
        Arc::new(slow_second_task_task().with_dependencies(vec![first_ns_clone.clone()]))
    });

    // Register workflow
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Start execution
    let input_context = Context::new();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Wait a moment for scheduler to pick up the pipeline
    // Note: Pipelines stay in "Pending" status while tasks execute, so we just wait briefly
    time::sleep(Duration::from_millis(200)).await;

    // Pause the pipeline (works on both Pending and Running status)
    execution.pause(Some("Test pause")).await.unwrap();

    // Verify the pipeline is paused
    let status = execution.get_status().await.unwrap();
    assert_eq!(status, PipelineStatus::Paused, "Pipeline should be paused");

    // Verify via DAL that pause metadata is set
    let dal = cloacina::dal::DAL::new(database.clone());
    let pipeline = dal
        .pipeline_execution()
        .get_by_id(UniversalUuid(pipeline_id))
        .await
        .unwrap();
    assert_eq!(pipeline.status, "Paused");
    assert!(pipeline.paused_at.is_some(), "paused_at should be set");
    assert_eq!(
        pipeline.pause_reason,
        Some("Test pause".to_string()),
        "pause_reason should be set"
    );

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_resume_paused_pipeline() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Create a workflow with slow tasks to give us time to pause and resume
    let workflow_name = format!(
        "resume_test_pipeline_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let first_ns = TaskNamespace::new("public", "embedded", &workflow_name, "slow_first_task");

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for resume")
        .add_task(Arc::new(slow_first_task_task()))
        .unwrap()
        .add_task(Arc::new(
            slow_second_task_task().with_dependencies(vec![first_ns.clone()]),
        ))
        .unwrap()
        .build()
        .unwrap();

    // Register tasks
    let namespace1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_first_task",
    );
    register_task_constructor(namespace1, || Arc::new(slow_first_task_task()));

    let namespace2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_second_task",
    );
    let first_ns_clone = first_ns.clone();
    register_task_constructor(namespace2, move || {
        Arc::new(slow_second_task_task().with_dependencies(vec![first_ns_clone.clone()]))
    });

    // Register workflow
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Start execution
    let input_context = Context::new();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Wait a moment for scheduler to pick up the pipeline
    time::sleep(Duration::from_millis(200)).await;

    // Pause the pipeline
    execution.pause(None).await.unwrap();
    let status = execution.get_status().await.unwrap();
    assert_eq!(status, PipelineStatus::Paused);

    // Resume the pipeline
    execution.resume().await.unwrap();

    // Verify the pipeline is active again (either Pending or Running)
    // Note: Resume sets status back to "Running" but the scheduler may not have
    // picked it up yet, or it may have already processed tasks
    let status = execution.get_status().await.unwrap();
    assert!(
        status == PipelineStatus::Running || status == PipelineStatus::Pending,
        "Pipeline should be active after resume, got {:?}",
        status
    );

    // Verify via DAL that pause metadata is cleared
    let dal = cloacina::dal::DAL::new(database.clone());
    let pipeline = dal
        .pipeline_execution()
        .get_by_id(UniversalUuid(pipeline_id))
        .await
        .unwrap();
    // Resume sets status to "Running"
    assert_eq!(pipeline.status, "Running");
    assert!(
        pipeline.paused_at.is_none(),
        "paused_at should be cleared after resume"
    );
    assert!(
        pipeline.pause_reason.is_none(),
        "pause_reason should be cleared after resume"
    );

    // Wait for completion
    time::sleep(Duration::from_secs(1)).await;

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_pause_non_running_pipeline_fails() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();

    // Create a simple workflow
    let workflow_name = format!(
        "pause_fail_test_pipeline_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for pause failure")
        .add_task(Arc::new(quick_task_task()))
        .unwrap()
        .build()
        .unwrap();

    // Register task
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "quick_task",
    );
    register_task_constructor(namespace, || Arc::new(quick_task_task()));

    // Register workflow
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Start execution
    let input_context = Context::new();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();

    // Wait for pipeline to complete
    time::sleep(Duration::from_secs(2)).await;

    // Try to pause a completed pipeline - should fail
    let result = execution.pause(None).await;
    assert!(result.is_err(), "Pausing a completed pipeline should fail");

    // Cleanup
    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_resume_non_paused_pipeline_fails() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();

    // Create a simple workflow
    let workflow_name = format!(
        "resume_fail_test_pipeline_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let workflow = Workflow::builder(&workflow_name)
        .description("Test pipeline for resume failure")
        .add_task(Arc::new(slow_first_task_task()))
        .unwrap()
        .build()
        .unwrap();

    // Register task
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_first_task",
    );
    register_task_constructor(namespace, || Arc::new(slow_first_task_task()));

    // Register workflow
    register_workflow_constructor(workflow.name().to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // Create runner
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Start execution
    let input_context = Context::new();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();

    // Wait for pipeline to start running
    time::sleep(Duration::from_millis(300)).await;

    // Try to resume a running pipeline (not paused) - should fail
    let result = execution.resume().await;
    assert!(
        result.is_err(),
        "Resuming a running (not paused) pipeline should fail"
    );

    // Cleanup
    runner.shutdown().await.unwrap();
}
