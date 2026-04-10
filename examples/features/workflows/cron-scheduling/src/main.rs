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
use cloacina::{task, workflow, Context, TaskError};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};

// ============================================================================
// Data Backup Workflow
// ============================================================================

#[workflow(
    name = "data_backup_workflow",
    description = "Automated data backup process"
)]
pub mod data_backup_workflow {
    use super::*;

    /// Check if backup prerequisites are met
    #[task(
        id = "check_backup_prerequisites",
        dependencies = [],
        retry_attempts = 2,
        retry_backoff = "fixed",
        retry_delay_ms = 5000
    )]
    pub async fn check_backup_prerequisites(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Checking backup prerequisites...");

        // Simulate checking disk space, permissions, etc.
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Add backup metadata to context
        context.insert("backup_timestamp", json!(chrono::Utc::now()))?;
        context.insert("backup_type", json!("incremental"))?;
        context.insert("disk_space_available", json!(true))?;

        info!("Backup prerequisites check completed");
        Ok(())
    }

    /// Create a backup snapshot
    #[task(
        id = "create_backup_snapshot",
        dependencies = ["check_backup_prerequisites"],
        retry_attempts = 3,
        retry_backoff = "exponential",
        retry_delay_ms = 2000
    )]
    pub async fn create_backup_snapshot(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Creating backup snapshot...");

        // Simulate backup creation process
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let backup_id = format!("backup_{}", chrono::Utc::now().timestamp());
        let backup_size = 1024 * 1024 * 50; // 50 MB simulated

        context.insert("backup_id", json!(backup_id))?;
        context.insert("backup_size_bytes", json!(backup_size))?;
        context.insert("backup_location", json!("/backups/incremental/"))?;

        info!("Backup snapshot created successfully: {}", backup_id);
        Ok(())
    }

    /// Verify backup integrity
    #[task(
        id = "verify_backup_integrity",
        dependencies = ["create_backup_snapshot"],
        retry_attempts = 2,
        retry_backoff = "fixed",
        retry_delay_ms = 3000
    )]
    pub async fn verify_backup_integrity(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Verifying backup integrity...");

        let backup_id = context
            .get("backup_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Backup ID not found in context".to_string(),
            })?
            .to_string();

        // Simulate integrity check
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        // Simulate occasional integrity check failure (5% chance)
        if rand::random::<f32>() < 0.05 {
            return Err(TaskError::ExecutionFailed {
                message: "Backup integrity check failed - checksum mismatch".to_string(),
                task_id: "verify_backup_integrity".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        context.insert("integrity_verified", json!(true))?;
        context.insert("checksum", json!("sha256:abc123def456"))?;

        info!("Backup integrity verified for: {}", backup_id);
        Ok(())
    }

    /// Clean up old backups
    #[task(
        id = "cleanup_old_backups",
        dependencies = ["verify_backup_integrity"],
        retry_attempts = 1,
        retry_backoff = "fixed",
        retry_delay_ms = 1000
    )]
    pub async fn cleanup_old_backups(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Cleaning up old backups...");

        // Simulate cleanup process
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        let cleaned_count = 3; // Simulate removing 3 old backups
        context.insert("backups_cleaned", json!(cleaned_count))?;

        info!("Cleaned up {} old backup files", cleaned_count);
        Ok(())
    }
}

// ============================================================================
// Health Check Workflow
// ============================================================================

#[workflow(
    name = "health_check_workflow",
    description = "System health monitoring checks"
)]
pub mod health_check_workflow {
    use super::*;

