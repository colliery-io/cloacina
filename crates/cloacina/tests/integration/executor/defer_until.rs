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

//! Integration tests for the `defer_until` / TaskHandle feature.
//!
//! These tests verify that tasks using the `#[task]` macro with a `TaskHandle`
//! parameter can defer execution, release their concurrency slot, and resume
//! once a condition is met.

use cloacina::database::universal_types::UniversalUuid;
use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::*;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::fixtures::get_or_init_fixture;

// ---------------------------------------------------------------------------
// Task definitions using #[task] macro with TaskHandle parameter
// ---------------------------------------------------------------------------

/// A task that defers until an external flag is set, then writes to context.
#[task(id = "deferred_flag_task", dependencies = [])]
async fn deferred_flag_task(
    context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    // Read the flag address from context (set by the test harness).
    // In a real scenario the condition would check an external system.
    // For the integration test we simply poll a few times then succeed.
    let poll_count = Arc::new(AtomicUsize::new(0));
    let pc = poll_count.clone();

    handle
        .defer_until(
            move || {
                let pc = pc.clone();
                async move {
                    let n = pc.fetch_add(1, Ordering::SeqCst);
                    // Return true after 3 polls
                    n >= 2
                }
            },
            Duration::from_millis(10),
        )
        .await
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("defer_until failed: {e}"),
            task_id: "deferred_flag_task".into(),
            timestamp: chrono::Utc::now(),
        })?;

    context.insert(
        "deferred_result",
        Value::String("resumed_after_defer".into()),
    )?;
    context.insert(
        "poll_count",
        Value::Number(serde_json::Number::from(
            poll_count.load(Ordering::SeqCst) as u64
        )),
    )?;

    Ok(())
}

/// A simple task that runs after the deferred task to verify chaining works.
#[task(id = "after_deferred_task", dependencies = ["deferred_flag_task"])]
async fn after_deferred_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    if let Some(val) = context.get("deferred_result") {
        context.insert("chain_result", Value::String(format!("chained: {}", val)))?;
    }
    Ok(())
}

