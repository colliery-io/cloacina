---
id: implement-start-continuous
level: task
title: "Implement start_continuous_scheduler in DefaultRunner services"
short_code: "CLOACI-T-0181"
created_at: 2026-03-16T13:23:23.729321+00:00
updated_at: 2026-03-16T13:35:52.766173+00:00
parent: CLOACI-I-0030
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0030
---

# Implement start_continuous_scheduler in DefaultRunner services

## Parent Initiative

[[CLOACI-I-0030]]

## Objective

Wire the continuous scheduler into DefaultRunner's background service lifecycle. Add a `start_continuous_scheduler` method to `services.rs` that assembles the reactive graph from registered data sources and task registrations, creates the `ContinuousScheduler`, runs the startup restore sequence, and spawns the scheduler's `run()` loop as a tokio task. Call it from `start_background_services()` gated by config.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New method `start_continuous_scheduler(&self, handles: &mut RuntimeHandles, shutdown_tx: &broadcast::Sender<()>)` in `services.rs`
- [ ] Method calls `assemble_graph(data_sources, task_registrations)` using registrations from T-0180 fields (drain the `Vec`s under write lock)
- [ ] Creates `ExecutionLedger` and wraps in `Arc<RwLock<...>>`
- [ ] Creates `ContinuousScheduler::new(graph, ledger, config)` with `ContinuousSchedulerConfig` from `DefaultRunnerConfig`
- [ ] Calls `.with_dal(dal)` to enable accumulator state persistence
- [ ] Runs startup restore sequence: `restore_from_persisted_state()`, `restore_pending_boundaries()`, `restore_detector_state()`
- [ ] Registers task implementations via `scheduler.register_task(task)` for each impl from T-0180's `continuous_task_impls`
- [ ] Creates `watch::channel(false)` for shutdown, spawns `scheduler.run(shutdown_rx)` as a tokio task with tracing span
- [ ] Stores `JoinHandle` and `watch::Sender<bool>` in `RuntimeHandles` fields from T-0180
- [ ] `start_background_services()` calls `start_continuous_scheduler` when `self.config.enable_continuous_scheduling()` is true
- [ ] `DefaultRunnerConfig` gains `enable_continuous_scheduling: bool` (default `false`) and corresponding builder method
- [ ] When `enable_continuous_scheduling` is false (default), no continuous scheduler is started and no performance impact

## Implementation Notes

### Technical Approach

**File: `crates/cloacina/src/runner/default_runner/services.rs`**

Follow the exact pattern of `start_trigger_services` (line 352-407):

```rust
async fn start_continuous_scheduler(
    &self,
    handles: &mut super::RuntimeHandles,
    shutdown_tx: &broadcast::Sender<()>,
) -> Result<(), PipelineError> {
    tracing::info!("Starting continuous scheduler");

    // Drain registered data sources and task registrations
    let data_sources = std::mem::take(&mut *self.continuous_data_sources.write().await);
    let task_registrations = std::mem::take(&mut *self.continuous_task_registrations.write().await);
    let task_impls = std::mem::take(&mut *self.continuous_task_impls.write().await);

    // Assemble the reactive graph
    let graph = assemble_graph(data_sources, task_registrations)
        .map_err(|e| PipelineError::Configuration { message: e.to_string() })?;

    // Create execution ledger
    let ledger = Arc::new(parking_lot::RwLock::new(ExecutionLedger::new()));

    // Create continuous scheduler with DAL
    let dal = Arc::new(DAL::new(self.database.clone()));
    let config = ContinuousSchedulerConfig::default(); // or from self.config
    let mut scheduler = ContinuousScheduler::new(graph, ledger, config)
        .with_dal(dal);

    // Startup restore sequence
    scheduler.restore_from_persisted_state().await;
    scheduler.restore_pending_boundaries().await;
    scheduler.restore_detector_state().await;

    // Register task implementations
    for task in task_impls {
        scheduler.register_task(task);
    }

    // Create watch channel for shutdown
    let (continuous_shutdown_tx, continuous_shutdown_rx) = watch::channel(false);

    // Spawn scheduler run loop
    let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
    let continuous_span = self.create_runner_span("continuous_scheduler");
    let continuous_handle = tokio::spawn(async move {
        tokio::select! {
            _fired = scheduler.run(continuous_shutdown_rx) => {
                tracing::info!("Continuous scheduler completed");
            }
            _ = broadcast_shutdown_rx.recv() => {
                tracing::info!("Continuous scheduler shutdown via broadcast");
                let _ = continuous_shutdown_tx.send(true);
            }
        }
    }.instrument(continuous_span));

    // Store handle and shutdown sender
    handles.continuous_scheduler_handle = Some(continuous_handle);
    handles.continuous_shutdown_tx = Some(continuous_shutdown_tx); // note: moved into spawn, need alternate approach
    // (see risk note below about shutdown_tx ownership)

    Ok(())
}
```

**File: `crates/cloacina/src/runner/default_runner/config.rs`**

Add to `DefaultRunnerConfig`:
```rust
enable_continuous_scheduling: bool,  // default: false
```

Add builder method and getter following the existing pattern for `enable_cron_scheduling`, `enable_trigger_scheduling`.

**Integration in `start_background_services()`** (after the trigger services block):
```rust
if self.config.enable_continuous_scheduling() {
    self.start_continuous_scheduler(&mut handles, &shutdown_tx).await?;
}
```

### Dependencies

- T-0180 must be complete (provides the struct fields and registration methods consumed here)

### Risk Considerations

- **shutdown_tx ownership**: The `watch::Sender` is moved into the spawned task for the broadcast-relay pattern. Store a clone in `RuntimeHandles` before spawning so the shutdown path (T-0182) can also signal directly. Alternatively, rely solely on the broadcast channel like `start_trigger_services` does.
- **Empty registrations**: If `enable_continuous_scheduling` is true but no data sources/tasks were registered, `assemble_graph` returns an empty graph. The scheduler's `run()` loop will poll an empty ledger harmlessly. Log a warning but do not error.
- **Restore failures**: The three restore methods log warnings internally and do not return errors. On a fresh database with no persisted state, they are no-ops.

## Status Updates

### 2026-03-16 — Completed
- Added `start_continuous_scheduler()` to services.rs following the exact pattern of `start_trigger_services()`
- Drains registered data sources, task registrations, and task impls from DefaultRunner fields
- Calls `assemble_graph()`, creates `ExecutionLedger` + `ContinuousScheduler` with `ContinuousSchedulerConfig` from runner config
- Wires DAL for persistence, runs full startup restore: init_drain_cursors → restore_pending_boundaries → restore_from_persisted_state → restore_detector_state
- Registers task impls, spawns `scheduler.run()` with tracing span and broadcast shutdown relay
- Gated by `self.config.enable_continuous_scheduling()` in `start_background_services()`
- Config fields `enable_continuous_scheduling` and `continuous_poll_interval` already existed in DefaultRunnerConfig
- Warns if enabled but no sources/tasks registered (empty graph is valid, just harmless)
- Compiles clean
