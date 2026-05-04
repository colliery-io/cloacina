---
id: audit-t4c-dal-runtime-registry
level: task
title: "Audit T4c: DAL/Runtime/Registry visibility downgrades (pub → pub(crate))"
short_code: "CLOACI-T-0565"
created_at: 2026-05-04T20:19:12.054404+00:00
updated_at: 2026-05-04T20:19:12.054404+00:00
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

# Audit T4c: DAL/Runtime/Registry visibility downgrades (pub → pub(crate))

Replaces the "API surface that looks like dead code" bucket of T-0558. These items have zero in-tree callers but read like deliberately-complete admin/introspection surface. Outright deletion would mean re-adding when the next consumer needs them; outright keeping (`pub`) leaks them as committed external API.

## Objective

Downgrade these to `pub(crate)` — preserves them as internal capability, removes them from the external surface, lets the compiler warn if they go truly dead.

## Backlog Item Details

### Type
- [x] Tech Debt — visibility narrowing.

### Priority
- [x] P3 — Low.

### Technical Debt Impact
- **Current Problems**: External API surface includes ~20 methods with no documented use cases. Reviewers / external consumers can't tell which methods are stable contracts.
- **Benefits of Fixing**: External surface shrinks to what's actually committed. Compiler dead-code lint becomes a real signal for these symbols (currently muted because they're `pub`).
- **Risk Assessment**: Medium-low. Some of these may have out-of-tree consumers we don't know about. Downgrade is reversible; user can re-promote on request.

## Acceptance Criteria

### DAL methods → `pub(crate)`

- [ ] `RecoveryEventDAL`: `get_by_workflow` (L143), `get_by_task` (L205), `get_by_type` (L267), `get_workflow_unavailable_events` (L331), `get_recent` (L339).
- [ ] `ExecutionEventDAL`: `list_by_task` (L210), `list_by_type` (L272), `get_recent` (L341), `delete_older_than` (L400), `count_by_workflow` (L462), `count_older_than` (L526).
- [ ] `TaskOutboxDAL::delete_older_than` (`dal/unified/task_outbox.rs:308`).
- [ ] `ContextDAL`: `list` (L327), `update` (L213).

### Runtime introspection → `pub(crate)`

- [ ] `Runtime::all_workflows` (L242), `Runtime::all_triggers` (L282).
- [ ] `Runtime::has_stream_backend` (L430), `Runtime::stream_backend_names` (L448), `Runtime::has_task` (L200) — only own-tests reference; `pub(crate)` retains test access.
- [ ] Type aliases at `runtime.rs:56-65` (`TriggerlessGraphConstructor`, `TaskConstructorFn`, `WorkflowConstructorFn`, `TriggerConstructorFn`) — confirm zero out-of-tree references via grep, then `pub(crate)`.

### `WorkflowRegistryImpl` convenience methods → `pub(crate)`

- [ ] `unregister_workflow_package_by_id`, `exists_by_id`, `exists_by_name`, `get_workflow_package_by_id`, `get_workflow_package_by_name`, `list_packages`. Cloacina-server uses `inspect_package_by_id` + `list_all_packages`, so the rest are internal.
- [ ] `with_strict_validation` (L88), `loaded_package_count` (L104), `total_registered_tasks` (L109).

### `RegistryReconciler` test API

- [ ] `RegistryReconciler::with_graph_scheduler` (`mod.rs:286`) — gate behind `#[cfg(test)]` rather than removing. Production uses `set_graph_scheduler_slot` for late binding; the constructor-binding form is test ergonomic.

### Public-API decisions to land

- [ ] `cloacina-computation-graph::ComputationGraphRegistration::entry_accumulators` field — written but no production reader (reconciler/scheduler use `accumulator_names`). Decide: delete `entry_accumulators` or migrate consumers and delete `accumulator_names`. Document the decision in the commit.
- [ ] `cloacina-computation-graph::GraphResult::completed_empty` (L223) — zero callers. If no near-term consumer is planned, `pub(crate)` then revisit in 90 days.
- [ ] `cloacina-computation-graph::json_to_wire` (L93) — zero callers. Same call.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` green.

## Implementation Notes

### Technical Approach

One commit per subsystem (DAL / Runtime / Registry / CG types). Each commit is a visibility-only change — if the test gates pass, the downgrade is safe.

### Dependencies

None blocking.

### Risk Considerations

- Out-of-tree consumers cannot be enumerated from this repo. If a downgrade breaks a downstream user, surface in PR review and revert that specific symbol to `pub`.

## Status Updates

*To be added during implementation*
