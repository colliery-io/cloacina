---
id: trigger-cron-mode-declarative
level: task
title: "#[trigger] cron mode — declarative schedule, built-in poll function"
short_code: "CLOACI-T-0305"
created_at: 2026-03-29T20:39:45.020591+00:00
updated_at: 2026-03-29T20:39:45.020591+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[trigger] cron mode — declarative schedule, built-in poll function

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Extend the `#[trigger]` macro (from T-0304) with cron mode. When `cron = "..."` is provided instead of `poll_interval`, the macro generates a trigger with a built-in poll function that checks the cron expression against the current time. No function body needed — the schedule IS the logic.

## Acceptance Criteria

- [ ] `#[trigger(on = "workflow_name", cron = "0 2 * * *", timezone = "UTC")]` — standalone, no function body
- [ ] Framework provides the built-in poll function (checks cron expression against current time)
- [ ] `timezone` optional, defaults to "UTC"
- [ ] Catchup policy configurable (optional parameter, defaults from workflow config)
- [ ] Generates a `Trigger` trait impl with cron-aware `poll()` that returns `Fire` when the schedule is due
- [ ] Embedded mode: registers cron schedule in the cron schedule DAL (same as `runner.register_cron_workflow()`)
- [ ] Packaged mode: generates trigger entry in manifest with `trigger_type = "cron"` and `cron_expression` in config
- [ ] Compile-time validation of cron expression syntax
- [ ] Coexists with custom poll triggers — both use `#[trigger]`, differentiated by parameters
- [ ] Integration test: cron trigger fires a workflow on schedule

## Implementation Notes

### Files to modify
- `crates/cloacina-macros/src/trigger.rs` — add cron branch (detect `cron` param vs `poll_interval`)
- `crates/cloacina/src/trigger/` — built-in `CronTrigger` struct that wraps a cron expression

### Key design points
- Macro detects mode by which parameters are present: `poll_interval` → custom, `cron` → cron
- Cron poll function: parse expression, compare `next_run_at` with `now`, return `Fire` or `Skip`
- Reuse existing `cron` crate for expression parsing (already a dependency)

### Depends on
- T-0304 (custom trigger mode must exist first)

## Status Updates

*To be added during implementation*
