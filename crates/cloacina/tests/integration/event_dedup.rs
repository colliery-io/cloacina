/*
 *  Copyright 2026 Colliery Software
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

//! Verify that task completion produces exactly one TaskCompleted event per task
//! and one PipelineCompleted event per pipeline (no duplicates from the dispatcher).
//!
//! Regression test for CLOACI-T-0474: the dispatcher previously called
//! mark_completed/mark_failed in addition to the executor, producing duplicate
//! execution events.

use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;

#[workflow(
    name = "event_dedup_test_workflow",
    description = "Two-task workflow for event dedup testing",
    author = "Test"
)]
pub mod event_dedup_test_workflow {
    use super::*;

    #[task(id = "first", dependencies = [])]
    pub async fn first(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("first_done", json!(true))?;
        Ok(())
    }

    #[task(id = "second", dependencies = ["first"])]
    pub async fn second(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("second_done", json!(true))?;
        Ok(())
    }
}

/// Execute a 2-task workflow and verify exactly one TaskCompleted event per task
/// and exactly one PipelineCompleted event for the pipeline.
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_no_duplicate_completion_events() {
    let db_url = ":memory:";
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build();

    let runner = DefaultRunner::with_config(db_url, config)
        .await
        .expect("Failed to create runner");

    let database = runner.database().clone();

    let context = Context::new();
    let result = runner
        .execute("event_dedup_test_workflow", context)
        .await
        .expect("Workflow execution failed");

    // Verify workflow completed successfully
    assert_eq!(
        result.final_context.get("first_done"),
        Some(&json!(true)),
        "first task should have completed"
    );
    assert_eq!(
        result.final_context.get("second_done"),
        Some(&json!(true)),
        "second task should have completed"
    );

    // Query execution events for this pipeline
    let dal = DAL::new(database);
    let events = dal
        .execution_event()
        .list_by_pipeline(UniversalUuid(result.execution_id))
        .await
        .expect("Failed to list events");

    // Count events by type
    let task_completed_count = events
        .iter()
        .filter(|e| e.event_type == "task_completed")
        .count();
    let pipeline_completed_count = events
        .iter()
        .filter(|e| e.event_type == "pipeline_completed")
        .count();

    // Must have exactly 2 TaskCompleted (one per task) — not 4 (no dispatcher duplicates)
    assert_eq!(
        task_completed_count, 2,
        "Expected exactly 2 TaskCompleted events (one per task), got {}. \
         Duplicate events indicate the dispatcher is still calling mark_completed.",
        task_completed_count
    );

    // Must have exactly 1 PipelineCompleted — not 2 (no race duplicates)
    assert_eq!(
        pipeline_completed_count, 1,
        "Expected exactly 1 PipelineCompleted event, got {}. \
         Duplicate events indicate a pipeline completion race condition.",
        pipeline_completed_count
    );

    runner.shutdown().await.expect("Shutdown failed");
}
