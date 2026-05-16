# Event Triggers Example

This example demonstrates Cloacina's event trigger system for automatically executing workflows based on custom conditions. Unlike cron scheduling (time-based), event triggers poll user-defined functions and fire workflows when specific conditions are met.

## Overview

Event triggers enable **event-driven workflow execution** by:
- Polling user-defined conditions at configurable intervals
- Firing workflows with optional context when conditions are met
- Deduplicating concurrent executions based on context hash
- Providing audit trails for all trigger activations

## Key Concepts

### The `#[trigger]` Macro

Define a trigger by annotating an async function. The macro generates
the underlying `Trigger` impl + a zero-arg constructor, and submits the
constructor to the inventory crate so `DefaultRunner` auto-registers it
on startup.

```rust
use cloacina::trigger;
use cloacina_workflow::{Context, TriggerError, TriggerResult};

#[trigger(
    on = "file_processing_workflow",
    poll_interval = "2s",
    allow_concurrent = false,
    name = "file_watcher",
)]
async fn file_watcher() -> Result<TriggerResult, TriggerError> {
    // poll body — returns Skip or Fire(Some(ctx))
}
```

Attributes:

| key | meaning |
|-----|---------|
| `on` | name of the workflow to dispatch when the trigger fires |
| `poll_interval` | how often the scheduler polls this trigger (`"100ms"`, `"5s"`, `"2m"`, `"1h"`) |
| `cron` | mutually exclusive with `poll_interval`; cron expression for time-based firing |
| `allow_concurrent` | `false` (default) deduplicates by context hash; `true` lets independent executions run in parallel |
| `name` | optional override; defaults to the function name |

### TriggerResult

The `poll()` body returns one of:
- `TriggerResult::Skip` — don't fire, continue polling
- `TriggerResult::Fire(Option<Context>)` — fire the workflow with optional context

### Stateful Triggers

The macro generates a unit struct, so the poll body has no `self`. Any
state lives in module-level statics:

```rust
static FILE_POLL_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[trigger(on = "file_processing_workflow", poll_interval = "2s")]
async fn file_watcher() -> Result<TriggerResult, TriggerError> {
    let count = FILE_POLL_COUNTER.fetch_add(1, Ordering::SeqCst);
    // ... use count to drive synthetic behaviour ...
    Ok(TriggerResult::Skip)
}
```

### Context Passing

Triggers can pass context data to workflows:

```rust
let mut ctx = Context::new();
ctx.insert("filename", serde_json::json!(filename))?;
Ok(TriggerResult::Fire(Some(ctx)))
```

### Deduplication

When `allow_concurrent = false`, the scheduler hashes the context. If an
execution with the same context hash is already running, the trigger
won't fire again until it completes.

### Wiring to schedules

`#[trigger]` registers the *trigger* with the runtime; it does NOT
persist the schedule row that the unified scheduler polls. For
in-process examples, you do that explicitly through the unified
scheduler:

```rust
let scheduler = runner.unified_scheduler().await.unwrap();
let runtime = runner.runtime();
for (name, workflow) in [("file_watcher", "file_processing_workflow")] {
    let trigger = runtime.get_trigger(name).unwrap();
    scheduler.register_trigger(trigger.as_ref(), workflow).await?;
}
```

For packaged workflows (`.cloacina` cdylibs), the registry reconciler
handles schedule rows automatically when the package loads.

### Deduplication

When `allow_concurrent = false`, the trigger scheduler prevents duplicate executions by hashing the context. If an execution with the same context hash is already running, the trigger won't fire again until it completes.

## Triggers in This Example

### 1. File Watcher Trigger

Polls for new files in a simulated directory and triggers file processing.

- **Poll Interval**: 2 seconds
- **Concurrent**: No (prevents duplicate processing of same file)
- **Workflow**: `file_processing_workflow`

### 2. Queue Depth Trigger

Monitors a simulated message queue and fires when depth exceeds a threshold.

- **Poll Interval**: 3 seconds
- **Threshold**: 10 messages
- **Concurrent**: Yes (allows parallel queue draining)
- **Workflow**: `queue_processing_workflow`

### 3. Health Check Trigger

Monitors service health and triggers recovery after consecutive failures.

