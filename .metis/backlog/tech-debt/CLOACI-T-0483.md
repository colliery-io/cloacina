---
id: extract-servicemanager-from
level: task
title: "Extract ServiceManager from DefaultRunner"
short_code: "CLOACI-T-0483"
created_at: 2026-04-11T14:49:51.972114+00:00
updated_at: 2026-04-20T11:21:58.045091+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Extract ServiceManager from DefaultRunner

## Objective

Replace the 6 `Arc<RwLock<Option<Arc<...>>>>` service fields on `DefaultRunner` with a `ServiceManager` holding `Vec<Box<dyn BackgroundService>>` with `start_all()`/`shutdown_all()`.

## Review Finding References

EVO-004, LEG-012 (REC-013)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `trait BackgroundService: Send + Sync` with `start()`, `shutdown()`, `name()`
- [ ] Each service (`TaskScheduler`, `CronRecovery`, `RegistryReconciler`, `ReactiveScheduler`) implements the trait
- [ ] `DefaultRunner` field count reduced to ~5 (runtime, database, config, scheduler, service_manager)
- [ ] Startup and shutdown ordering preserved

## Implementation Notes

### Key Files
- `crates/cloacina/src/runner/default.rs`

### Dependencies
None. Independent.

## Status Updates

### 2026-04-20 — Implementation complete

**Design**

- New `crates/cloacina/src/runner/default_runner/service_manager.rs`:
  - `trait BackgroundService: Send + Sync` with `name()`, `start(broadcast::Receiver<()>)`, `shutdown()`.
  - `ServiceManager` owns `Vec<Box<dyn BackgroundService>>`, a broadcast `shutdown_tx`, and typed slots (`cron_recovery`, `workflow_registry`, `registry_reconciler`, `unified_scheduler`, `reactive_scheduler`) so external accessors keep working.
  - Wrapper impls: `TaskSchedulerService`, `UnifiedSchedulerService`, `CronRecoveryServiceWrapper`, `RegistryReconcilerService`, `StaleClaimSweeperService`. Each stores its `Option<JoinHandle<()>>` + per-service `watch::Sender<bool>` and propagates the broadcast shutdown.

**DefaultRunner reduction**

Fields went from 11 → **5**: `runtime`, `database`, `config`, `scheduler`, `service_manager`. Removed: `runtime_handles` + 5 `Arc<RwLock<Option<Arc<...>>>>` service slots + the `RuntimeHandles` struct.

**Call sites updated**

- `mod.rs`: `shutdown()` delegates to `service_manager.shutdown_all()` → `database.close()`. `unified_scheduler()` and `set_reactive_scheduler()` read/write through `ServiceManager`.
- `services.rs`: `start_background_services()` builds each service, registers wrappers with the manager, then calls `start_all()`. Preserves original registration order so startup + shutdown ordering is unchanged.
- `cron_api.rs`: `get_workflow_registry` / `get_registry_reconciler_status` read from `service_manager` slots.
- `config.rs` builder: `DefaultRunner` construction simplified to 5 fields.

**Dead internal API removed**

- Deleted `get_registry_reconciler_status()` on `DefaultRunner` and the `registry_reconciler` typed slot on `ServiceManager`. The original slot was always `None` (reconciler is consumed by `start_reconciliation_loop`) and the accessor had zero callers.
- Reactive scheduler slot kept as `Arc<RwLock<Option<...>>>` inside the manager — genuinely needed: `set_reactive_scheduler()` is called by cloacina-server *after* runner construction, and the reconciler (already spawned) reads through the shared slot.
- Stale-claim sweeper now tracked and awaited via its wrapper's `JoinHandle` on shutdown, instead of the previous `drop(sweeper_handle)`. Minor behavior improvement — deterministic shutdown for the sweeper.

**Verification**

- `cargo check --workspace --no-default-features --features cloacina/postgres` → clean.
- `cargo test -p cloacina --lib ... runner::default_runner::` → 9/9 pass.
- `angreal cloacina unit` → 696/696 pass.

### Acceptance criteria

- [x] `trait BackgroundService: Send + Sync` with `start()`, `shutdown()`, `name()`
- [x] Each service implements the trait (task scheduler, unified scheduler, cron recovery, registry reconciler, stale-claim sweeper)
- [x] `DefaultRunner` field count reduced to ~5 (runtime, database, config, scheduler, service_manager)
- [x] Startup and shutdown ordering preserved (registration order = start order; reverse for shutdown)
