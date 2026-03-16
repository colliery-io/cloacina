---
id: server-phase-2-continuous
level: initiative
title: "Server Phase 2: Continuous Scheduling in Server Mode"
short_code: "CLOACI-I-0030"
created_at: 2026-03-16T01:32:33.428932+00:00
updated_at: 2026-03-16T13:23:13.346037+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: S
initiative_id: server-phase-2-continuous
---

# Server Phase 2: Continuous Scheduling in Server Mode Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation)
**Blocks**: None (parallel with Phase 3+)

## Context

ContinuousScheduler (I-0023/I-0024/I-0025, all completed) exists as a full reactive scheduling engine but is not wired into DefaultRunner's background services. Config fields exist (`enable_continuous_scheduling`, `continuous_poll_interval`) but `start_background_services()` doesn't start the ContinuousScheduler.

## Goals

- Wire ContinuousScheduler into DefaultRunner as a background service
- Graph assembly from loaded workflow packages at startup
- Full startup restore sequence (drain cursors → boundaries → watermarks → detector state)
- Graceful shutdown via existing watch-channel pattern
- ContinuousScheduler runs when `enable_continuous_scheduling = true` in config

## Detailed Design

### What needs to happen in `start_background_services()`

The existing method in `runner/default_runner/services.rs` starts: TaskScheduler, CronScheduler, CronRecovery, RegistryReconciler, TriggerScheduler — each gated by a config flag. ContinuousScheduler needs the same treatment:

1. Check `self.config.enable_continuous_scheduling()`
2. Create `ExecutionLedger` (shared with the scheduler)
3. Assemble `DataSourceGraph` from loaded workflow packages (need to extract DataSource + ContinuousTaskRegistration from the registry)
4. Create `ContinuousScheduler` with graph, ledger, config
5. Wire DAL for persistence: `scheduler.with_dal(dal)`
6. Run startup restore: `init_drain_cursors()` → `restore_pending_boundaries()` → `restore_from_persisted_state()` → `restore_detector_state()`
7. Spawn scheduler.run() as tokio task with shutdown signal
8. Store handle in RuntimeHandles

### Graph assembly challenge

The current `assemble_graph()` takes `Vec<DataSource>` and `Vec<ContinuousTaskRegistration>`. These don't exist in the workflow package format yet — packages define regular workflows with `@task` / `#[task]` decorators. The continuous task registrations need to come from somewhere.

**For this phase**: Support programmatic registration via `DefaultRunner` API. The serve command or user code registers data sources and continuous task declarations before starting. Package-based registration is a future enhancement.

```rust
// API on DefaultRunner:
runner.register_data_source(source);
runner.register_continuous_task(registration);
// Then on start, if enable_continuous, assemble graph from registered items
```

### Shutdown

ContinuousScheduler already uses `watch::Receiver<bool>` for shutdown — same pattern as the existing scheduler. Create a `watch::channel`, give the receiver to the continuous scheduler, and send `true` on the sender when the global shutdown fires.

## Implementation Plan

- [ ] Add `ContinuousScheduler` + `ExecutionLedger` fields to `DefaultRunner` struct (behind `Arc<RwLock<Option<...>>>`)
- [ ] Add `register_data_source()` and `register_continuous_task()` methods on DefaultRunner
- [ ] Add `start_continuous_scheduler()` method in services.rs, gated by `enable_continuous_scheduling`
- [ ] Startup restore sequence inside `start_continuous_scheduler()`
- [ ] Shutdown: send to continuous scheduler's watch channel on global shutdown
- [ ] Integration test: DefaultRunner with continuous scheduling enabled starts and stops cleanly
