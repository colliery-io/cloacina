---
id: c4-level-3-component-diagrams
level: task
title: "C4 Level 3 — Component Diagrams: Scheduling Subsystem"
short_code: "CLOACI-T-0094"
created_at: 2026-03-13T14:29:59.382865+00:00
updated_at: 2026-03-13T16:35:38.247850+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 3 — Component Diagrams: Scheduling Subsystem

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 3 (Component) diagram and documentation for the Scheduling Subsystem — cron-based scheduling, event-driven triggers, trigger rule evaluation, missed execution recovery, and timezone handling.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Mermaid C4 Component diagram for the Scheduling Subsystem
- [ ] Components: CronScheduler, CronEvaluator, CronRecovery, TriggerScheduler, TriggerRules
- [ ] Cron scheduling flow: expression parsing → next-fire calculation → execution → recovery
- [ ] Event trigger flow: event received → rule evaluation (All/Any/None/TaskSuccess/ContextValue) → workflow trigger
- [ ] Timezone handling documented
- [ ] Missed execution recovery mechanism documented
- [ ] All components verified against source in `crates/cloacina/src/scheduling/`

## Implementation Notes

### Components to Document
- **CronScheduler** (`crates/cloacina/src/scheduling/cron/`) — time-based workflow triggers
- **CronEvaluator** — cron expression parsing, next-fire calculation
- **CronRecovery** — missed execution detection and catchup
- **TriggerScheduler** (`crates/cloacina/src/scheduling/triggers/`) — event-based workflow triggers
- **TriggerRules** — composable condition evaluation: All, Any, None, TaskSuccess, ContextValue

## Status Updates

### Completed 2026-03-13

**Created:** `docs/content/explanation/architecture/c4-scheduling.md`

**Components:** CronScheduler, CronEvaluator, CronRecoveryService, TriggerScheduler, Trigger trait, TriggerRules
- Cron scheduling flow with guaranteed execution (saga pattern)
- Event trigger flow with context deduplication
- Trigger rules: All/Any/None with TaskSuccess/TaskFailed/TaskSkipped/ContextValue conditions
- Timezone handling and DST documented
- Recovery service documented with config defaults
- Sequence diagrams for both cron and trigger flows

**Build:** 99 pages, clean
