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

//! # Event Triggers Example
//!
//! This example demonstrates how to use Cloacina's event trigger system to
//! automatically execute workflows based on custom conditions. Unlike cron
//! scheduling (time-based), event triggers poll user-defined functions and
//! fire workflows when conditions are met.
//!
//! ## Key Concepts
//!
//! - **Triggers**: User-defined polling functions that return `TriggerResult`
//! - **Poll Interval**: How often each trigger checks its condition
//! - **Context Passing**: Triggers can pass context data to workflows
//! - **Deduplication**: Prevents duplicate executions based on context hash
//!
//! ## Triggers Demonstrated
//!
//! 1. **File Watcher** - Polls for new files and triggers processing workflow
//! 2. **Queue Monitor** - Fires when queue depth exceeds threshold
//! 3. **Health Check** - Triggers recovery workflow on service failures
//!
//! ## Running the Example
//!
//! ```bash
//! cd examples/features/event-triggers
//! cargo run
//! ```

use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use std::time::Duration;
use tracing::info;

// Triggers live in their own module. Each one is decorated with
// `#[trigger]`, which submits a `TriggerEntry` to the inventory crate at
// compile time; `Runtime::new()` (inside `DefaultRunner`) seeds itself
// from that inventory so no explicit `register_trigger()` call is needed.
mod triggers;

// ============================================================================
// File Processing Workflow
// ============================================================================

#[workflow(
    name = "file_processing_workflow",
    description = "Process incoming data files"
)]
pub mod file_processing_workflow {
    use super::*;

    /// Validates and parses an incoming file.
    #[task]
    pub async fn validate_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Clone string value to avoid borrow issues
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Validating file: {}", filename);

        // Simulate validation
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        context.insert("validated", serde_json::json!(true))?;
        context.insert("file_size_bytes", serde_json::json!(1024))?;
        context.insert("record_count", serde_json::json!(100))?;

        info!("File '{}' validated successfully", filename);
        Ok(())
    }

    /// Processes the validated file data.
    #[task(dependencies = ["validate_file"])]
    pub async fn process_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let record_count = context
            .get("record_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!("Processing {} records from '{}'", record_count, filename);

        // Simulate processing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        context.insert("processed_records", serde_json::json!(record_count))?;
        context.insert("processing_status", serde_json::json!("completed"))?;

        info!("Successfully processed {} records", record_count);
        Ok(())
    }

    /// Archives the processed file.
    #[task(dependencies = ["process_file"])]
    pub async fn archive_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Archiving file: {}", filename);

        // Simulate archiving
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let archive_path = format!("/data/archive/{}", filename);
        context.insert("archive_path", serde_json::json!(archive_path.clone()))?;

        info!("File archived to: {}", archive_path);
        Ok(())
    }
}

// ============================================================================
// Queue Processing Workflow
// ============================================================================

#[workflow(
    name = "queue_processing_workflow",
    description = "Process messages when queue backs up"
)]
pub mod queue_processing_workflow {
    use super::*;

    /// Drains messages from the queue.
    #[task]
    pub async fn drain_queue(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let queue_name = context
            .get("queue_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let queue_depth = context
            .get("queue_depth")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!(
            "Draining {} messages from queue '{}'",
            queue_depth, queue_name
        );

        // Simulate draining
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        context.insert("messages_drained", serde_json::json!(queue_depth))?;

        info!("Drained {} messages from '{}'", queue_depth, queue_name);
        Ok(())
    }

    /// Processes the drained messages.
    #[task(dependencies = ["drain_queue"])]
    pub async fn process_messages(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let messages_drained = context
            .get("messages_drained")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!("Processing {} messages", messages_drained);

        // Simulate processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        context.insert("messages_processed", serde_json::json!(messages_drained))?;
        context.insert("processing_complete", serde_json::json!(true))?;

        info!("Processed {} messages successfully", messages_drained);
        Ok(())
    }

    /// Acknowledges processed messages.
    #[task(dependencies = ["process_messages"])]
    pub async fn ack_messages(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let messages_processed = context
            .get("messages_processed")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!("Acknowledging {} messages", messages_processed);

        // Simulate acknowledgment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        context.insert(
            "messages_acknowledged",
            serde_json::json!(messages_processed),
        )?;

        info!("Acknowledged {} messages", messages_processed);
        Ok(())
    }
}

// ============================================================================
// Service Recovery Workflow
// ============================================================================

#[workflow(
    name = "service_recovery_workflow",
    description = "Recover failed services automatically"
)]
pub mod service_recovery_workflow {
    use super::*;

    /// Diagnoses the service failure.
    #[task]
    pub async fn diagnose_failure(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let service_name = context
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let failures = context
            .get("consecutive_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!(
            "Diagnosing failure for service '{}' ({} consecutive failures)",
            service_name, failures
        );

        // Simulate diagnosis
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        context.insert("diagnosis", serde_json::json!("connection_timeout"))?;
        context.insert("severity", serde_json::json!("medium"))?;

        info!("Diagnosis complete: connection_timeout (severity: medium)");
        Ok(())
    }

