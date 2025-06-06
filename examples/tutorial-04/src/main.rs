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

//! # Tutorial 04: Error Handling and Retries
//!
//! This tutorial demonstrates error handling and retry patterns in Cloacina:
//! - Basic retry policies with exponential backoff
//! - Fallback strategies when external dependencies fail
//! - Different approaches to handling task failures
//! - Monitoring task execution outcomes

use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, TaskError};
use rand::Rng;
use serde_json::json;
use std::time::Duration;
use tracing::{error, info, warn};

// Task 1: Fetch data from external source with retries
#[task(
    id = "fetch_data",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 1000,
    retry_backoff = "exponential"
)]
async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Attempting to fetch data from external source");

    // Simulate network call that might fail (70% failure rate)
    let success_rate = 0.3;
    let random_value: f64 = rand::random();

    if random_value > success_rate {
        error!("External source temporarily unavailable - will retry");
        return Err(TaskError::ExecutionFailed {
            message: "External source temporarily unavailable".to_string(),
            task_id: "fetch_data".to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    // Simulate API response delay
    tokio::time::sleep(Duration::from_millis(500)).await;

    let data = json!({
        "records": [
            {"id": 1, "value": "data_1", "quality": 85},
            {"id": 2, "value": "data_2", "quality": 75},
            {"id": 3, "value": "data_3", "quality": 90}
        ],
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "source": "external",
        "total_records": 3
    });

    context.insert("raw_data", data)?;
    info!("Successfully fetched data from external source");
    Ok(())
}

// Task 2: Fallback to cached data when fetch fails
#[task(
    id = "cached_data",
    dependencies = ["fetch_data"],
    trigger_rules = task_failed("fetch_data")
)]
async fn cached_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Using cached data as fallback");

    // Simulate loading from cache
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cached_data = json!({
        "records": [
            {"id": 1, "value": "cached_1", "quality": 60},
            {"id": 2, "value": "cached_2", "quality": 65},
            {"id": 3, "value": "cached_3", "quality": 70}
        ],
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "source": "cache",
        "total_records": 3
    });

    context.insert("raw_data", cached_data)?;
    info!("Successfully loaded cached data");
    Ok(())
}

// Task 3: Process the data and evaluate quality
#[task(
    id = "process_data",
    dependencies = ["fetch_data", "cached_data"]
)]
async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Processing data");

    let raw_data = context
        .get("raw_data")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing raw_data".to_string(),
        })?;

    // Extract all needed data before any mutable operations
    let source = raw_data["source"].as_str().unwrap_or("unknown").to_string();
    let records = raw_data["records"].as_array().unwrap();
    let total_quality: i32 = records
        .iter()
        .map(|r| r["quality"].as_i64().unwrap_or(0) as i32)
        .sum();
    let avg_quality = total_quality / records.len() as i32;

    // Now we can safely make mutable borrows of context
    context.insert("data_quality_score", json!(avg_quality))?;
    context.insert(
        "processed_data",
        json!({
            "source": source,
            "quality_score": avg_quality,
            "processed_at": chrono::Utc::now().to_rfc3339()
        }),
    )?;

    info!(
        "Data processing completed with quality score: {}",
        avg_quality
    );
    Ok(())
}

// Task 4: High quality processing path
#[task(
    id = "high_quality_processing",
    dependencies = ["process_data"],
    trigger_rules = all(
        task_success("process_data"),
        context_value("data_quality_score", greater_than, 80)
    )
)]
async fn high_quality_processing(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    info!("Processing high quality data");

    let processed_data =
        context
            .get("processed_data")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Missing processed_data".to_string(),
            })?;

    let quality_score = processed_data["quality_score"].as_i64().unwrap_or(0);

    // Simulate premium processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    context.insert(
        "processing_result",
        json!({
            "path": "high_quality",
            "quality_score": quality_score,
            "processing_time_ms": 300,
            "enhancements_applied": ["advanced_validation", "premium_processing"]
        }),
    )?;

    info!("High quality processing completed");
    Ok(())
}