    /// Check system resources (CPU, memory, disk)
    #[task(
        id = "check_system_resources",
        dependencies = [],
        retry_attempts = 1,
        retry_backoff = "fixed",
        retry_delay_ms = 1000
    )]
    pub async fn check_system_resources(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Checking system resources...");

        // Simulate resource checking
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Simulate realistic resource usage
        let cpu_usage = rand::random::<f32>() * 50.0 + 10.0; // 10-60%
        let memory_usage = rand::random::<f32>() * 40.0 + 20.0; // 20-60%
        let disk_usage = rand::random::<f32>() * 30.0 + 30.0; // 30-60%

        context.insert("cpu_usage_percent", json!(cpu_usage))?;
        context.insert("memory_usage_percent", json!(memory_usage))?;
        context.insert("disk_usage_percent", json!(disk_usage))?;
        context.insert("resource_check_timestamp", json!(chrono::Utc::now()))?;

        // Warn if resources are high
        if cpu_usage > 80.0 || memory_usage > 80.0 || disk_usage > 80.0 {
            warn!(
                "High resource usage detected: CPU={:.1}%, Memory={:.1}%, Disk={:.1}%",
                cpu_usage, memory_usage, disk_usage
            );
            context.insert("resource_warning", json!(true))?;
        }

        info!("System resource check completed");
        Ok(())
    }

    /// Check database connectivity
    #[task(
        id = "check_database_connectivity",
        dependencies = [],
        retry_attempts = 3,
        retry_backoff = "exponential",
        retry_delay_ms = 1000
    )]
    pub async fn check_database_connectivity(
        context: &mut Context<Value>,
    ) -> Result<(), TaskError> {
        info!("Checking database connectivity...");

        // Simulate database connection check
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Simulate occasional connection issues (2% chance)
        if rand::random::<f32>() < 0.02 {
            return Err(TaskError::ExecutionFailed {
                message: "Database connection timeout".to_string(),
                task_id: "check_database_connectivity".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        let response_time_ms = rand::random::<u32>() % 50 + 5; // 5-55ms
        context.insert("db_response_time_ms", json!(response_time_ms))?;
        context.insert("db_connection_healthy", json!(true))?;

        info!(
            "Database connectivity check passed ({}ms)",
            response_time_ms
        );
        Ok(())
    }

    /// Check external service availability
    #[task(
        id = "check_external_services",
        dependencies = [],
        retry_attempts = 2,
        retry_backoff = "linear",
        retry_delay_ms = 2000
    )]
    pub async fn check_external_services(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Checking external services...");

        // Simulate checking multiple external services
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

        let mut services_status = json!({});
        let services = ["auth_service", "payment_gateway", "notification_service"];

        for service in &services {
            // Simulate service availability (95% uptime)
            let is_healthy = rand::random::<f32>() > 0.05;
            let response_time = if is_healthy {
                rand::random::<u32>() % 200 + 50 // 50-250ms
            } else {
                5000 // Timeout
            };

            services_status[service] = json!({
                "healthy": is_healthy,
                "response_time_ms": response_time
            });

            if !is_healthy {
                warn!("Service {} is unhealthy", service);
            }
        }

        context.insert("external_services_status", services_status)?;
        info!("External services check completed");
        Ok(())
    }

    /// Update health metrics aggregation
    #[task(
        id = "update_health_metrics",
        dependencies = ["check_system_resources", "check_database_connectivity", "check_external_services"],
        retry_attempts = 2,
        retry_backoff = "fixed",
        retry_delay_ms = 1000
    )]
    pub async fn update_health_metrics(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Updating health metrics...");

        // Simulate metrics aggregation
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Calculate overall health score
        let mut health_score = 100.0;

        // Reduce score based on resource usage
        if let Some(cpu) = context.get("cpu_usage_percent").and_then(|v| v.as_f64()) {
            if cpu > 80.0 {
                health_score -= 20.0;
            } else if cpu > 60.0 {
                health_score -= 10.0;
            }
        }

        // Check database health
        if context
            .get("db_connection_healthy")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            // DB is healthy, no reduction
        } else {
            health_score -= 30.0;
        }

        // Check external services
        if let Some(services) = context.get("external_services_status") {
            let services_obj = services.as_object().unwrap();
            let unhealthy_count = services_obj
                .values()
                .filter(|status| !status["healthy"].as_bool().unwrap_or(false))
                .count();
            health_score -= (unhealthy_count as f64) * 15.0;
        }

        context.insert("overall_health_score", json!(health_score.max(0.0)))?;
        context.insert("health_check_timestamp", json!(chrono::Utc::now()))?;

        info!("Health metrics updated (score: {:.1})", health_score);
        Ok(())
    }
}

// ============================================================================
// Daily Report Workflow
// ============================================================================

#[workflow(
    name = "daily_report_workflow",
    description = "Daily summary report generation"
)]
pub mod daily_report_workflow {
    use super::*;

