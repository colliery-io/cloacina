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

//! # Cron Scheduling Example
//!
//! This example demonstrates how to use Cloacina's cron scheduling feature to
//! automatically execute workflows on a time-based schedule. It showcases:
//!
//! - Creating workflows with the macro system
//! - Setting up cron schedules programmatically
//! - Automatic workflow execution via cron triggers
//! - Monitoring and management of scheduled executions
//! - Recovery from failures and missed executions
//!
//! ## Workflows Demonstrated
//!
//! 1. **data_backup_workflow** - Runs every 30 minutes to backup data
//! 2. **health_check_workflow** - Runs every 5 minutes for system monitoring
//! 3. **daily_report_workflow** - Runs once daily at 6 AM to generate reports
//!
//! ## Cron Features Shown
//!
//! - Multiple concurrent schedules
//! - Different cron expressions and timezones
//! - Execution history and statistics
//! - Graceful shutdown and cleanup
//! - Recovery service for missed executions

use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::workflow;
use std::time::Duration;
use tracing::info;

mod tasks;
use tasks::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cron_scheduling_example=info,cloacina=debug")
        .init();

    info!("Starting Cron Scheduling Example");

    // Create DefaultRunner with cron scheduling enabled
    let mut config = DefaultRunnerConfig::default();
    config.enable_cron_scheduling = true;
    config.cron_enable_recovery = true;
    config.cron_poll_interval = Duration::from_secs(10); // Check every 10 seconds for demo
    config.cron_recovery_interval = Duration::from_secs(30); // Recovery check every 30 seconds

    let runner = DefaultRunner::with_config("sqlite://cronscheduling.db", config).await?;

    info!("DefaultRunner initialized with cron scheduling enabled");

    // Create our workflows
    let _data_backup_workflow = create_data_backup_workflow()?;
    let _health_check_workflow = create_health_check_workflow()?;
    let _daily_report_workflow = create_daily_report_workflow()?;

    info!("Workflows registered successfully");

    // Create cron schedules
    create_cron_schedules(&runner).await?;

    info!("Cron schedules created, workflows will execute automatically");
    info!("You can monitor execution in the logs below...");
    info!("Press Ctrl+C to shutdown gracefully");

    // Run for a demo period (or until interrupted)
    let runtime_duration = Duration::from_secs(300); // 5 minutes demo
    info!(
        "Running cron scheduler for {} seconds...",
        runtime_duration.as_secs()
    );

    // Sleep for demo duration or until interrupted
    tokio::select! {
        _ = tokio::time::sleep(runtime_duration) => {
            info!("Demo time completed");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    // Show execution statistics before shutdown
    show_execution_stats(&runner).await?;

    // Graceful shutdown
    info!("Shutting down gracefully...");
    runner.shutdown().await?;

    info!("Cron Scheduling Example completed successfully");
    Ok(())
}

/// Create the data backup workflow that runs every 30 minutes
fn create_data_backup_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "data_backup_workflow",
        description: "Automated data backup process",
        tasks: [
            check_backup_prerequisites,
            create_backup_snapshot,
            verify_backup_integrity,
            cleanup_old_backups
        ]
    };

    Ok(workflow)
}

/// Create the health check workflow that runs every 5 minutes
fn create_health_check_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "health_check_workflow",
        description: "System health monitoring checks",
        tasks: [
            check_system_resources,
            check_database_connectivity,
            check_external_services,
            update_health_metrics
        ]
    };

    Ok(workflow)
}

/// Create the daily report workflow that runs once per day
fn create_daily_report_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "daily_report_workflow",
        description: "Daily summary report generation",
        tasks: [
            collect_daily_metrics,
            generate_usage_report,
            send_report_notification
        ]
    };

    Ok(workflow)
}

/// Create cron schedules for our workflows
async fn create_cron_schedules(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>> {
    // Schedule 1: Data backup every 30 minutes
    let backup_schedule_id = runner
        .register_cron_workflow(
            "data_backup_workflow",
            "* * * * *", // Every minute
            "UTC",
        )
        .await?;
    info!(
        "Created backup schedule (ID: {}) - runs every 30 minutes",
        backup_schedule_id
    );

    // Schedule 2: Health check every 5 minutes
    let health_schedule_id = runner
        .register_cron_workflow(
            "health_check_workflow",
            "*/2 * * * *", // Every 2 minutes
            "UTC",
        )
        .await?;
    info!(
        "Created health check schedule (ID: {}) - runs every 5 minutes",
        health_schedule_id
    );

    // Schedule 3: Daily report at 6 AM UTC
    let report_schedule_id = runner
        .register_cron_workflow(
            "daily_report_workflow",
            "*/3 * * * *", // Every 3 minutes
            "UTC",
        )
        .await?;
    info!(
        "Created daily report schedule (ID: {}) - runs daily at 6 AM UTC",
        report_schedule_id
    );

    Ok(())
}

/// Display execution statistics
async fn show_execution_stats(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>> {
    let stats = runner
        .get_cron_execution_stats(chrono::Utc::now() - chrono::Duration::try_hours(1).unwrap())
        .await?;

    info!("Execution Statistics (last hour):");
    info!("   Total executions: {}", stats.total_executions);
    info!("   Successful executions: {}", stats.successful_executions);
    info!("   Lost executions: {}", stats.lost_executions);
    info!("   Success rate: {:.1}%", stats.success_rate);

    Ok(())
}
