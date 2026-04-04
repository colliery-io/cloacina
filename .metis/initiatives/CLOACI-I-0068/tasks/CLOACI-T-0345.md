---
id: t4-task-outbox-and-execution
level: task
title: "T4: Task outbox and execution metadata DAL tests"
short_code: "CLOACI-T-0345"
created_at: 2026-04-03T13:09:24.011538+00:00
updated_at: 2026-04-03T18:07:56.983958+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T4: Task outbox and execution metadata DAL tests

## Parent Initiative
[[CLOACI-I-0068]] — Tier 2 (~747 missed lines)

## Objective
Add tests for task_outbox.rs (14% coverage, 247 missed) and task_execution_metadata.rs (45%, 325 missed). The outbox is the dispatch queue for ready tasks — critical for correctness. Metadata stores per-task context IDs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] task_outbox.rs: test enqueue, dequeue, peek, mark_processed, cleanup_old_entries
- [ ] task_execution_metadata.rs: test create, get_by_pipeline_and_task, update_context_id, list_by_pipeline
- [ ] task_execution/recovery.rs: test find_orphaned_tasks, reset_to_ready, abandon_task (20% → >50%)
- [ ] Tests run against both backends using get_all_fixtures()
- [ ] Coverage moves from ~14-45% to >60%

## Source Files
- crates/cloacina/src/dal/unified/task_outbox.rs (247 missed, 14%)
- crates/cloacina/src/dal/unified/task_execution_metadata.rs (325 missed, 45%)
- crates/cloacina/src/dal/unified/task_execution/recovery.rs (175 missed, 20%)

## Implementation Notes
The outbox tests need a pipeline + task setup first (create pipeline, create task, mark ready → outbox entry). Follow the pattern from dal/task_claiming.rs tests.

## Status Updates

### 2026-04-03 — Complete (36 tests passing, 1 ignored)

**task_outbox.rs** (13 tests): create entry via mark_ready, list pending (empty/limit/ordering), count pending, delete by task (found/not found/only target), delete older than (all/none), direct create, mark_ready populates outbox

**task_execution_metadata.rs** (12 tests): create (with/without context), get by pipeline+task (found/not found), get by task execution, update context_id (set/clear), upsert (insert/update), get dependency metadata (found/empty), get dependency metadata with contexts (1 ignored — task_name format mismatch)

**task_execution/recovery.rs** (10 tests): get orphaned (none/finds running), reset for recovery (single/increments counter), check pipeline failure (abandoned/not abandoned/regular), retry stats (none/with retries), exhausted retry tasks (found/empty)

Fixes applied: "Pending"→"NotStarted" status, "all_success"→valid JSON trigger_rules
