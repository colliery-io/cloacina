---
id: migrate-examples-and-tutorials-to
level: task
title: "Migrate examples and tutorials to unified #[workflow] + #[trigger] macros"
short_code: "CLOACI-T-0306"
created_at: 2026-03-29T20:39:45.867227+00:00
updated_at: 2026-03-29T20:39:45.867227+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# Migrate examples and tutorials to unified #[workflow] + #[trigger] macros

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Migrate all existing examples and tutorials from `workflow!` / `#[packaged_workflow]` to the unified `#[workflow]` + `#[trigger]` macros. Verify everything compiles, runs, and passes tests.

## Acceptance Criteria

- [ ] All Rust tutorials (01-06) migrated from `workflow!` + loose `#[task]` to `#[workflow]` module pattern
- [ ] All packaged workflow examples migrated from `#[packaged_workflow]` to `#[workflow]` with `features = ["packaged"]`
- [ ] Event triggers example migrated from manual `Trigger` trait impl to `#[trigger(on = "...")]`
- [ ] Cron scheduling example migrated from `runner.register_cron_workflow()` to `#[trigger(on = "...", cron = "...")]`
- [ ] Packaged triggers example migrated to `#[trigger]` in packaged mode
- [ ] All `angreal demos` tasks still pass
- [ ] All unit and integration tests pass
- [ ] Python tutorials unaffected (Python uses `@cloaca.trigger` which is already unified)

## Implementation Notes

### Scope
- `examples/tutorials/01-basic-workflow/` through `06-*`
- `examples/features/simple-packaged/`
- `examples/features/packaged-workflows/`
- `examples/features/event-triggers/`
- `examples/features/cron-scheduling/`
- `examples/features/packaged-triggers/`

### Depends on
- T-0302, T-0303, T-0304, T-0305 (all macros must be implemented first)

## Status Updates

*To be added during implementation*
