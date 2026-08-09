---
id: cron-recovery-attempt-cap-is-in
level: task
title: "Cron recovery attempt cap is in-memory and has no CAS — restart resets it, replicas race"
short_code: "CLOACI-T-0926"
created_at: 2026-08-06T02:07:56.978498+00:00
updated_at: 2026-08-06T18:43:05.015076+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Cron recovery attempt cap is in-memory and has no CAS — restart resets it, replicas race

## Objective

Close the two residuals T-0914 recorded when it fixed cron duplicate-fires. The duplicate-fire-for-long-running-workflows bug is fixed and merged (#229); these are the remaining durability/concurrency gaps in the same recovery path.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. ATTEMPT CAP IS IN-MEMORY. CronRecoveryService.recovery_attempts is an Arc<Mutex<HashMap<..>>> (crates/cloacina/src/cron_recovery.rs:93), reset on every restart. A schedule that reliably fails recovery gets a fresh budget after each restart. The natural fix needs an attempts column on schedule_executions — T-0914 deliberately did not add a migration because the schema was owned by concurrent work at the time. NOTE: workflow_executions.recovery_attempts exists but is workflow-level, the wrong table.
2. NO CAS ON THE AUDIT ROW. Two runners' recovery services can both select the same unlinked lost handoff and both re-fire it — there is no claim/CAS on schedule_executions the way task claiming has claim_for_runner. Pre-existing, and now the last remaining duplicate-fire vector after #229 closed the long-running-workflow case.

## Context from the #229 fix (do not regress)

Recovery now consults the LINKED workflow execution first: linked + non-terminal = not lost (skip); linked + terminal = backfill completion, never re-fire; unreadable = warn + skip (fail toward no-duplicate). Only unlinked handoffs re-fire, and a successful re-fire completes the row. Any claim/CAS design must preserve those semantics.

## Acceptance Criteria

## Acceptance Criteria

- [x] Attempt cap persisted on schedule_executions (migration 049, both backends); survives restart — test builds a fresh service against the same DB and asserts no re-fire
- [x] CAS claim prevents two concurrent recovery services from both re-firing (test drives two services against one DB, exactly one fires); stale-claim takeover tested
- [x] #229 linked-execution semantics untouched; both its regression tests stay green (21 cron integration tests pass)

## Status Updates

- 2026-08-06: Filed from CLOACI-T-0914's recorded residuals (merged in #229). Both were explicitly deferred there: the cap because it needed a migration during concurrent schema work, the CAS because it is pre-existing and out of that ticket's scope.

- 2026-08-06 (impl, worktree `.claude/worktrees/t-0926`, branch `fix/t-0926-cron-recovery-durability`): current-state verification.
  - `CronRecoveryService.recovery_attempts: Arc<tokio::sync::Mutex<HashMap<UniversalUuid, usize>>>` confirmed at `crates/cloacina/src/cron_recovery.rs:99`; constructed fresh at `:121`; incremented at `:309-319`; cleared at `:254` and `:453`. Nothing durable — restart = fresh budget, as filed.
  - Recovery selection is `ScheduleExecutionDAL::find_lost_executions` (`crates/cloacina/src/dal/unified/schedule_execution/mod.rs:183`) — plain `completed_at IS NULL AND started_at < cutoff` SELECT, no claim column, no CAS. Confirmed: two services get identical row sets.
  - #229 semantics live at `cron_recovery.rs:217-276` (linked-first branch) and are untouched by this work.
  - Migration numbering: highest on main is 045 (`045_create_fleet_agents`, both backends). 046/047/048 reserved by in-flight branches → **using 049**.
  - IMPORTANT finding that drives the claim design: `WorkflowExecutor::execute` for `DefaultRunner`
    (`crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:56-119`) BLOCKS, polling until the
    workflow reaches a terminal state. A recovery re-fire is therefore NOT short — it is as long as the
    workflow. A fixed `claimed_at` staleness window would expire mid-execution and let a second recovery
    service re-fire → the exact duplicate this ticket closes. So the claim gets a HEARTBEAT (mirroring
    `task_executions.claimed_by` + `heartbeat_at` + `find_stale_claims`), not a bare timestamp window.

- 2026-08-06 (impl, cont.): design locked, implementing.
  - Migration **049** in both backends (`049_add_recovery_claim_to_schedule_executions`); highest on the
    worktree base (7aed099e) is 045 in both dirs. Columns:
    `recovery_attempts INTEGER NOT NULL DEFAULT 0`, `recovery_claimed_by` (UUID / BLOB),
    `recovery_heartbeat_at` (TIMESTAMP / TEXT), plus a partial index on open+claimed rows.
  - Counter lives ONLY on the row. The in-memory `HashMap` is deleted outright (not kept as a cache) —
    a cache would need invalidating by the other replica's writes and buys nothing: the increment is
    one UPDATE already inside the claimed critical section.
  - Claim CAS: `UPDATE ... WHERE id=? AND completed_at IS NULL AND (recovery_claimed_by IS NULL OR
    recovery_heartbeat_at < cutoff)`, `rows_updated == 1` wins — same shape as
    `task_execution::claiming::claim_for_runner`. `completed_at IS NULL` is part of the CAS so a loser
    holding a stale row snapshot cannot re-fire a handoff the winner already finished.
  - Expiry: heartbeat, not a fixed window (see previous update). New config fields
    `claim_heartbeat_interval` (30s) and `claim_stale_after` (120s). A crashed service's claim goes
    stale after 120s and the next sweep takes it over — nothing is permanently locked.

- 2026-08-06 (impl, DONE — uncommitted in the worktree). Both acceptance criteria met.
  - Files: `cron_recovery.rs`, `dal/unified/schedule_execution/mod.rs`, `dal/unified/{mod,models}.rs`,
    `database/schema.rs` (unified table! only — the backend-specific `schedule_executions` blocks are
    already stale/unused for this table), `models/schedule.rs`, `runner/default_runner/services.rs`,
    `tests/integration/scheduler/cron_recovery.rs`, plus migration 049 in both backends.
  - `recover_execution` now: linked-first (#229, untouched) → age check → **CAS claim** → re-read under
    the claim → **durable increment + cap** → schedule checks → heartbeat-guarded `execute` → link +
    complete + reset attempts. Every post-claim exit path releases the claim (split into
    `recover_claimed_execution` so the release is unconditional).
  - Public API change: `clear_recovery_attempts()` now takes an execution id and returns
    `Result` (there is no process-local cache left to wipe); `get_recovery_attempts` reads the row.
    No in-repo callers besides the service itself.
  - Tests: 3 new integration (cap survives restart / two services fire exactly once / stale claim is
    taken over) + 4 new sqlite DAL unit tests. `cargo test --features postgres,macros --test integration
    cron` → 21 passed, 0 failed (includes both #229 regression tests). sqlite lib DAL tests 17 passed.
    `cargo check --no-default-features --features postgres,sqlite` clean; `cargo fmt --all --check` clean.
  - Pre-existing unrelated failure seen once: `cron_basic::test_workflow_instance_register_roundtrip`
    duplicate-keys on `idx_schedules_instance_name` because it builds a `DefaultRunner` from the base
    URL (public schema) rather than the fixture's test schema, so rows leak between runs. Passes after
    `delete from public.schedules where instance_name is not null`. Worth its own ticket.

- 2026-08-06: MERGED to main in PR #242 (squash) — ticket complete. The design point worth carrying
  forward: this ticket's own brief suggested "a recovery pass is short, so a claim+release or a fixed
  staleness window is likely enough." That was WRONG and the implementation proved it — DefaultRunner's
  WorkflowExecutor::execute blocks until the workflow is terminal, so a re-fire lasts as long as the
  workflow, and a fixed window would have expired mid-execution and handed the row to a second service,
  recreating the exact duplicate this ticket exists to close. Heartbeat-guarded claim instead.
  Second subtlety: `completed_at IS NULL` had to be INSIDE the CAS predicate rather than a pre-check,
  or a loser holding a pre-completion row snapshot claims the finished row after the winner releases it.
  RESIDUALS (open): cron integration tests exercised against postgres only (sqlite claim/counter coverage
  is at the DAL unit level; the migration itself is proven to apply there); find_lost_executions still
  returns claimed rows and losers filter via the failed CAS (pushing the predicate into the SELECT would
  have meant touching #229's selection query); clear_recovery_attempts' signature change is
  semver-relevant for external consumers of the cloacina crate.
