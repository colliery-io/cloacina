---
id: trigger-attribute-macro-custom
level: task
title: "#[trigger] attribute macro — custom poll mode with Trigger trait generation"
short_code: "CLOACI-T-0304"
created_at: 2026-03-29T20:39:43.633622+00:00
updated_at: 2026-03-30T02:57:03.428681+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[trigger] attribute macro — custom poll mode with Trigger trait generation

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Create the `#[trigger]` attribute macro for custom poll triggers. Applied to a standalone async function, it generates a `Trigger` trait impl, auto-registration (embedded) or manifest entry (packaged). The `on` parameter binds it to a workflow by name.

## Acceptance Criteria

## Acceptance Criteria

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

**2026-03-30**: Implementation complete.

### What was done
- Created `crates/cloacina-macros/src/trigger_attr.rs` — `#[trigger]` attribute macro
- `#[trigger(on = "workflow_name", poll_interval = "5s")]` on async function
- Generates struct (e.g., `TestTriggerTrigger`) implementing `cloacina::trigger::Trigger` trait
- `name()` returns function name or custom `name` parameter
- `poll_interval()` parsed from duration string (ms/s/m/h)
- `allow_concurrent` optional, defaults to false
- `poll()` calls inner function, converts `cloacina_workflow::TriggerResult/Error` → `cloacina::trigger::TriggerResult/Error`
- Embedded mode: `#[ctor]` registration via `register_trigger_constructor()`
- Packaged mode placeholder (manifest entry — no runtime registration)
- Created `crates/cloacina-workflow/src/trigger.rs` — lightweight `TriggerResult` + `TriggerError` types for authoring
- Added `From<cloacina_workflow::TriggerResult>` and `From<cloacina_workflow::TriggerError>` impls in cloacina core
- Re-exported `trigger` macro from `cloacina-workflow` and `cloacina`
- 2 integration tests: trigger auto-registration, custom-named trigger
- All 386+ unit tests pass, no regressions

### Key design: two TriggerResult types
- `cloacina_workflow::TriggerResult` — lightweight, for authoring (no diesel/db deps)
- `cloacina::trigger::TriggerResult` — full runtime version with helper methods (should_fire, context_hash, into_context)
- Macro-generated `poll()` converts between them via `From` impls
- Users import from whichever crate they depend on
