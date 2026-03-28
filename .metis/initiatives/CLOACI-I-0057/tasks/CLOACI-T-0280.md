---
id: scheduler-wiring-cron-trigger
level: task
title: "Scheduler wiring — cron + trigger schedulers from reconciled packages"
short_code: "CLOACI-T-0280"
created_at: 2026-03-28T15:30:08.209216+00:00
updated_at: 2026-03-28T15:30:08.209216+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# Scheduler wiring — cron + trigger schedulers from reconciled packages

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Wire the `CronScheduler` and `TriggerScheduler` into the daemon so that after the reconciler loads packages, their cron schedules and triggers are active and polling. When packages define cron expressions or triggers in their manifests, the daemon should create the corresponding DB schedule records and start polling.

## Acceptance Criteria

- [ ] `CronScheduler` started as a tokio task in the daemon, polls at `--poll-interval` (default 50ms)
- [ ] `TriggerScheduler` started as a tokio task in the daemon
- [ ] After reconciler loads a package with cron schedules, `CronSchedule` records are created in SQLite
- [ ] After reconciler loads a package with triggers, `TriggerSchedule` records are created in SQLite
- [ ] Cron schedules fire workflows at the expected times
- [ ] Triggers poll and fire workflows when conditions are met
- [ ] Catchup policy defaults to `run_all` (archive learning)
- [ ] End-to-end test: drop a package with cron schedule into watch dir → cron fires → workflow executes

## Implementation Notes

### Files to modify
- `crates/cloacinactl/src/commands/daemon.rs` — start schedulers after reconciler
- May need to extend reconciler to create `CronSchedule` and `TriggerSchedule` DAL records from manifest data

### Key design points
- The `DefaultRunner` already has `enable_cron_scheduling` and `enable_trigger_scheduling` config flags — use those
- Cron schedules from packages: may need a `CronDefinitionV2` in ManifestV2 (similar to how we added triggers in I-0056)
- Archive learnings: 50ms poll interval, `run_all` catchup policy

### Depends on
- T-0278 (daemon subcommand)
- T-0279 (directory watcher — to trigger reconciliation on new packages)

## Status Updates

*To be added during implementation*
