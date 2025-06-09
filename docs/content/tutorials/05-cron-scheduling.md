---
title: "05 - Cron Scheduling"
description: "Creating complex, fault-tolerant workflows with Cloacina"
weight: 15
reviewer: "dstorey"
review_date: "2024-04-2"
---

## Overview

This tutorial will guide you through setting up automated workflow execution using Cloacina's cron scheduling feature. You'll learn how to create time-based triggers for your workflows, manage schedules, and monitor execution.

## Prerequisites

Before starting this tutorial, you should:

- Completion of [Tutorial 4]({{< ref "/tutorials/04-error-handling/" >}})
- Be familiar with creating workflows and tasks using Cloacina's macro system
- Understand basic cron expression syntax
- Have Rust toolchain installed (rustc, cargo)

## Time Estimate
20-30 minutes

## What You'll Learn

- How to enable cron scheduling in the DefaultRunner
- Creating and registering cron schedules for workflows
- Understanding cron expressions and timezones
- Monitoring scheduled executions
- Configuring recovery and catchup policies
- Best practices for scheduled workflows

## Setting Up Your Project

Create a new Rust project for this tutorial:

```bash
# From your project directory
cargo new cron-tutorial
cd cron-tutorial
```

Add the required dependencies to your `Cargo.toml`:

```toml
[package]
name = "cron-tutorial"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { path = "../cloacina", default-features = false, features = ["macros", "sqlite"] }
# cloacina = { path = "../cloacina", default-features = false, features = ["macros", "postgres"] }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
ctor = "0.2"
rand = "0.8"
```

## Understanding Cron Scheduling

Cloacina's cron scheduling system provides:

- **Time-based triggers**: Execute workflows at specific times using cron expressions
- **Timezone support**: Schedule workflows in any timezone
- **Recovery mechanisms**: Handle missed executions due to downtime
- **Execution tracking**: Monitor and audit all scheduled executions
- **Guaranteed execution**: Prevent duplicate executions and ensure reliability

The system uses a dedicated `CronScheduler` that:
1. Polls for due schedules at configurable intervals
2. Creates pipeline executions for scheduled workflows
3. Tracks execution attempts to prevent duplicates
4. Provides recovery for lost or missed executions

## Step 1: Create Your Scheduled Tasks

First, let's create some tasks that will run on a schedule. Create `src/tasks.rs`:

```rust
//! Task definitions for cron scheduling tutorial

use cloacina::{task, Context, TaskError};
use serde_json::{json, Value};
use tracing::info;

/// A data backup task that runs periodically
#[task(
    id = "backup_data",
    dependencies = [],
    retry_attempts = 2,
    retry_backoff = "exponential",
    retry_delay_ms = 2000
)]
pub async fn backup_data(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Starting data backup...");

    // Simulate backup process
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let backup_id = format!("backup_{}", chrono::Utc::now().timestamp());
    let backup_size = 1024 * 1024 * 25; // 25 MB simulated

    context.insert("backup_id", json!(backup_id))?;
    context.insert("backup_size_bytes", json!(backup_size))?;
    context.insert("backup_timestamp", json!(chrono::Utc::now()))?;

    info!("Data backup completed: {}", backup_id);
    Ok(())
}

/// A health monitoring task
#[task(
    id = "health_check",
    dependencies = [],
    retry_attempts = 1,
    retry_backoff = "fixed",
    retry_delay_ms = 1000
)]
pub async fn health_check(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Performing health check...");

    // Simulate health checks
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let cpu_usage = rand::random::<f32>() * 50.0 + 10.0; // 10-60%
    let memory_usage = rand::random::<f32>() * 40.0 + 20.0; // 20-60%

    context.insert("cpu_usage_percent", json!(cpu_usage))?;
    context.insert("memory_usage_percent", json!(memory_usage))?;
    context.insert("health_check_timestamp", json!(chrono::Utc::now()))?;
    context.insert("system_healthy", json!(cpu_usage < 80.0 && memory_usage < 80.0))?;

    info!("Health check completed - CPU: {:.1}%, Memory: {:.1}%", cpu_usage, memory_usage);
    Ok(())
}

/// A report generation task
#[task(
    id = "generate_report",
    dependencies = [],
    retry_attempts = 2,
    retry_backoff = "linear",
    retry_delay_ms = 1500
)]
pub async fn generate_report(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Generating daily report...");

    // Simulate report generation
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    let report_id = format!("report_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    let metrics = json!({
        "total_users": rand::random::<u32>() % 1000 + 5000,
        "total_requests": rand::random::<u32>() % 10000 + 50000,
        "uptime_percentage": 99.0 + rand::random::<f32>(),
        "generation_time": chrono::Utc::now()
    });

    context.insert("report_id", json!(report_id))?;
    context.insert("report_data", metrics)?;

    info!("Report generated: {}", report_id);
    Ok(())
}
```

## Step 2: Define Your Workflows

