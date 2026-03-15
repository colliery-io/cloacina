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

//! Tests for the #[continuous_task] proc macro.

use cloacina::continuous_task;
use cloacina_workflow::{Context, Task, TaskError};
use serde_json::json;

// --- Basic continuous task ---

#[continuous_task(id = "aggregate_hourly", sources = ["raw_events"])]
async fn aggregate_hourly(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("aggregated", json!(true)).unwrap();
    Ok(())
}

#[test]
fn test_continuous_task_compiles_and_creates() {
    let task = aggregate_hourly_task();
    assert_eq!(task.id(), "aggregate_hourly");
}

#[test]
fn test_continuous_task_sources() {
    assert_eq!(AggregateHourlyTask::sources(), &["raw_events"]);
}

#[test]
fn test_continuous_task_referenced_empty() {
    assert_eq!(AggregateHourlyTask::referenced_sources(), &[] as &[&str]);
}

#[test]
fn test_continuous_task_is_continuous() {
    assert!(AggregateHourlyTask::is_continuous());
}

#[test]
fn test_continuous_task_has_fingerprint() {
    let fp = AggregateHourlyTask::code_fingerprint_value();
    assert!(!fp.is_empty());
}

#[tokio::test]
async fn test_continuous_task_execute_runs_function() {
    let task = aggregate_hourly_task();
    let ctx = Context::new();
    let result = task.execute(ctx).await;
    assert!(result.is_ok());
    let ctx = result.unwrap();
    assert_eq!(ctx.get("aggregated"), Some(&json!(true)));
}

// --- Continuous task with referenced sources ---

#[continuous_task(
    id = "join_metrics",
    sources = ["clicks", "impressions"],
    referenced = ["config"]
)]
async fn join_metrics(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("joined", json!(true)).unwrap();
    Ok(())
}

#[test]
fn test_continuous_task_multiple_sources() {
    assert_eq!(JoinMetricsTask::sources(), &["clicks", "impressions"]);
}

#[test]
fn test_continuous_task_with_referenced() {
    assert_eq!(JoinMetricsTask::referenced_sources(), &["config"]);
}

#[tokio::test]
async fn test_continuous_task_with_referenced_executes() {
    let task = join_metrics_task();
    let ctx = Context::new();
    let result = task.execute(ctx).await;
    assert!(result.is_ok());
}

// --- Continuous task that reads boundary from context ---

#[continuous_task(id = "process_boundaries", sources = ["events"])]
async fn process_boundaries(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // This is how a real continuous task would read the boundary
    if let Some(boundary) = context.get("__boundary") {
        context
            .insert("processed_boundary", boundary.clone())
            .unwrap();
    }
    Ok(())
}

#[tokio::test]
async fn test_continuous_task_reads_boundary_from_context() {
    let task = process_boundaries_task();
    let mut ctx = Context::new();
    ctx.insert(
        "__boundary",
        json!({"kind": {"type": "OffsetRange", "start": 0, "end": 100}}),
    )
    .unwrap();

    let result = task.execute(ctx).await.unwrap();
    assert!(result.get("processed_boundary").is_some());
}

// --- Continuous task that fails ---

#[continuous_task(id = "failing_task", sources = ["events"])]
async fn failing_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Err(TaskError::ExecutionFailed {
        message: "intentional failure".into(),
        task_id: "failing_task".into(),
        timestamp: chrono::Utc::now(),
    })
}

#[tokio::test]
async fn test_continuous_task_failure_propagates() {
    let task = failing_task_task();
    let ctx = Context::new();
    let result = task.execute(ctx).await;
    assert!(result.is_err());
}
