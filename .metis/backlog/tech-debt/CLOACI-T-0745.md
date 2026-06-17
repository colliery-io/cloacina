---
id: scheduler-dispatch-throughput
level: task
title: "Scheduler dispatch throughput — eliminate O(executions x tasks) per-tick DB round-trips that stall the loop under backlog"
short_code: "CLOACI-T-0745"
created_at: 2026-06-17T23:34:07.029193+00:00
updated_at: 2026-06-17T23:34:46.009701+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: NULL
---

# Scheduler dispatch throughput — eliminate O(executions x tasks) per-tick DB round-trips that stall the loop under backlog

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Make the scheduler's per-tick work scale sub-linearly with the active-workflow
backlog so dispatch throughput doesn't collapse under load. Found 2026-06-17
load-testing the demo (driver @500ms): at ~180–410 concurrent active workflows
the scheduler loop effectively stalled — `Task ready` events dropped to ~0, the
48-slot fleet sat idle, and the backlog wouldn't drain. The bottleneck is
upstream of the workers, in the scheduling loop itself.

## Root cause (code-cited)

`execution_planner/scheduler_loop.rs::process_active_executions` (:159) runs
every `scheduler_poll_interval` (100ms, `default_runner/config.rs:295`) and does
O(active_executions × pending_tasks) **sequentially-awaited** DB round-trips:

1. `process_executions_batch` (:207) batch-loads pending tasks in ONE query
   (`get_pending_tasks_batch`, good) — then a `for execution in &active_executions`
   loop (:232) runs, per execution, sequentially awaited:
   - `update_workflow_task_readiness(...)`
   - `check_workflow_completion(execution.id)` (`task_execution/queries.rs:219`) —
     a real query, run **unconditionally for all N** even with no state change.
2. `update_workflow_task_readiness` (`state_manager.rs:53`) loops the execution's
   pending tasks and per task awaits: `check_task_dependencies` — which
   **re-fetches the same `workflow_execution` via `get_by_id` on every task**
   (`state_manager.rs:96`) + dependency-state queries — then `evaluate_trigger_rules`,
   then a `mark_ready`/`mark_skipped` write.
3. `dispatch_ready_tasks` (:265) awaits `dispatcher.dispatch(event)` per ready
   task, serially.

At N≈180–410 that's thousands of serial round-trips per intended 100ms tick →
each tick takes seconds → the loop falls hopelessly behind → workers starve.
It is **algorithmic** (serial N×T query pattern + a redundant per-task workflow
re-fetch), not pool/lock contention. Restarting a backed-up server makes it worse
(the cron recovery/catchup service replays missed runs → thundering herd; observed
180 → 410).

## Technical Debt Impact
- **Current problems:** scheduling throughput collapses (<1 task/s) under a few
  hundred concurrent workflows; the fleet can't be utilized; deep backlogs don't
  self-drain and a restart amplifies them.
- **Benefits of fixing:** the fleet becomes the real concurrency knob; the server
  scales to meaningful workflow fan-out; the embedded runner benefits too.
- **Risk:** core scheduling hot path — a bug delays/duplicates/drops tasks.
  Mitigate with the existing scheduler integration tests + load-test on the demo.

## Technical Approach (ranked; ticket = all of these)
1. **Hoist the redundant per-task workflow fetch** — `check_task_dependencies`
   fetches the workflow_execution + workflow def once per task though they're
   constant across a call's pending tasks. Fetch once per execution, pass in.
2. **Skip idle executions** — only run `check_workflow_completion` for executions
   with NO pending tasks this tick (an execution with pending tasks is by
   definition not complete). Removes most completion queries.
3. **Batch dependency-state resolution** — load all task states for the active
   execution ids in one query per tick, build an in-memory map, resolve
   dependencies (and trigger rules where possible) without per-task queries.
   Batch the readiness writes (`mark_ready`/`mark_skipped`) where the DAL allows.
4. **Parallelize / bound** the residual per-execution work (`buffer_unordered`)
   and consider capping work-per-tick so a huge backlog can't monopolize a tick.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Per-tick scheduler DB round-trips are O(1)–O(distinct workflows), not
      O(active_executions × pending_tasks); no per-task `get_by_id` re-fetch.
- [ ] `check_workflow_completion` is not run for executions that still have
      pending tasks.
- [ ] Dependency/readiness evaluation resolves from batched state, not per-task
      queries; readiness writes batched where possible.
- [ ] No behavioral regression: dependency gating, trigger-rule skipping,
      completion, and retries still correct (existing scheduler integration tests
      pass; add coverage for the batched path).
- [ ] Load-test validation on the demo (driver @500ms): the fleet actually
      saturates / the backlog drains, `Task ready` keeps flowing, throughput no
      longer collapses at a few hundred active workflows.

### Type
- [x] Tech Debt — scheduler hot-path optimization
### Priority
- [x] P1 — gates real concurrency / fleet utilization under load

## Notes
- Standalone task → own branch off `main` + own PR (independent of #133 cron and
  #132 — touches `execution_planner/*` + `dal/unified/task_execution/`).
- Related: the restart thundering-herd (cron recovery/catchup) is a separate
  facet worth a follow-up if it persists after this lands.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*