Now create the workflows that will be scheduled. Update your `src/main.rs`:

```rust
//! Cron Scheduling Tutorial
//!
//! This tutorial demonstrates how to set up automated workflow execution
//! using Cloacina's cron scheduling feature.

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
        .with_env_filter("cron_tutorial=info,cloacina=debug")
        .init();

    info!("Starting Cron Scheduling Tutorial");

    // Create workflows
    let _backup_workflow = create_backup_workflow()?;
    let _health_workflow = create_health_workflow()?;
    let _report_workflow = create_report_workflow()?;

    // Configure DefaultRunner with cron scheduling
    let mut config = DefaultRunnerConfig::default();
    config.enable_cron_scheduling = true;
    config.cron_enable_recovery = true;
    config.cron_poll_interval = Duration::from_secs(5); // Check every 5 seconds
    config.cron_recovery_interval = Duration::from_secs(30); // Recovery check every 30 seconds

    let runner = DefaultRunner::with_config(
        "sqlite://cron-tutorial.db",
        config,
    )
    .await?;

    info!("DefaultRunner initialized with cron scheduling enabled");

    // Register cron schedules
    register_schedules(&runner).await?;

    info!("Cron schedules registered, workflows will execute automatically");
    info!("Monitor execution in the logs below...");
    info!("Press Ctrl+C to shutdown gracefully");

    // Run until interrupted
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(120)) => {
            info!("Tutorial time completed");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    // Show execution statistics
    show_execution_stats(&runner).await?;

    // Graceful shutdown
    info!("Shutting down...");
    runner.shutdown().await?;

    info!("Cron Scheduling Tutorial completed");
    Ok(())
}

/// Create the backup workflow
fn create_backup_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "backup_workflow",
        description: "Automated data backup",
        tasks: [
            backup_data
        ]
    };
    Ok(workflow)
}

/// Create the health check workflow
fn create_health_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "health_workflow",
        description: "System health monitoring",
        tasks: [
            health_check
        ]
    };
    Ok(workflow)
}

/// Create the report generation workflow
fn create_report_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "report_workflow",
        description: "Daily report generation",
        tasks: [
            generate_report
        ]
    };
    Ok(workflow)
}

/// Register cron schedules for the workflows
async fn register_schedules(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>> {
    // Schedule 1: Backup every 2 minutes (for demo purposes)
    let backup_schedule_id = runner.register_cron_workflow(
        "backup_workflow",
        "*/2 * * * *",  // Every 2 minutes
        "UTC"
    ).await?;
    info!("Backup schedule created (ID: {}) - runs every 2 minutes", backup_schedule_id);

    // Schedule 2: Health check every minute
    let health_schedule_id = runner.register_cron_workflow(
        "health_workflow",
        "* * * * *",   // Every minute
        "UTC"
    ).await?;
    info!("Health check schedule created (ID: {}) - runs every minute", health_schedule_id);

    // Schedule 3: Report every 5 minutes
    let report_schedule_id = runner.register_cron_workflow(
        "report_workflow",
        "*/5 * * * *",     // Every 5 minutes
        "UTC"
    ).await?;
    info!("Report schedule created (ID: {}) - runs every 5 minutes", report_schedule_id);

    Ok(())
}

/// Display execution statistics
async fn show_execution_stats(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>> {
    let stats = runner
        .get_cron_execution_stats(chrono::Utc::now() - chrono::Duration::try_minutes(10).unwrap())
        .await?;

    info!("Execution Statistics (last 10 minutes):");
    info!("   Total executions: {}", stats.total_executions);
    info!("   Successful executions: {}", stats.successful_executions);
    info!("   Lost executions: {}", stats.lost_executions);
    info!("   Success rate: {:.1}%", stats.success_rate);

    Ok(())
}
```

## Step 3: Run Your Scheduled Workflows

Build and run your project:

```bash
cargo run
```

You should see output like:

Starting Cron Scheduling Tutorial
DefaultRunner initialized with cron scheduling enabled
Backup schedule created (ID: 01942234-...) - runs every 2 minutes
Health check schedule created (ID: 01942234-...) - runs every minute
Report schedule created (ID: 01942234-...) - runs every 5 minutes
Cron schedules registered, workflows will execute automatically
Monitor execution in the logs below...

Performing health check...
Health check completed - CPU: 45.2%, Memory: 38.7%
Starting data backup...
Data backup completed: backup_1736283642
```

## Step 4: Advanced Configuration

Now that you have the basic tutorial working, let's explore the configuration options available.

### Cron Scheduler Settings

The `DefaultRunnerConfig` provides several important cron-related settings:

```rust
let mut config = DefaultRunnerConfig::default();

// Enable cron scheduling functionality
config.enable_cron_scheduling = true;

// Enable automatic recovery of lost executions
config.cron_enable_recovery = true;

// How often to check for due schedules (responsiveness vs load)
config.cron_poll_interval = Duration::from_secs(5);