    /// Collect daily metrics from various sources
    #[task(
        id = "collect_daily_metrics",
        dependencies = [],
        retry_attempts = 2,
        retry_backoff = "exponential",
        retry_delay_ms = 3000
    )]
    pub async fn collect_daily_metrics(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Collecting daily metrics...");

        // Simulate collecting metrics from various sources
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let metrics = json!({
            "total_users": rand::random::<u32>() % 1000 + 5000,
            "total_transactions": rand::random::<u32>() % 10000 + 50000,
            "revenue_usd": (rand::random::<f32>() * 50000.0 + 100000.0).round() / 100.0,
            "uptime_percentage": 99.5 + rand::random::<f32>() * 0.5,
            "api_requests": rand::random::<u32>() % 1000000 + 5000000,
            "error_rate_percentage": rand::random::<f32>() * 0.5,
            "collection_date": chrono::Utc::now().date_naive()
        });

        context.insert("daily_metrics", metrics)?;
        info!("Daily metrics collection completed");
        Ok(())
    }

    /// Generate usage report from collected metrics
    #[task(
        id = "generate_usage_report",
        dependencies = ["collect_daily_metrics"],
        retry_attempts = 2,
        retry_backoff = "fixed",
        retry_delay_ms = 2000
    )]
    pub async fn generate_usage_report(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Generating usage report...");

        let metrics = context
            .get("daily_metrics")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Daily metrics not found in context".to_string(),
            })?;

        // Simulate report generation
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        let report_id = format!("daily_report_{}", chrono::Utc::now().format("%Y%m%d"));
        let report_content = json!({
            "report_id": report_id,
            "generated_at": chrono::Utc::now(),
            "metrics": metrics,
            "summary": {
                "status": "healthy",
                "key_insights": [
                    "User growth steady",
                    "Transaction volume normal",
                    "System performance optimal"
                ]
            }
        });

        context.insert("report_content", report_content)?;
        context.insert("report_id", json!(report_id))?;

        info!("Usage report generated: {}", report_id);
        Ok(())
    }

    /// Send report notification to stakeholders
    #[task(
        id = "send_report_notification",
        dependencies = ["generate_usage_report"],
        retry_attempts = 3,
        retry_backoff = "exponential",
        retry_delay_ms = 1000
    )]
    pub async fn send_report_notification(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Sending report notification...");

        let report_id = context
            .get("report_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Report ID not found in context".to_string(),
            })?
            .to_string();

        // Simulate sending notification
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Simulate occasional notification failure (3% chance)
        if rand::random::<f32>() < 0.03 {
            return Err(TaskError::ExecutionFailed {
                message: "Failed to send notification - mail server unavailable".to_string(),
                task_id: "send_report_notification".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        let recipients = vec!["team@company.com", "manager@company.com"];
        context.insert("notification_sent", json!(true))?;
        context.insert("notification_recipients", json!(recipients))?;
        context.insert("notification_timestamp", json!(chrono::Utc::now()))?;

        info!("Report notification sent for: {}", report_id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cron_scheduling_example=info,cloacina=info")
        .init();

    info!("Starting Cron Scheduling Example");

    // Create DefaultRunner with cron scheduling enabled
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .cron_enable_recovery(true)
        .cron_poll_interval(Duration::from_secs(10)) // Check every 10 seconds for demo
        .cron_recovery_interval(Duration::from_secs(30)) // Recovery check every 30 seconds
        .build();

    // Clean up stale database from previous runs
    let _ = std::fs::remove_file("cronscheduling.db");

    let runner = DefaultRunner::with_config("sqlite://cronscheduling.db", config).await?;

    info!("DefaultRunner initialized with cron scheduling enabled");

    // Workflows are auto-registered by #[workflow] attribute macro

    info!("Workflows registered successfully");

    // Create cron schedules
    create_cron_schedules(&runner).await?;

    info!("Cron schedules created, workflows will execute automatically");
    info!("You can monitor execution in the logs below...");
    info!("Press Ctrl+C to shutdown gracefully");

    // Run for a demo period (or until interrupted)
    let runtime_duration = Duration::from_secs(60); // 5 minutes demo

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

/// Create cron schedules for our workflows
async fn create_cron_schedules(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>> {
    // Schedule 1: Data backup every 30 minutes
    let backup_schedule_id = runner
        .register_cron_workflow(
            "data_backup_workflow",
            "*/30 * * * * *", // Every 30 seconds
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
            "*/15 * * * * *", // Every 2 minutes
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
            "*/10 * * * * *", // Every 3 minutes
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
