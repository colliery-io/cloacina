---
title: "09 - Event Triggers"
description: "Event-driven workflow execution with custom trigger conditions"
weight: 19
reviewer: "dstorey"
review_date: "2025-01-10"
---

## Overview

This tutorial guides you through implementing event-driven workflow execution using Cloacina's trigger system. Unlike cron scheduling (time-based), event triggers poll user-defined conditions and fire workflows when those conditions are met.

## Prerequisites

Before starting this tutorial, you should:

- Completion of [Tutorial 5 - Cron Scheduling]({{< ref "/tutorials/05-cron-scheduling/" >}})
- Be familiar with creating workflows and tasks using Cloacina's macro system
- Understand async/await patterns in Rust
- Have Rust toolchain installed (rustc, cargo)

## Time Estimate
25-35 minutes

## What You'll Learn

- How to implement the `Trigger` trait for custom conditions
- Registering triggers with workflows
- Context passing from triggers to workflows
- Deduplication strategies for concurrent executions
- Combining triggers with cron scheduling
- Best practices for trigger implementations

## Understanding Event Triggers

Event triggers complement cron scheduling by providing condition-based workflow execution:

| Feature | Event Triggers | Cron Scheduling |
|---------|---------------|-----------------|
| Activation | Condition-based | Time-based |
| Poll Logic | User-defined | Cron expression |
| Context | Dynamic from trigger | Static |
| Deduplication | Context hash | Time window |
| Use Case | Reactive workflows | Scheduled jobs |

## Key Concepts

### The Trigger Trait

Triggers implement the `Trigger` trait:

```rust
#[async_trait]
pub trait Trigger: Send + Sync {
    /// Unique identifier for this trigger
    fn name(&self) -> &str;

    /// How often to poll this trigger
    fn poll_interval(&self) -> Duration;

    /// Whether to allow concurrent executions
    fn allow_concurrent(&self) -> bool;

    /// Check condition and return result
    async fn poll(&self) -> Result<TriggerResult, TriggerError>;
}
```

### TriggerResult

The `poll()` function returns one of:

```rust
pub enum TriggerResult {
    /// Don't fire, continue polling
    Skip,
    /// Fire the workflow with optional context
    Fire(Option<Context<Value>>),
}
```

### Deduplication

When `allow_concurrent = false`, the trigger scheduler prevents duplicate executions:

1. Context is hashed when `TriggerResult::Fire` is returned
2. Active executions are tracked by (trigger_name, context_hash)
3. If an execution with the same hash is running, the trigger skips

## Setting Up Your Project

Create a new Rust project for this tutorial:

```bash
cargo new event-triggers-tutorial
cd event-triggers-tutorial
```

Add the required dependencies to your `Cargo.toml`:

```toml
[package]
name = "event-triggers-tutorial"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { path = "../cloacina", features = ["sqlite", "macros"] }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde_json = "1.0"
chrono = "0.4"
async-trait = "0.1"
ctor = "0.2"
```

## Step 1: Create Your Trigger

Let's create a file watcher trigger that monitors for new files:

```rust
// src/triggers.rs

use async_trait::async_trait;
use cloacina::trigger::{Trigger, TriggerError, TriggerResult};
use cloacina::Context;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tracing::info;

/// Counter for simulating file arrivals
static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A trigger that polls for new files in a directory
#[derive(Debug, Clone)]
pub struct FileWatcherTrigger {
    name: String,
    poll_interval: Duration,
    watch_path: String,
}

impl FileWatcherTrigger {
    pub fn new(name: &str, watch_path: &str, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            watch_path: watch_path.to_string(),
        }
    }

    /// Check for new files (simulated for demo)
    async fn check_for_new_files(&self) -> Option<String> {
        let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        if count % 5 == 4 {
            // "Find" a file every 5th poll
            let filename = format!("data_{}.csv", chrono::Utc::now().timestamp());
            info!("Found new file: {}", filename);
            Some(filename)
        } else {
            None
        }
    }
}

#[async_trait]
impl Trigger for FileWatcherTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        false // Don't process same file twice
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        if let Some(filename) = self.check_for_new_files().await {
            let mut ctx = Context::new();
            ctx.insert("filename", serde_json::json!(filename))
                .map_err(|e| TriggerError::PollError {
                    message: e.to_string(),
                })?;
            ctx.insert("watch_path", serde_json::json!(self.watch_path.clone()))
                .map_err(|e| TriggerError::PollError {
                    message: e.to_string(),
                })?;
            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}
```

## Step 2: Create the Workflow Tasks

Create tasks that will be triggered:

```rust
// src/tasks.rs

use cloacina::prelude::*;
use cloacina::task;
use tracing::info;

#[task(id = "validate_file", dependencies = [])]
pub async fn validate_file(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    let filename = context
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Validating file: {}", filename);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    context.insert("validated", serde_json::json!(true))?;
    info!("File '{}' validated", filename);
    Ok(())
}

#[task(id = "process_file", dependencies = ["validate_file"])]
pub async fn process_file(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    let filename = context
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Processing file: {}", filename);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    context.insert("processed", serde_json::json!(true))?;
    info!("File '{}' processed", filename);
    Ok(())
}
```

## Step 3: Register and Run

Set up the runner with trigger scheduling enabled:

