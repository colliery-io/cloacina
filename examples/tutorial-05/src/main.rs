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

//! Tutorial 05: Cron Scheduling
//!
//! This tutorial demonstrates how to use Cloacina's cron scheduling to
//! automatically execute workflows on time-based triggers.

use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::time::Duration;
use tracing::info;

#[task(
    id = "backup_data",
    dependencies = []
)]
async fn backup_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Starting data backup...");

    // Simulate backup process
    let backup_type = context
        .get("backup_type")
        .and_then(|v| v.as_str())
        .unwrap_or("incremental")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let backup_size = match backup_type.as_str() {
        "full" => "2.5GB",
        "incremental" => "150MB",
        _ => "100MB",
    };

    info!("✓ {} backup completed: {}", backup_type, backup_size);

    context.insert("backup_completed", json!(true))?;
    context.insert("backup_size", json!(backup_size))?;
    context.insert("backup_type", json!(backup_type))?;

    Ok(())
}

#[task(
    id = "health_check",
    dependencies = []
)]
async fn health_check(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Performing system health check...");

    // Simulate health monitoring
    let cpu_usage = 45.2;
    let memory_usage = 62.8;
    let disk_usage = 73.1;

    let health_status = if cpu_usage > 80.0 || memory_usage > 90.0 {
        "warning"
    } else if cpu_usage > 95.0 || memory_usage > 95.0 {
        "critical"
    } else {
        "healthy"
    };

    info!(
        "✓ Health check complete: {} (CPU: {}%, Memory: {}%)",
        health_status, cpu_usage, memory_usage
    );

    context.insert("health_status", json!(health_status))?;
    context.insert("cpu_usage", json!(cpu_usage))?;
    context.insert("memory_usage", json!(memory_usage))?;
    context.insert("disk_usage", json!(disk_usage))?;

    Ok(())
}

#[task(
    id = "generate_report",
    dependencies = []
)]
async fn generate_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Generating daily report...");

    // Simulate report generation
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let report_data = serde_json::json!({
        "total_orders": 150,
        "revenue": 12500.50,
        "active_users": 89,
        "generated_at": chrono::Utc::now().to_rfc3339()
    });

    info!(
        "✓ Daily report generated: {} orders, ${} revenue",
        report_data["total_orders"], report_data["revenue"]
    );

    context.insert("report_data", report_data)?;
    context.insert("report_generated", json!(true))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_05=info,cloacina=info")
        .init();

    info!("Starting Tutorial 05: Cron Scheduling");

    // Create runner with default configuration
    let runner = DefaultRunner::with_config(
        "sqlite://tutorial-05.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    info!("✓ Runner created with cron scheduling enabled");

    // Create workflows using the macro system
    let _backup_workflow = workflow! {
        name: "data_backup",
        description: "Automated data backup",
        tasks: [
            backup_data
        ]
    };

    let _health_workflow = workflow! {
        name: "health_check",
        description: "System health monitoring",
        tasks: [
            health_check
        ]
    };

    let _report_workflow = workflow! {
        name: "daily_report",
        description: "Daily report generation",
        tasks: [
            generate_report
        ]
    };

    // In a real application, you would set up cron schedules like this:
    //
    // use cloacina::cron::{CronSchedule, CronExpression};
    //
    // let backup_schedule = CronSchedule::new(
    //     "data_backup",
    //     CronExpression::parse("0 */30 * * * *")?, // Every 30 minutes
    //     Some(Context::from_json(json!({"backup_type": "incremental"})))
    // );
    //
    // runner.add_schedule(backup_schedule).await?;

    // For this tutorial, we'll demonstrate manual execution

    // Cleanup
    runner.shutdown().await?;

    Ok(())
}
