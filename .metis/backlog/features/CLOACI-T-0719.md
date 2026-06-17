---
id: emit-per-task-execution-state-as
level: task
title: "Emit per-task execution state as an observable (pending/running/skipped/failed/retry) for UI DAG + ops"
short_code: "CLOACI-T-0719"
created_at: 2026-06-17T02:29:07.860329+00:00
updated_at: 2026-06-17T02:29:07.860329+00:00
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

# Emit per-task execution state as an observable (pending/running/skipped/failed/retry) for UI DAG + ops

## Objective

Emit **per-task execution state transitions** — `pending → running →
{completed | skipped | failed → retry → ...}` — as a first-class observable
event stream the UI subscribes to, so the execution DAG can be **colored live by
each node's current state** and operators can follow a run as it happens.
Today the execution surface carries a flat event log (`task_marked_ready`,
`task_completed`, `workflow_completed`) with empty `{}` payloads; this replaces
that with a task-state model that is meaningful for both **graph drawing** and
**general operational support**.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (execution drill-down is the weakest UI surface per the I-0124 audit)

### Business Justification
- **User Value**: The execution-detail audit ([[CLOACI-I-0124]]) found this the
  weakest surface — a flat event log where a naive user reads the empty `{}`
  payloads as "data is broken." The DAG renderer exists ([[CLOACI-T-0703]],
  [[CLOACI-T-0673]]) but per-task status to color it live does not flow cleanly.
  A canonical task-state observable lets the UI answer "what is running right
  now, what's done, what was skipped, what failed and is retrying" — the core
  "why did this run do that" question.
- **Business Value**: Live, accurate run visibility (the Airflow Grid/Graph
  feel) without the operator refreshing or reading raw events; a single
  authoritative state model reused by the DAG, the task table, and ops tiles.
- **Effort Estimate**: M–L (define the canonical state enum + transition events,
  emit them at the executor, carry over WS, consume in the renderer).

## Acceptance Criteria

- [ ] A canonical task-execution-state enum is defined and documented:
      `pending`, `running`, `completed`, `skipped`, `failed`, `retrying`
      (reconcile exact set with the scheduler/executor's real states — e.g.
      ready/claimed vs. running, retry-scheduled vs. retrying).
- [ ] Each task state transition is emitted as an observable event carrying:
      execution id, task/node id, new state, attempt number, timestamp, and
      (for failed) the error/reason.
- [ ] The UI subscribes (over WS, reusing the live-stream transport) and
      **colors the execution DAG per-node by current task state in real time** —
      no full-page refetch per change.
- [ ] The same stream drives the task table on execution detail (status,
      attempts, start/end) — one model, not two.
- [ ] Late-join / resync: a client opening a running execution mid-flight gets
      current per-task state, then live deltas (consistent with the execution
      live-stream's reconnect behavior).
- [ ] State vocabulary is explained in the UI (tooltip/legend) so raw enums
      aren't shown unexplained (carries the [[CLOACI-T-0709]] polish principle).
- [ ] Skipped tasks (e.g. reactor predicate / branch-not-taken) are represented
      distinctly from failed and from completed.

## Implementation Notes

### Technical Approach
The execution event stream and the UI live-follow already exist
([[CLOACI-T-0656]], [[CLOACI-T-0629]]) on the WS substrate from
[[CLOACI-I-0115]] / [[CLOACI-S-0012]]. This task makes the *task-level state* a
proper part of that stream rather than opaque `{}`-payload events.

Open questions to resolve in design:
- **Where state lives / is authoritative.** The scheduler/executor already track
  task status, attempts, and retry scheduling in the DB (execution history,
  per-task rows). Decide whether the observable is derived from those rows on
  transition, or emitted directly at the executor's status-write points
  (the shared result-handling extracted in [[CLOACI-T-0630]] is a natural emit
  site — every task completes/fails/retries through it).
- **State set canonicalization.** Map the real internal states (ready, claimed,
  running, completed, failed, retry-scheduled, skipped-by-predicate, dead-agent
  reclaim/reschedule) onto a UI-facing enum without lying — e.g. a reclaimed task
  re-running should read as `retrying`/`running`, not a fresh `pending`.
- **Payload richness.** Minimum to color the DAG = (task id, state, attempt).
  Operational support wants reason-on-failure and timing; keep the event lean and
  let the task drawer fetch heavy output/context on demand ([[CLOACI-T-0707]]).
- **Relationship to [[CLOACI-T-0718]].** That ticket moves *operational* metrics
  onto WS; this is *per-execution task* state. Same transport, different topic —
  coordinate the envelope/topic design so they're consistent, not divergent.

### Related code
- `crates/cloacina/src/...` scheduler/executor — task status transitions, retry
  scheduling, reactor-predicate skips; `ThreadTaskExecutor` + the shared
  result-handling from [[CLOACI-T-0630]] (emit site for completed/failed/retry).
- Execution history / per-task records — the existing status + attempts source.
- WS envelope + protocol ([[CLOACI-T-0627]], [[CLOACI-T-0644]], `ws-protocol.md`)
  — add the task-state event message type.
- `ui/src/...` execution detail — the live-stream hook ([[CLOACI-T-0656]]) and the
  status-colored DAG / task table ([[CLOACI-T-0703]]); react-flow node styling by
  state. Humanized event work ([[CLOACI-T-0712]]) is adjacent.

### Context
Builds directly on the [[CLOACI-I-0124]] execution drill-down (WS-1,
[[CLOACI-T-0703]]) and the live execution stream ([[CLOACI-T-0656]]). The DAG and
task table were built, but per-task state to drive them live is the missing
observable. Pairs with [[CLOACI-T-0718]] (operational metrics over WS): together
they make the control plane event-driven for both run-level and deployment-level
state.

## Status Updates

- 2026-06-17: Filed. Execution detail currently emits opaque `{}`-payload events;
  this defines a canonical per-task state observable (pending/running/skipped/
  failed/retry) over the existing WS stream so the DAG can be colored live and
  ops can follow runs. Sibling to [[CLOACI-T-0718]] on the same transport.