- **Poll Interval**: 2 seconds
- **Failure Threshold**: 3 consecutive failures
- **Concurrent**: No (prevents concurrent recovery attempts)
- **Workflow**: `service_recovery_workflow`

## Running the Example

```bash
cd examples/features/event-triggers
cargo run
```

### Expected Output

```
Starting Event Triggers Example
================================
DefaultRunner initialized with trigger scheduling enabled
Workflows registered successfully
Registered: file_watcher trigger
Registered: queue_monitor trigger
Registered: service_health trigger
Triggers registered successfully

Event triggers are now active!
- File Watcher: polls every 2s for new files
- Queue Monitor: polls every 3s, fires when depth > 10
- Health Check: polls every 2s, fires after 3 failures

Watch the logs to see triggers fire and workflows execute...
Press Ctrl+C to shutdown gracefully

Running trigger scheduler for 60 seconds...

FileWatcherTrigger: Found new file 'data_file_1234567890.csv' in '/data/inbox'
Validating file: data_file_1234567890.csv
File 'data_file_1234567890.csv' validated successfully
Processing 100 records from 'data_file_1234567890.csv'
...
```

## Project Structure

```
event-triggers/
├── Cargo.toml          # Dependencies and configuration
├── README.md           # This file
└── src/
    ├── main.rs         # Runner setup and workflow registration
    ├── tasks.rs        # Task definitions for all workflows
    └── triggers.rs     # Trigger implementations
```

## Configuration

The `DefaultRunnerConfig` includes trigger-specific settings:

```rust
let mut config = DefaultRunnerConfig::default();
config.enable_trigger_scheduling = true;        // Enable trigger scheduler
config.trigger_base_poll_interval = Duration::from_secs(1);  // Base poll interval
config.trigger_poll_timeout = Duration::from_secs(10);       // Timeout per trigger poll
```

## Database Tables

The trigger system uses two tables:

### `trigger_schedules`

Persists trigger configuration for recovery across restarts:
- `trigger_name` - Unique trigger identifier
- `workflow_name` - Associated workflow to fire
- `poll_interval_ms` - Poll interval in milliseconds
- `allow_concurrent` - Whether concurrent executions are allowed
- `enabled` - Whether the trigger is active
- `last_poll_at` - Timestamp of last poll

### `trigger_executions`

Audit trail and deduplication:
- `trigger_name` - Which trigger fired
- `context_hash` - Hash of context for deduplication
- `pipeline_execution_id` - Linked workflow execution
- `started_at` / `completed_at` - Execution timestamps

## Best Practices

### 1. Keep Polls Lightweight

The `poll()` function should be fast and non-blocking. Heavy operations should happen in the workflow tasks, not in the trigger.

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

Pass meaningful data in the context to enable proper deduplication:

```rust
// Good: Context identifies the specific file
ctx.insert("filename", json!(filename))?;
ctx.insert("file_hash", json!(hash))?;

// Bad: No identifying information
Ok(TriggerResult::Fire(None))  // All fires look identical!
```

### 3. Choose Concurrent Carefully

- `allow_concurrent = false`: For operations that shouldn't overlap (file processing, service recovery)
- `allow_concurrent = true`: For operations that can run in parallel (queue processing, scaling)

### 4. Handle Errors Gracefully

Errors in `poll()` are logged and polling continues on the next interval. Design triggers to be resilient:

```rust
async fn poll(&self) -> Result<TriggerResult, TriggerError> {
    match self.check_condition().await {
        Ok(true) => Ok(TriggerResult::Fire(None)),
        Ok(false) => Ok(TriggerResult::Skip),
        Err(e) => {
            // Log and continue polling
            tracing::warn!("Check failed: {}", e);
            Ok(TriggerResult::Skip)
        }
    }
}
```

## Comparison with Cron Scheduling

| Feature | Event Triggers | Cron Scheduling |
|---------|---------------|-----------------|
| Activation | Condition-based | Time-based |
| Poll Logic | User-defined | Cron expression |
| Context | Dynamic from trigger | Static |
| Deduplication | Context hash | Time window |
| Use Case | Reactive workflows | Scheduled jobs |

## Related Examples

- `cron-scheduling/` - Time-based workflow scheduling
- `complex-dag/` - Complex task dependencies
- `multi-tenant/` - Schema-based isolation