// How often to check for lost executions
config.cron_recovery_interval = Duration::from_secs(30);

// Consider executions lost after this many minutes
config.cron_lost_threshold_minutes = 5;

// Maximum number of missed executions to catch up (usize::MAX = unlimited by default)
config.cron_max_catchup_executions = 50;
```

### Timezone Support

Cloacina supports timezone-aware scheduling:

```rust
// Schedule in Eastern Time
runner.register_cron_workflow(
    "business_report",
    "0 9 * * 1-5",      // 9 AM weekdays
    "America/New_York"   // EST/EDT timezone
).await?;

// Schedule in UTC (recommended for most cases)
runner.register_cron_workflow(
    "global_sync",
    "0 */6 * * *",       // Every 6 hours
    "UTC"                // Coordinated Universal Time
).await?;
```

## Step 5: Recovery vs Catchup Policies

Cloacina has two distinct mechanisms for handling missed executions:

**1. Recovery System (Automatic)**
The recovery system automatically detects and recovers executions that were claimed but failed to complete due to system crashes or process failures. This is handled transparently by the recovery service.

**2. Catchup Policies (Manual/Planned)**
Catchup policies determine how to handle missed executions when the system is intentionally down for maintenance or when starting up after planned downtime:

```rust
use cloacina::models::cron_schedule::{ScheduleConfig, CatchupPolicy};

// Skip missed executions (default) - for health checks or monitoring
let config = ScheduleConfig {
    name: "health_check".to_string(),
    cron: "*/5 * * * *".to_string(),
    workflow: "health_workflow".to_string(),
    timezone: "UTC".to_string(),
    catchup_policy: CatchupPolicy::Skip, // Skip missed checks during downtime
    start_date: None,
    end_date: None,
};

// Run all missed executions - for data processing that needs backfill
let config = ScheduleConfig {
    name: "data_sync".to_string(),
    cron: "0 * * * *".to_string(),
    workflow: "sync_workflow".to_string(),
    timezone: "UTC".to_string(),
    catchup_policy: CatchupPolicy::RunAll, // Process all missed hourly syncs
    start_date: None,
    end_date: None,
};
```

Use `RunAll` catchup policy for workflows that must process every scheduled interval (like data ETL), and `Skip` for monitoring or health checks where only the current state matters.

**Note**: The `cron_max_catchup_executions` setting in DefaultRunnerConfig controls how many missed executions the scheduler will attempt to catch up when it starts up. By default this is `usize::MAX` (unlimited), but you can set a lower value to prevent catchup storms. This works with catchup policies - if a schedule has `CatchupPolicy::RunAll` and the system was down for 6 hours with hourly executions, it will run up to `cron_max_catchup_executions` missed executions to bring the schedule current.

### Time Windows

You can limit schedules to specific time windows:

```rust
use chrono::{DateTime, Utc};

let start = DateTime::parse_from_rfc3339("2025-02-01T00:00:00Z")?.with_timezone(&Utc);
let end = DateTime::parse_from_rfc3339("2025-02-28T23:59:59Z")?.with_timezone(&Utc);

let config = ScheduleConfig {
    name: "february_promotion".to_string(),
    cron: "0 12 * * *".to_string(),  // Daily at noon
    workflow: "promotion_workflow".to_string(),
    timezone: "UTC".to_string(),
    catchup_policy: CatchupPolicy::Skip,
    start_date: Some(start),
    end_date: Some(end),
};
```


## Best Practices

### 1. Choose Appropriate Poll Intervals

```rust
// For responsive systems (higher DB load)
config.cron_poll_interval = Duration::from_secs(5);

// For efficient systems (lower DB load)
config.cron_poll_interval = Duration::from_secs(30);
```

### 2. Use UTC for Most Schedules

```rust
// Recommended: Use UTC to avoid timezone complications
runner.register_cron_workflow("backup", "0 2 * * *", "UTC").await?;

// Only use local timezones when business logic requires it
runner.register_cron_workflow("business_report", "0 9 * * 1-5", "America/New_York").await?;
```


## Summary

In this tutorial, you learned how to:

- Enable cron scheduling in DefaultRunner
- Create scheduled workflows with time-based triggers
- Use cron expressions and timezone support
- Configure recovery and catchup policies
- Monitor execution statistics and performance
- Apply best practices for production systems

Cron scheduling in Cloacina provides a robust foundation for automating your workflows with precise timing control, comprehensive recovery mechanisms, and detailed execution tracking.

## Next Steps

- Explore advanced cron expressions for complex scheduling needs
- Integrate with monitoring systems for production alerting
- Learn about multi-tenant cron scheduling in the [Multi-Tenant Setup Guide]({{< ref "/how-to-guides/multi-tenant-setup/" >}})
- Study the [Error Handling Tutorial]({{< ref "/tutorials/04-error-handling/" >}}) for robust scheduled workflows
