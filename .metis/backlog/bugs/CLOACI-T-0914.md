---
id: execution-core-crash-edge-defects
level: task
title: "Execution-core crash-edge defects — unsweepable claim window, dead Abandoned machinery, fail-open claim errors"
short_code: "CLOACI-T-0914"
created_at: 2026-08-02T16:33:13.496800+00:00
updated_at: 2026-08-02T18:36:21.576816+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Execution-core crash-edge defects — unsweepable claim window, dead Abandoned machinery, fail-open claim errors

## Objective

Close the crash-edge recovery gaps in the execution core found by the 2026-08-02 architecture deep dive (verdict: A- hot path, C+ crash-edge perimeter). The hot path is I-0140-hardened; these are the perimeter holes.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Findings (each independently fixable)

1. UNSWEEPABLE STUCK-TASK WINDOW. A crash between claim_for_runner (status still Ready, claimed_by set) and mark_started produces a row NO mechanism recovers: find_stale_claims filters status='Running' (crates/cloacina/src/dal/unified/task_execution/claiming.rs:479) while re-dispatch requires claimed_by IS NULL (claiming.rs:516). The window spans semaphore wait, so it is real under load; check_workflow_completion never passes and the caller's execute() burns the full 1h workflow_timeout. Fix: sweep expired-heartbeat claims regardless of Ready/Running status (likely one filter change) + a test crashing in the window.
2. DEAD RECOVERY MACHINERY / NO RECYCLE CAP. The documented Abandoned terminal state and its machinery (mark_abandoned, get_orphaned_tasks, reset_task_for_recovery, recovery_attempts counter) have zero production callers — the StaleClaimSweeper replaced them and re-Readies WITHOUT an attempt cap, so a runner-killing task recycles forever. Fix: either wire an abandonment cap into the sweeper (use recovery_attempts) or delete the dead machinery and document the recycle-forever semantics deliberately.
3. FAIL-OPEN CLAIM-WRITE ERROR. When the claim WRITE itself errors, the executor proceeds to execute unguarded (crates/cloacina/src/executor/thread_task_executor.rs:424-431) — an inversion of the I-0140 loud-failure doctrine and a double-execution vector under claiming. Fix: fail closed (skip + retry_transient like terminal writes).
4. WRITE-ONLY task_outbox. claim_ready_task (the pull-based outbox consumer with its FOR UPDATE SKIP LOCKED / BEGIN IMMEDIATE twins) is production-dead; task_outbox grows unboundedly with no sweeper. Fix: delete the consumer + table, or add pruning; do not leave an unbounded tracked table.
5. CRON DUPLICATE-FIRE for workflows running >10 min: the cron recovery service (10-min lost threshold) can re-hand-off a schedule whose workflow is still legitimately running. Fix: recovery check should consult the live execution row, not just the claim age.

## Acceptance Criteria

- [ ] Crash-in-claim-window test passes: task is re-dispatched/reclaimed within one sweep interval, never stuck
- [ ] Re-Ready recycling is capped (or the cap is explicitly documented as a non-goal and the dead Abandoned code is removed)
- [ ] Claim-write error path fails closed with retries
- [ ] task_outbox has an owner: consumed or pruned or removed
- [ ] Long-workflow cron duplicate-fire covered by a test

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (execution-core report; DEEPDIVE.md risk register R-high entries). Findings verified against main @ 5216e632.
- 2026-08-02: ACTIVE on fix/t-0914-execution-crash-edges. Verified plan: (a) heartbeat starts AT CLAIM (claim_for_runner sets heartbeat_at; heartbeat loop spawns before semaphore acquire) so stale heartbeat on Ready+claimed is a true death signal — broadening find_stale_claims is safe, BUT must be status IN (Running, Ready), never claim-only: a crash after mark_completed leaves a claim on a terminal row that must not be re-Readied. (b) Terminal set is exactly {Completed, Failed, Skipped} (queries.rs:85, state_manager.rs:135) — a literal Abandoned status would hang workflows; HOWEVER mark_abandoned already writes status=Failed + ABANDONED: error prefix + TaskAbandoned event transactionally (state.rs:421-469), and check_workflow_failure already queries that convention — the machinery was designed for this cap and never wired. PLAN: sweeper gains max_recovery_attempts (default 3, mirroring cron); StaleClaim carries recovery_attempts; per claim: release + (attempts >= cap ? mark_abandoned : reset_task_for_recovery — which Ready-resets AND increments, replacing mark_ready). Delete get_orphaned_tasks (misleading, zero callers). Sweeper writes stay warn+continue (30s re-sweep = natural retry). (c) Claim Err at thread_task_executor.rs:424-430 -> return skipped (stays Ready+unclaimed, re-selected next tick) + metric outcome=error. (d) task_outbox: verify no non-test readers, stop writing + prune (no schema migration). (e) cron: link audit row at handoff. Implementing a,b,c then d,e.
