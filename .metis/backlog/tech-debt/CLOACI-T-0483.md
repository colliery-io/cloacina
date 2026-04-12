---
id: extract-servicemanager-from
level: task
title: "Extract ServiceManager from DefaultRunner"
short_code: "CLOACI-T-0483"
created_at: 2026-04-11T14:49:51.972114+00:00
updated_at: 2026-04-11T14:49:51.972114+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

*To be added during implementation*