// Task 5: Low quality processing path
#[task(
    id = "low_quality_processing",
    dependencies = ["process_data"],
    trigger_rules = all(
        task_success("process_data"),
        context_value("data_quality_score", less_than, 81)
    )
)]
async fn low_quality_processing(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Processing low quality data");

    let processed_data =
        context
            .get("processed_data")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Missing processed_data".to_string(),
            })?;

    let quality_score = processed_data["quality_score"].as_i64().unwrap_or(0);

    // Simulate basic processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    context.insert(
        "processing_result",
        json!({
            "path": "low_quality",
            "quality_score": quality_score,
            "processing_time_ms": 100,
            "enhancements_applied": ["basic_validation"]
        }),
    )?;

    info!("Low quality processing completed");
    Ok(())
}

// Task 6: Failure notification
#[task(
    id = "failure_notification",
    dependencies = ["fetch_data", "cached_data"],
    trigger_rules = all(
        task_failed("fetch_data"),
        task_failed("cached_data")
    )
)]
async fn failure_notification(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    error!("Critical failure: Both fetch and cache operations failed");

    context.insert(
        "failure_notification",
        json!({
            "status": "critical_failure",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "message": "Both data sources failed",
            "alert_level": "high"
        }),
    )?;

    Ok(())
}

// Task 7: Final report generation
#[task(
    id = "final_report",
    dependencies = ["high_quality_processing", "low_quality_processing"],
    trigger_rules = any(
        task_success("high_quality_processing"),
        task_success("low_quality_processing")
    )
)]
async fn final_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Generating final execution report");

    let processing_result = context.get("processing_result");
    let processed_data = context.get("processed_data");
    let failure_notification = context.get("failure_notification");

    let report = json!({
        "execution_summary": {
            "status": if failure_notification.is_some() { "failed" } else { "success" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data_source": processed_data.and_then(|d| d["source"].as_str()).unwrap_or("unknown"),
            "quality_score": processed_data.and_then(|d| d["quality_score"].as_i64()).unwrap_or(0),
            "processing_path": processing_result.and_then(|r| r["path"].as_str()).unwrap_or("unknown"),
            "failure_details": failure_notification
        }
    });

    context.insert("execution_report", report)?;
    info!("Final report generated successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_04=info,cloacina=info")
        .init();

    info!("Starting Tutorial 04: Error Handling and Retries");
    info!("This demonstrates retry policies, fallback strategies, and resilient workflows");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let runner = DefaultRunner::new(
        "tutorial-04.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
    )
    .await?;

    // Create a workflow with our tasks
    let _workflow = workflow! {
        name: "resilient_pipeline",
        description: "Pipeline demonstrating error handling patterns and retry mechanisms",
        tasks: [
            fetch_data,
            cached_data,
            process_data,
            high_quality_processing,
            low_quality_processing,
            failure_notification,
            final_report
        ]
    };

    // Create input context
    let input_context = Context::new();

    // Execute the workflow
    info!("Executing resilient pipeline with error handling...");
    let result = runner
        .execute("resilient_pipeline", input_context)
        .await?;

    info!("Pipeline completed with status: {:?}", result.status);
    info!("Total execution time: {:?}", result.duration);
    info!("Tasks executed: {}", result.task_results.len());

    // Show detailed execution results including retry information
    for task_result in &result.task_results {
        let (status_icon, display_status) = match &task_result.status {
            cloacina::task::TaskState::Completed { .. } => ("✅", "Completed"),
            cloacina::task::TaskState::Failed { .. } => ("❌", "Failed"),
            cloacina::task::TaskState::Pending => ("⏳", "Pending"),
            cloacina::task::TaskState::Running { .. } => ("🔄", "Running"),
            cloacina::task::TaskState::Skipped { .. } => ("⏭️", "Skipped"),
        };

        let attempts_text = if task_result.attempt_count > 1 {
            format!(" (retried {} times)", task_result.attempt_count - 1)
        } else {
            String::new()
        };

        info!(
            "  {} Task '{}': {}{}",
            status_icon, task_result.task_name, display_status, attempts_text
        );

        // Display error or skip reason based on task state
        match &task_result.status {
            cloacina::task::TaskState::Failed { error, .. } => {
                info!("    Error: {}", error);
            }
            cloacina::task::TaskState::Skipped { reason, .. } => {
                info!("    Reason: {}", reason);
            }
            _ => {
                if let Some(error) = &task_result.error_message {
                    info!("    Error: {}", error);
                }
            }
        }
    }

    Ok(())
}
