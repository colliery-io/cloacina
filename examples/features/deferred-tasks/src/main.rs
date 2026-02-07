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

//! # Deferred Tasks Example
//!
//! This example demonstrates how to use `TaskHandle` and `defer_until` to
//! release a concurrency slot while waiting for an external condition, then
//! resume execution once the condition is met.
//!
//! ## Key Concepts
//!
//! - **TaskHandle**: An optional second parameter on `#[task]` functions that
//!   provides execution control capabilities.
//! - **defer_until**: Releases the task's concurrency slot, polls a condition
//!   at a given interval, and reclaims a slot when the condition returns `true`.
//! - **Concurrency Slot Management**: While deferred, other tasks can use the
//!   freed slot. The async future stays parked in the tokio runtime consuming
//!   minimal resources.
//!
//! ## Workflow
//!
//! ```text
//! wait_for_data ──► process_data
//!     │
//!     └─ defers until simulated external data is "ready"
//! ```

use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError, TaskHandle};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Simulates waiting for external data to become available.
///
/// This task uses `defer_until` to release its concurrency slot while polling
/// for a condition. In a real application, the condition might check:
/// - Whether a file has appeared on disk
/// - Whether an API endpoint returns a ready status
/// - Whether a message has arrived in a queue
#[task(id = "wait_for_data", dependencies = [])]
async fn wait_for_data(
    context: &mut Context<serde_json::Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    info!("wait_for_data: Starting — will defer until data is ready");

    // Simulate an external readiness check.
    // In production this would call an API, check a file, etc.
    let poll_count = Arc::new(AtomicUsize::new(0));
    let pc = poll_count.clone();

    handle
        .defer_until(
            move || {
                let pc = pc.clone();
                async move {
                    let n = pc.fetch_add(1, Ordering::SeqCst);
                    info!("wait_for_data: polling external source (attempt {})", n + 1);
                    // Simulate: data becomes ready after 3 polls
                    n >= 2
                }
            },
            Duration::from_millis(500),
        )
        .await
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("defer_until failed: {e}"),
            task_id: "wait_for_data".into(),
            timestamp: chrono::Utc::now(),
        })?;

    info!(
        "wait_for_data: Data is ready after {} polls — slot reclaimed",
        poll_count.load(Ordering::SeqCst)
    );

    // Write the "received" data into context for downstream tasks
    context.insert("external_data", json!({"status": "ready", "records": 42}))?;
    Ok(())
}

/// Processes data that was fetched by the deferred task.
#[task(id = "process_data", dependencies = ["wait_for_data"])]
async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let data = context
        .get("external_data")
        .ok_or_else(|| TaskError::ExecutionFailed {
            message: "external_data not found in context".into(),
            task_id: "process_data".into(),
            timestamp: chrono::Utc::now(),
        })?
        .clone();

    info!("process_data: Processing external data: {}", data);

    let records = data.get("records").and_then(|v| v.as_u64()).unwrap_or(0);

    context.insert("processed_count", json!(records))?;
    context.insert("processing_complete", json!(true))?;

    info!("process_data: Processed {} records", records);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("deferred_tasks=info,cloacina=info")
        .init();

    info!("=== Deferred Tasks Example ===");
    info!("Demonstrates TaskHandle::defer_until for concurrency slot management");

    let runner = DefaultRunner::with_config(
        "sqlite://deferred-tasks.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    let _workflow = workflow! {
        name: "deferred_pipeline",
        description: "Pipeline demonstrating deferred task execution",
        tasks: [wait_for_data, process_data]
    };

    info!("Executing deferred_pipeline...");

    let result = runner.execute("deferred_pipeline", Context::new()).await?;

    info!("=== Pipeline Complete ===");
    info!("Status: {:?}", result.status);

    if let Some(count) = result.final_context.get("processed_count") {
        info!("Processed {} records", count);
    }
    if let Some(complete) = result.final_context.get("processing_complete") {
        info!("Processing complete: {}", complete);
    }

    runner.shutdown().await?;

    info!("=== Example Finished ===");
    Ok(())
}
