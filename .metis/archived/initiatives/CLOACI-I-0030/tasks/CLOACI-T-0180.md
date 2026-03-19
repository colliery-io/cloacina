---
id: add-continuous-scheduling-fields
level: task
title: "Add continuous scheduling fields and registration API to DefaultRunner"
short_code: "CLOACI-T-0180"
created_at: 2026-03-16T13:23:22.646708+00:00
updated_at: 2026-03-16T13:33:32.549624+00:00
parent: CLOACI-I-0030
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0030
---

# Add continuous scheduling fields and registration API to DefaultRunner

## Parent Initiative

[[CLOACI-I-0030]]

## Objective

Add the struct fields and registration methods to `DefaultRunner` so that users can register data sources and continuous task declarations before the runner starts. These registrations are stored in-memory and consumed later by `start_continuous_scheduler` (T-0181) during background service startup.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DefaultRunner` struct gains field `continuous_data_sources: Arc<RwLock<Vec<DataSource>>>` for storing registered data sources
- [ ] `DefaultRunner` struct gains field `continuous_task_registrations: Arc<RwLock<Vec<ContinuousTaskRegistration>>>` for storing task declarations
- [ ] `RuntimeHandles` gains field `continuous_scheduler_handle: Option<JoinHandle<()>>` for the scheduler task handle
- [ ] `RuntimeHandles` gains field `continuous_shutdown_tx: Option<watch::Sender<bool>>` for the watch-based shutdown signal
- [ ] New fields are initialized to empty/None in `with_config()` and included in `Clone` impl
- [ ] Public method `register_data_source(&self, source: DataSource)` pushes to `continuous_data_sources` under write lock
- [ ] Public method `register_continuous_task(&self, reg: ContinuousTaskRegistration)` pushes to `continuous_task_registrations` under write lock
- [ ] Public method `register_continuous_task_impl(&self, task: Arc<dyn Task>)` stores a task implementation for later `scheduler.register_task()` during startup
- [ ] All three methods can be called before `start()` without panic
- [ ] Code compiles with no new warnings

## Implementation Notes

### Technical Approach

**File: `crates/cloacina/src/runner/default_runner/mod.rs`**

Add fields to `DefaultRunner`:
```rust
pub(super) continuous_data_sources: Arc<RwLock<Vec<DataSource>>>,
pub(super) continuous_task_registrations: Arc<RwLock<Vec<ContinuousTaskRegistration>>>,
pub(super) continuous_task_impls: Arc<RwLock<Vec<Arc<dyn cloacina_workflow::Task>>>>,
```

Add fields to `RuntimeHandles`:
```rust
pub(super) continuous_scheduler_handle: Option<tokio::task::JoinHandle<()>>,
pub(super) continuous_shutdown_tx: Option<watch::Sender<bool>>,
```

Initialize in `with_config()`:
```rust
continuous_data_sources: Arc::new(RwLock::new(Vec::new())),
continuous_task_registrations: Arc::new(RwLock::new(Vec::new())),
continuous_task_impls: Arc::new(RwLock::new(Vec::new())),
```

Initialize in `RuntimeHandles` literal:
```rust
continuous_scheduler_handle: None,
continuous_shutdown_tx: None,
```

Add registration methods on `impl DefaultRunner`:
```rust
pub async fn register_data_source(&self, source: DataSource) {
    self.continuous_data_sources.write().await.push(source);
}

pub async fn register_continuous_task(&self, reg: ContinuousTaskRegistration) {
    self.continuous_task_registrations.write().await.push(reg);
}

pub async fn register_continuous_task_impl(&self, task: Arc<dyn cloacina_workflow::Task>) {
    self.continuous_task_impls.write().await.push(task);
}
```

Update `Clone` impl to clone the three new `Arc<RwLock<...>>` fields.

**Types needed** (already exist in codebase):
- `DataSource` from `crate::continuous::datasource`
- `ContinuousTaskRegistration` from `crate::continuous::graph`
- `cloacina_workflow::Task` trait

### Dependencies

- No blockers. Existing continuous scheduling types (`DataSource`, `ContinuousTaskRegistration`, `assemble_graph`) are already implemented.

### Risk Considerations

- Registration methods take `&self` (not `&mut self`) because `DefaultRunner` is behind `Arc` in typical usage. The `RwLock` provides interior mutability. This matches the existing pattern for `cron_scheduler`, `trigger_scheduler`, etc.

## Status Updates

### 2026-03-16 — Completed
- Added 3 fields to DefaultRunner: `continuous_data_sources`, `continuous_task_registrations`, `continuous_task_impls` (all `Arc<RwLock<Vec<...>>>`)
- Added 2 fields to RuntimeHandles: `continuous_scheduler_handle`, `continuous_shutdown_tx`
- Added 3 registration methods: `register_data_source()`, `register_continuous_task()`, `register_continuous_task_impl()`
- Updated both constructors (mod.rs `with_config()` and config.rs builder `build()`) to initialize new fields
- Updated Clone impl to clone new fields
- Compiles clean
