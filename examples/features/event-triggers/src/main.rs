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
use cloacina::trigger::register_trigger;
use cloacina::workflow;
use std::time::Duration;
use tracing::info;

mod tasks;
mod triggers;

use tasks::*;
use triggers::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("event_triggers_example=info,cloacina=info")
        .init();

    info!("Starting Event Triggers Example");
    info!("================================");

    // Create DefaultRunner with trigger scheduling enabled
    let mut config = DefaultRunnerConfig::default();
    config.enable_trigger_scheduling = true;
    config.trigger_base_poll_interval = Duration::from_secs(1); // Base poll every 1 second
    config.trigger_poll_timeout = Duration::from_secs(10); // 10 second timeout per trigger

    let runner = DefaultRunner::with_config(
        "sqlite://event_triggers.db?mode=rwc&_journal_mode=WAL",
        config,
    )
    .await?;

    info!("DefaultRunner initialized with trigger scheduling enabled");

    // Register our workflows
    let _file_processing = create_file_processing_workflow()?;
    let _queue_processing = create_queue_processing_workflow()?;
    let _service_recovery = create_service_recovery_workflow()?;

    info!("Workflows registered successfully");

    // Register triggers in the global registry
    register_triggers();

    info!("Triggers registered successfully");

    // Register triggers with the scheduler (persists to database)
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

/// Create the file processing workflow triggered by file watcher.
fn create_file_processing_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "file_processing_workflow",
        description: "Process incoming data files",
        tasks: [
            validate_file,
            process_file,
            archive_file
        ]
    };

    Ok(workflow)
}

/// Create the queue processing workflow triggered by queue depth.
fn create_queue_processing_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "queue_processing_workflow",
        description: "Process messages when queue backs up",
        tasks: [
            drain_queue,
            process_messages,
            ack_messages
        ]
    };

    Ok(workflow)
}

/// Create the service recovery workflow triggered by health check failures.
fn create_service_recovery_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "service_recovery_workflow",
        description: "Recover failed services automatically",
        tasks: [
            diagnose_failure,
            restart_service,
            verify_recovery,
            notify_incident
        ]
    };

    Ok(workflow)
}

/// Register triggers in the global trigger registry.
fn register_triggers() {
    // File watcher trigger
    let file_trigger = create_file_watcher_trigger();
    register_trigger(file_trigger);
    info!("Registered: file_watcher trigger");

    // Queue depth trigger
    let queue_trigger = create_queue_depth_trigger();
    register_trigger(queue_trigger);
    info!("Registered: queue_monitor trigger");

    // Health check trigger
    let health_trigger = create_health_check_trigger();
    register_trigger(health_trigger);
    info!("Registered: service_health trigger");
}

/// Register trigger schedules with the runner (persists configuration to DB).
async fn register_trigger_schedules(
    runner: &DefaultRunner,
) -> Result<(), Box<dyn std::error::Error>> {
    use cloacina::trigger::get_trigger;

    // Get access to the DAL through the runner
    let dal = runner.dal();

    // Register file watcher -> file processing workflow
    if let Some(trigger) = get_trigger("file_watcher") {
        let schedule = dal
            .trigger_schedule()
            .upsert(cloacina::models::trigger_schedule::NewTriggerSchedule::new(
                trigger.name(),
                "file_processing_workflow",
                trigger.poll_interval(),
            ))
            .await?;
        info!(
            "Registered schedule: {} -> {} (ID: {})",
            trigger.name(),
            "file_processing_workflow",
            schedule.id
        );
    }

    // Register queue monitor -> queue processing workflow
    if let Some(trigger) = get_trigger("queue_monitor") {
        let schedule = dal
            .trigger_schedule()
            .upsert(
                cloacina::models::trigger_schedule::NewTriggerSchedule::new(
                    trigger.name(),
                    "queue_processing_workflow",
                    trigger.poll_interval(),
                )
                .with_allow_concurrent(true), // Allow concurrent queue processing
            )
            .await?;
        info!(
            "Registered schedule: {} -> {} (ID: {})",
            trigger.name(),
            "queue_processing_workflow",
            schedule.id
        );
    }

    // Register health check -> service recovery workflow
    if let Some(trigger) = get_trigger("service_health") {
        let schedule = dal
            .trigger_schedule()
            .upsert(cloacina::models::trigger_schedule::NewTriggerSchedule::new(
                trigger.name(),
                "service_recovery_workflow",
                trigger.poll_interval(),
            ))
            .await?;
        info!(
            "Registered schedule: {} -> {} (ID: {})",
            trigger.name(),
            "service_recovery_workflow",
            schedule.id
        );
    }

    Ok(())
}