```rust
// src/main.rs

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
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Configure runner with trigger scheduling
    let mut config = DefaultRunnerConfig::default();
    config.enable_trigger_scheduling = true;
    config.trigger_base_poll_interval = Duration::from_secs(1);

    let runner = DefaultRunner::with_config(
        "sqlite://triggers.db?mode=rwc",
        config,
    ).await?;

    // Create workflow
    let _workflow = workflow! {
        name: "file_processing",
        tasks: [validate_file, process_file]
    };

    // Create and register trigger
    let trigger = FileWatcherTrigger::new(
        "file_watcher",
        "/data/inbox",
        Duration::from_secs(2),
    );
    register_trigger(trigger.clone());

    // Register trigger schedule with DAL
    let dal = runner.dal();
    dal.trigger_schedule().upsert(
        cloacina::models::trigger_schedule::NewTriggerSchedule::new(
            "file_watcher",
            "file_processing",
            Duration::from_secs(2),
        )
    ).await?;

    info!("Trigger registered. Running for 30 seconds...");

    tokio::time::sleep(Duration::from_secs(30)).await;

    runner.shutdown().await?;
    Ok(())
}
```

## Common Trigger Patterns

### Queue Depth Trigger

Fire when a queue exceeds a threshold:

```rust
#[derive(Debug, Clone)]
pub struct QueueDepthTrigger {
    name: String,
    queue_name: String,
    threshold: usize,
    poll_interval: Duration,
}

#[async_trait]
impl Trigger for QueueDepthTrigger {
    // ... other methods

    fn allow_concurrent(&self) -> bool {
        true // Allow parallel queue draining
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let depth = self.get_queue_depth().await;
        if depth >= self.threshold {
            let mut ctx = Context::new();
            ctx.insert("queue_depth", serde_json::json!(depth))?;
            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}
```

### Health Check Trigger

Fire recovery workflow after consecutive failures:

```rust
#[derive(Debug, Clone)]
pub struct HealthCheckTrigger {
    name: String,
    service_name: String,
    failure_threshold: usize,
    consecutive_failures: Arc<AtomicUsize>,
    poll_interval: Duration,
}

#[async_trait]
impl Trigger for HealthCheckTrigger {
    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        if self.check_service_health().await {
            self.consecutive_failures.store(0, Ordering::SeqCst);
            return Ok(TriggerResult::Skip);
        }

        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= self.failure_threshold {
            self.consecutive_failures.store(0, Ordering::SeqCst);
            let mut ctx = Context::new();
            ctx.insert("service_name", serde_json::json!(self.service_name))?;
            ctx.insert("consecutive_failures", serde_json::json!(failures))?;
            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}
```

## Best Practices

### 1. Keep Polls Lightweight

```rust
// Good: Quick check
async fn poll(&self) -> Result<TriggerResult, TriggerError> {
    if file_exists(&self.path).await? {
        Ok(TriggerResult::Fire(Some(ctx)))
    } else {
        Ok(TriggerResult::Skip)
    }
}

// Bad: Heavy processing in poll
async fn poll(&self) -> Result<TriggerResult, TriggerError> {
    let data = download_large_file().await?;  // Don't do this!
    process_data(&data).await?;
    Ok(TriggerResult::Fire(None))
}
```

### 2. Use Context for Deduplication

```rust
// Good: Context identifies the specific item
ctx.insert("filename", json!(filename))?;
ctx.insert("file_hash", json!(hash))?;

// Bad: No identifying information
Ok(TriggerResult::Fire(None))  // All fires look identical!
```

### 3. Choose Concurrent Carefully

- `allow_concurrent = false`: File processing, service recovery
- `allow_concurrent = true`: Queue processing, parallel scaling

### 4. Handle Errors Gracefully

```rust
async fn poll(&self) -> Result<TriggerResult, TriggerError> {
    match self.check_condition().await {
        Ok(true) => Ok(TriggerResult::Fire(None)),
        Ok(false) => Ok(TriggerResult::Skip),
        Err(e) => {
            tracing::warn!("Check failed: {}", e);
            Ok(TriggerResult::Skip)  // Continue polling
        }
    }
}
```

## Python Management

From Python, you can manage triggers (though definition requires Rust):

```python
import cloaca

runner = cloaca.DefaultRunner("sqlite://triggers.db")

# List all triggers
schedules = runner.list_trigger_schedules()
for schedule in schedules:
    print(f"{schedule['trigger_name']}: {schedule['enabled']}")

# Enable/disable triggers
runner.set_trigger_enabled("file_watcher", False)

# View execution history
history = runner.get_trigger_execution_history("file_watcher")
for execution in history:
    print(f"Started: {execution['started_at']}")
```

## Summary

You've learned how to:

1. Implement the `Trigger` trait for custom conditions
2. Register triggers with workflows
3. Pass context from triggers to workflows
4. Use deduplication to prevent duplicate executions
5. Apply common trigger patterns
6. Manage triggers from Python

## Next Steps

- Explore [Tutorial 10 - Advanced Patterns]({{< ref "/tutorials/10-advanced-patterns/" >}}) for combining triggers with other features
- See the [Trigger API Reference]({{< ref "/reference/triggers/" >}}) for complete documentation
- Check `examples/features/event-triggers/` for a full working example