    /// Attempts to restart the service.
    #[task(dependencies = ["diagnose_failure"])]
    pub async fn restart_service(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let service_name = context
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Attempting to restart service '{}'", service_name);

        // Simulate restart
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        context.insert("restart_attempted", serde_json::json!(true))?;
        context.insert("restart_success", serde_json::json!(true))?;

        info!("Service '{}' restarted successfully", service_name);
        Ok(())
    }

    /// Verifies service health after restart.
    #[task(dependencies = ["restart_service"])]
    pub async fn verify_recovery(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let service_name = context
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Verifying recovery of service '{}'", service_name);

        // Simulate verification
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        context.insert("recovery_verified", serde_json::json!(true))?;
        context.insert("health_status", serde_json::json!("healthy"))?;

        info!(
            "Service '{}' recovery verified - status: healthy",
            service_name
        );
        Ok(())
    }

    /// Sends notification about the incident.
    #[task(dependencies = ["verify_recovery"])]
    pub async fn notify_incident(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let service_name = context
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let health_status = context
            .get("health_status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!(
            "Sending incident notification for '{}' (current status: {})",
            service_name, health_status
        );

        // Simulate notification
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        context.insert("notification_sent", serde_json::json!(true))?;

        info!("Incident notification sent for '{}'", service_name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("event_triggers_example=info,cloacina=info")
        .init();

    info!("Starting Event Triggers Example");
    info!("================================");

    // Create DefaultRunner with trigger scheduling enabled
    let config = DefaultRunnerConfig::builder()
        .enable_trigger_scheduling(true)
        .trigger_base_poll_interval(Duration::from_secs(1))
        .trigger_poll_timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Clean up stale database from previous runs
    let _ = std::fs::remove_file("event_triggers.db");

    let runner = DefaultRunner::with_config("sqlite://event_triggers.db", config).await?;

    info!("DefaultRunner initialized with trigger scheduling enabled");

    // Workflows + triggers are auto-registered: `#[workflow]` and
    // `#[trigger]` each submit their constructors to the inventory crate
    // at compile time, and `Runtime::new()` walks the inventory during
    // `DefaultRunner::with_config`.

    info!("Workflows + triggers auto-registered from #[workflow]/#[trigger]");

    // Persist trigger → workflow schedule rows so the unified scheduler
    // picks them up on its poll loop. This is the only piece the user
    // still has to wire by hand for an in-process demo; packaged
    // workflows go through the reconciler instead.
    register_trigger_schedules(&runner).await?;

    info!("");
    info!("Event triggers are now active!");
    info!("- File Watcher: polls every 2s for new files");
    info!("- Queue Monitor: polls every 3s, fires when depth > 10");
    info!("- Health Check: polls every 2s, fires after 3 failures");
    info!("");
    info!("Watch the logs to see triggers fire and workflows execute...");
    info!("Press Ctrl+C to shutdown gracefully");
    info!("");

    // Run for a demo period (or until interrupted)
    let runtime_duration = Duration::from_secs(60);

    info!(
        "Running trigger scheduler for {} seconds...",
        runtime_duration.as_secs()
    );
    info!("");

    // Sleep for demo duration or until interrupted
    tokio::select! {
        _ = tokio::time::sleep(runtime_duration) => {
            info!("Demo time completed");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    // Graceful shutdown
    info!("");
    info!("Shutting down gracefully...");
    runner.shutdown().await?;

    info!("Event Triggers Example completed successfully");
    Ok(())
}

/// Persist a `schedules` row for each trigger so the unified scheduler
/// knows which workflow to dispatch when the trigger fires.
///
/// `Runtime::new()` has already seeded the trigger registry from the
/// inventory entries emitted by `#[trigger]`; we look each one up by
/// name and ask the unified scheduler to upsert its schedule row.
async fn register_trigger_schedules(
    runner: &DefaultRunner,
) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = runner
        .unified_scheduler()
        .await
        .ok_or("unified scheduler not enabled — check enable_trigger_scheduling()")?;
    let runtime = runner.runtime();

    // (trigger name, target workflow). The trigger name comes from the
    // `name = "..."` attribute on the #[trigger] macro; the workflow name
    // mirrors the `on = "..."` attribute.
    let bindings = [
        ("file_watcher", "file_processing_workflow"),
        ("queue_monitor", "queue_processing_workflow"),
        ("service_health", "service_recovery_workflow"),
    ];

    for (trigger_name, workflow_name) in bindings {
        let trigger = runtime
            .get_trigger(trigger_name)
            .ok_or_else(|| format!("trigger '{}' not in runtime inventory", trigger_name))?;
        let schedule = scheduler
            .register_trigger(trigger.as_ref(), workflow_name)
            .await?;
        info!(
            "Registered schedule: {} -> {} (ID: {})",
            trigger_name, workflow_name, schedule.id
        );
    }

    Ok(())
}
