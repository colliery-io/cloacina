---
id: trigger-attribute-macro-custom
level: task
title: "#[trigger] attribute macro — custom poll mode with Trigger trait generation"
short_code: "CLOACI-T-0304"
created_at: 2026-03-29T20:39:43.633622+00:00
updated_at: 2026-03-29T20:39:43.633622+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[trigger] attribute macro — custom poll mode with Trigger trait generation

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Create the `#[trigger]` attribute macro for custom poll triggers. Applied to a standalone async function, it generates a `Trigger` trait impl, auto-registration (embedded) or manifest entry (packaged). The `on` parameter binds it to a workflow by name.

## Acceptance Criteria

- [ ] `#[trigger(on = "workflow_name", poll_interval = "5s")]` on an async function
- [ ] Generates a struct implementing the `Trigger` trait — `name()`, `poll_interval()`, `allow_concurrent()`, `poll()`
- [ ] `on` parameter binds the trigger to a workflow name (validated at registration time, not compile time)
- [ ] `poll_interval` parsed from duration string ("100ms", "5s", "2m", "1h")
- [ ] `allow_concurrent` optional (defaults to false)
- [ ] Embedded mode: generates `#[ctor]` registration into global trigger registry + trigger schedule DAL upsert
- [ ] Packaged mode: generates trigger entry in manifest `triggers` array (name, workflow, poll_interval, type="custom")
- [ ] Function signature validated: must be `async fn() -> Result<TriggerResult, TriggerError>`
- [ ] Multiple `#[trigger]` functions can target the same workflow
- [ ] Unit tests for macro expansion, integration test with trigger firing a workflow

## Implementation Notes

### Files to create/modify
- `crates/cloacina-macros/src/trigger.rs` — new proc macro implementation
- `crates/cloacina-macros/src/lib.rs` — register `#[trigger]` proc macro
- `crates/cloacina-workflow/src/lib.rs` — re-export `TriggerResult`, `TriggerError` for workflow authors

### Key design points
- Generates a private struct named after the function (e.g., `InboxWatcherTrigger`)
- The function body becomes the `poll()` method
- `on` is stored as a const string — the scheduler uses it to look up the workflow at runtime
- Same `cfg(feature = "packaged")` branching as `#[workflow]`

### Depends on
- T-0302 (need `#[workflow]` to exist for end-to-end testing)

## Status Updates

*To be added during implementation*
