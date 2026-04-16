---
id: accumulators-as-workflow-event
level: task
title: "Accumulators as workflow event triggers"
short_code: "CLOACI-T-0499"
created_at: 2026-04-16T12:41:51.010637+00:00
updated_at: 2026-04-16T12:41:51.010637+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Accumulators as workflow event triggers

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Allow accumulators to trigger standard workflow executions. Today the two systems are separate: workflows are triggered by cron schedules, event triggers, or manual API calls, while accumulators feed computation graphs via the reactive scheduler. This feature bridges them — when an accumulator emits a boundary, it can optionally fire a workflow instead of (or in addition to) feeding a reactor.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Enables event-driven workflows without computation graphs. A Kafka consumer (accumulator) detects a condition and kicks off a multi-step workflow — e.g., an anomaly detector accumulator triggers an incident response workflow. Today users would need to wire a CG terminal node to make an API call back to the server.
- **Effort Estimate**: L

## Acceptance Criteria

- [ ] Accumulator boundary can be configured to trigger a named workflow
- [ ] Boundary data is passed to the workflow as trigger context / input parameters
- [ ] Works with all accumulator types (passthrough, polling, batch, stream)
- [ ] Trigger is idempotent — rapid accumulator emissions don't create duplicate workflow runs (debounce or dedup)
- [ ] Works in both embedded and server mode

## Implementation Notes

### Design considerations
- New trigger type alongside cron and event triggers: `accumulator_trigger`
- The boundary sender could optionally write to the trigger system instead of / alongside the reactor boundary channel
- Alternatively: a special "workflow trigger" terminal node in a CG that bridges the two systems
- Debounce strategy: cooldown period, or deduplicate by boundary content hash
- The accumulator doesn't need to know about workflows — a bridge component watches boundaries and fires triggers

## Status Updates

*To be added during implementation*
