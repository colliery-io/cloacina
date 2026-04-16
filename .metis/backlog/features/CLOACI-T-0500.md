---
id: computation-graphs-as-workflow
level: task
title: "Computation graphs as workflow tasks — run CGs inside scheduled or triggered workflows"
short_code: "CLOACI-T-0500"
created_at: 2026-04-16T12:41:52.023493+00:00
updated_at: 2026-04-16T12:41:52.023493+00:00
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

# Computation graphs as workflow tasks — run CGs inside scheduled or triggered workflows

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Allow computation graphs to be embedded as tasks inside standard workflows. Today CGs are long-lived reactive processes managed by the ReactiveScheduler, and workflows are DAG-based task pipelines managed by the workflow scheduler. This feature lets a workflow step spin up a computation graph, run it for a bounded duration or until a condition is met, collect its output, and pass it to the next task.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Enables hybrid workflows that mix traditional tasks with reactive computation. Example: a cron-scheduled workflow runs nightly, spins up a CG that processes the last 24h of Kafka data, collects the result, and feeds it into a report generation task. Today these are separate systems requiring manual orchestration.
- **Effort Estimate**: XL

## Acceptance Criteria

- [ ] A workflow task type `computation_graph` that references a named CG
- [ ] CG starts when the task is claimed, stops when the task completes
- [ ] Configurable termination: time-bounded (`run_for: 5m`), event-count-bounded, or condition-based (terminal node returns a "done" signal)
- [ ] Terminal node outputs are captured as the task's output context, available to downstream tasks
- [ ] Works with cron-scheduled and event-triggered workflows
- [ ] CG lifecycle is tied to task lifecycle — if the task is cancelled, the CG shuts down
- [ ] Works in both embedded and server mode

## Implementation Notes

### Design considerations
- The CG task executor would need to: create accumulators, create reactor, run the graph, wait for termination condition, collect outputs, shut down
- This is essentially `accumulator_runtime + reactor.run()` scoped to a task execution instead of running indefinitely
- The reactor needs a "completion" signal — today it runs forever. Options:
  - Terminal node returns a special `Done` variant
  - Fire count limit (`fire_count: 100`)
  - Time limit (`duration: 300s`)
  - External signal (workflow cancellation)
- Accumulator sources for bounded CGs: could be Kafka (consume N messages), polling (run N polls), or WebSocket (accept events for duration)
- Related to T-0499 (accumulator triggers) — both bridge the CG and workflow systems

### Key question
- Should the CG be defined inline in the workflow, or referenced by name from an already-uploaded CG package?

## Status Updates

*To be added during implementation*