/// A task that defers with a longer interval so we can observe "Deferred" sub_status.
#[task(id = "slow_deferred_task", dependencies = [])]
async fn slow_deferred_task(
    context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    let poll_count = Arc::new(AtomicUsize::new(0));
    let pc = poll_count.clone();

    handle
        .defer_until(
            move || {
                let pc = pc.clone();
                async move {
                    let n = pc.fetch_add(1, Ordering::SeqCst);
                    // Need 5 polls at 200ms each = ~1s of deferral time
                    n >= 4
                }
            },
            Duration::from_millis(200),
        )
        .await
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("defer_until failed: {e}"),
            task_id: "slow_deferred_task".into(),
            timestamp: chrono::Utc::now(),
        })?;

    context.insert("slow_deferred_result", Value::String("completed".into()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: WorkflowTask for building workflows
// ---------------------------------------------------------------------------
use async_trait::async_trait;

#[derive(Debug)]
struct SimpleTask {
    id: String,
    dependencies: Vec<TaskNamespace>,
}

impl SimpleTask {
    fn new(id: &str, deps: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps
                .into_iter()
                .map(|s| TaskNamespace::from_string(s).unwrap())
                .collect(),
        }
    }

    /// Create a SimpleTask with dependencies specified as simple task names.
    /// Constructs full namespace using default tenant/package and the given workflow name.
    fn with_workflow(id: &str, deps: Vec<&str>, workflow_name: &str) -> Self {
        Self {
            id: id.to_string(),
            dependencies: deps
                .into_iter()
                .map(|dep| TaskNamespace::new("public", "embedded", workflow_name, dep))
                .collect(),
        }
    }
}

#[async_trait]
impl Task for SimpleTask {
    async fn execute(&self, context: Context<Value>) -> Result<Context<Value>, TaskError> {
        Ok(context)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Verifies that a task using `defer_until` via TaskHandle completes
/// successfully through the full executor pipeline.
#[tokio::test]
async fn test_defer_until_full_pipeline() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Build workflow with the deferred task
    let workflow = Workflow::builder("defer_pipeline")
        .description("Pipeline with deferred task")
        .add_task(Arc::new(SimpleTask::new("deferred_flag_task", vec![])))
        .unwrap()
        .build()
        .unwrap();

    // Register task constructor (the macro-generated task struct)
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "deferred_flag_task",
    );
    register_task_constructor(namespace, || Arc::new(deferred_flag_task_task()));

    // Register workflow
    register_workflow_constructor("defer_pipeline".to_string(), {
        let wf = workflow.clone();
        move || wf.clone()
    });

    // Create runner
    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    // Execute
    let input_context = Context::new();
    let execution = runner
        .execute_async("defer_pipeline", input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Wait for completion (defer_until polls every 10ms, needs 3 polls + overhead)
    time::sleep(Duration::from_millis(1000)).await;

    // Verify task completed
    let dal = cloacina::dal::DAL::new(database.clone());
    let task_executions = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
        .await
        .unwrap();

    assert_eq!(task_executions.len(), 1, "Expected 1 task execution");
    let task = &task_executions[0];
    assert_eq!(task.status, "Completed", "Deferred task should complete");
    // sub_status should be cleared after completion
    assert!(
        task.sub_status.is_none(),
        "sub_status should be None after completion, got: {:?}",
        task.sub_status
    );

    runner.shutdown().await.unwrap();
}

/// Verifies that a deferred task correctly chains with a downstream task.
#[tokio::test]
async fn test_defer_until_with_downstream_dependency() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    // Build workflow: deferred_flag_task -> after_deferred_task
    let workflow = Workflow::builder("defer_chain_pipeline")
        .description("Pipeline with deferred task and downstream dependency")
        .add_task(Arc::new(SimpleTask::new("deferred_flag_task", vec![])))
        .unwrap()
        .add_task(Arc::new(SimpleTask::with_workflow(
            "after_deferred_task",
            vec!["deferred_flag_task"],
            "defer_chain_pipeline",
        )))
        .unwrap()
        .build()
        .unwrap();

    // Register both task constructors
    let ns1 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "deferred_flag_task",
    );
    register_task_constructor(ns1, || Arc::new(deferred_flag_task_task()));

    let ns2 = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "after_deferred_task",
    );
    register_task_constructor(ns2, || Arc::new(after_deferred_task_task()));

    register_workflow_constructor("defer_chain_pipeline".to_string(), {
        let wf = workflow.clone();
        move || wf.clone()
    });

    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    let input_context = Context::new();
    let execution = runner
        .execute_async("defer_chain_pipeline", input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    // Wait for both tasks to complete
    time::sleep(Duration::from_millis(2000)).await;

    let dal = cloacina::dal::DAL::new(database.clone());
    let task_executions = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
        .await
        .unwrap();

    assert_eq!(task_executions.len(), 2, "Expected 2 task executions");

    // Both should be completed
    for task in &task_executions {
        assert_eq!(
            task.status, "Completed",
            "Task '{}' should be Completed, got '{}'",
            task.task_name, task.status
        );
    }

    runner.shutdown().await.unwrap();
}

/// Verifies that sub_status transitions through "Deferred" while the task is
/// waiting and is cleared back to None after completion.
#[tokio::test]
async fn test_sub_status_transitions_during_deferral() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    let workflow = Workflow::builder("sub_status_pipeline")
        .description("Pipeline for observing sub_status transitions")
        .add_task(Arc::new(SimpleTask::new("slow_deferred_task", vec![])))
        .unwrap()
        .build()
        .unwrap();

    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "slow_deferred_task",
    );
    register_task_constructor(namespace, || Arc::new(slow_deferred_task_task()));

    register_workflow_constructor("sub_status_pipeline".to_string(), {
        let wf = workflow.clone();
        move || wf.clone()
    });

    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .unwrap();

    let input_context = Context::new();
    let execution = runner
        .execute_async("sub_status_pipeline", input_context)
        .await
        .unwrap();
    let pipeline_id = execution.execution_id;

    let dal = cloacina::dal::DAL::new(database.clone());

    // Poll for the "Deferred" sub_status while the task is waiting.
    // The task defers for ~1s (5 polls × 200ms). We check every 100ms.
    let mut saw_deferred = false;
    for _ in 0..30 {
        time::sleep(Duration::from_millis(100)).await;
        let tasks = dal
            .task_execution()
            .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
            .await
            .unwrap();
        if let Some(task) = tasks.first() {
            if task.sub_status.as_deref() == Some("Deferred") {
                saw_deferred = true;
                break;
            }
        }
    }

    assert!(
        saw_deferred,
        "Should have observed sub_status='Deferred' during deferral"
    );

    // Wait for completion
    time::sleep(Duration::from_millis(2000)).await;

    let tasks = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(pipeline_id))
        .await
        .unwrap();

    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(task.status, "Completed");
    assert!(
        task.sub_status.is_none(),
        "sub_status should be None after completion, got: {:?}",
        task.sub_status
    );

    runner.shutdown().await.unwrap();
}
