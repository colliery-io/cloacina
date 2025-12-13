---
id: event-triggers-user-defined
level: task
title: "Event Triggers - User-defined trigger functions for workflow activation"
short_code: "CLOACI-T-0046"
created_at: 2025-12-13T20:00:15.001926+00:00
updated_at: 2025-12-13T21:45:26.322927+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Event Triggers - User-defined trigger functions for workflow activation

## Objective

Allow workflows to define custom trigger functions that run on a polling cadence and activate the workflow when conditions are met. This enables event-driven workflow activation based on external state (files, APIs, queues, databases, etc.).

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: React to external events (file arrival, API state changes, queue messages) without manual triggering or cron schedules
- **Business Value**: Enables true event-driven architectures, reduces latency between event and response
- **Effort Estimate**: L

## Concept

### Trigger Function Contract

```rust
/// Trigger function signature
/// - Takes optional configuration/arguments
/// - Returns (should_fire: bool, optional_context: Option<Context>)
async fn my_trigger() -> (bool, Option<Context<Value>>) {
    // Check some external condition
    if new_file_exists("/inbox/") {
        let ctx = Context::new();
        ctx.insert("file_path", "/inbox/data.csv");
        (true, Some(ctx))
    } else {
        (false, None)
    }
}
```

### Trigger Behavior

1. **Polling Loop**: Trigger function runs on a configurable cadence (e.g., every 5 seconds)
2. **On False**: Sleep until next poll interval
3. **On True**: Fire the workflow with the returned context (if any)

### Context as Deduplication Key

The returned context serves two purposes:
1. **Initial workflow context**: Inject relevant data (file path, message ID, etc.)
2. **Deduplication key**: Prevent duplicate workflow runs for the same trigger event
   - Default: Same context = don't fire again until previous run completes
   - Configurable: Allow concurrent runs with same context

### Proposed Interface

```rust
// Define a trigger function
#[trigger(
    poll_interval = "5s",
    allow_concurrent = false,  // default: prevent duplicate runs
)]
async fn file_watcher_trigger() -> (bool, Option<Context<Value>>) {
    // ... check for new files
}

// Attach trigger to workflow
let workflow = workflow! {
    name: "file_processor",
    trigger: file_watcher_trigger,
    tasks: [process_file, archive_file]
};

// Or via builder
let workflow = Workflow::builder("file_processor")
    .trigger(file_watcher_trigger)
    .poll_interval(Duration::from_secs(5))
    .build();
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Trigger trait for defining trigger functions (manual implementation, macro deferred)
- [x] Trigger functions run on configurable poll interval
- [x] Workflow fires when trigger returns `Fire`
- [x] Returned context is passed to workflow as initial context
- [x] Context-based deduplication prevents duplicate runs (configurable via allow_concurrent)
- [x] Trigger state persisted for recovery after restart
- [x] Graceful shutdown stops trigger polling
- [x] Documentation with examples (file watcher, queue monitor, health check)
- [x] Python bindings for trigger management (list, get, enable/disable, history)
- [x] Rust example demonstrating three trigger patterns
- [ ] Multiple triggers can be attached to same workflow (OR logic) - deferred
- [ ] `#[trigger()]` macro for code generation - deferred to future iteration

## Implementation Notes

### Technical Approach

1. **Trigger Registry**: Similar to task registry, register trigger functions
2. **Trigger Scheduler**: New component that polls triggers on their cadence
3. **Deduplication Store**: Track active/recent trigger contexts to prevent duplicates
4. **Integration**: When trigger fires, call existing `schedule_workflow_execution` with context

### Example Use Cases

- **File watcher**: Check for new files in a directory
- **API poller**: Check if external API returns specific state
- **Queue consumer**: Check for messages in a queue
- **Database sensor**: Check for new/changed rows in a table
- **Health check**: Monitor service and trigger alerts

### Dependencies
- Workflow scheduler (existing)
- Context system (existing)
- New: Trigger registry and scheduler

### Design Questions to Resolve

1. How to handle trigger errors? (retry? backoff? alert?)
2. Should triggers support async initialization (e.g., establish connections)?
3. How to pass configuration to trigger functions (env vars, config file, context)?
4. Should there be built-in triggers (file, cron, http) or only user-defined?
5. How does this interact with existing cron scheduler?

### Risk Considerations
- Polling overhead at scale (many triggers polling frequently)
- Deduplication storage growth over time
- Trigger function errors blocking the polling loop
- Resource cleanup if triggers hold connections

## Status Updates

- **2025-12-13**: Created backlog item for design exploration
- **2025-12-13**: Implementation complete:
  - Core types: Trigger trait, TriggerResult enum, TriggerError
  - Database: trigger_schedules and trigger_executions tables with migrations
  - TriggerScheduler service with configurable poll intervals
  - DefaultRunner integration with background trigger scheduling
  - DAL layer with CRUD operations for both tables
  - Rust example: FileWatcherTrigger, QueueDepthTrigger, HealthCheckTrigger
  - Python bindings: list_trigger_schedules, get_trigger_schedule, set_trigger_enabled, get_trigger_execution_history
  - Documentation: Tutorial 09 - Event Triggers
  - All 210 unit tests passing, all 168 integration tests passing
  - Demo verified working via `angreal demos event-triggers`
