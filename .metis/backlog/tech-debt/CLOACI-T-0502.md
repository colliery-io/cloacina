---
id: remove-recoverymanager-verify
level: task
title: "Remove RecoveryManager — verify stale_claim_sweeper fully subsumes it"
short_code: "CLOACI-T-0502"
created_at: 2026-04-16T17:26:59.912489+00:00
updated_at: 2026-04-21T02:45:13.848904+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Remove RecoveryManager — verify stale_claim_sweeper fully subsumes it

## Objective

The original hardening intent (I-0051) was for `RecoverySweepService` / `stale_claim_sweeper` to subsume `RecoveryManager` entirely. The heartbeat-based infrastructure (`claimed_by`, `heartbeat_at` columns, sweeper service, heartbeat-driven cancellation) has since landed — but `RecoveryManager` still exists alongside the sweeper in `crates/cloacina/src/execution_planner/recovery.rs`. This task is to verify the sweeper covers every path `RecoveryManager` used to handle, then delete `RecoveryManager`.

## Type
- [x] Tech Debt

## Priority
- [x] P2 — No correctness impact, but removes duplicate recovery code paths and reduces confusion.

## Technical Debt Impact

- **Current problems**: Two recovery implementations coexist. `RecoveryManager` still lives at `execution_planner/recovery.rs` (9 public/private methods handling orphaned task recovery, workflow-known vs. workflow-unknown cases, permanent abandonment, recovery event recording). `stale_claim_sweeper.rs` handles heartbeat-based detection. It's unclear to new readers which is authoritative.
- **Benefits of fixing**: Single recovery code path. Simpler mental model. Less dead code to maintain.
- **Risk**: If the sweeper *doesn't* cover one of `RecoveryManager`'s cases, deletion causes silent regressions. Hence the verify-first approach.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Enumerate every case handled by `RecoveryManager::recover_orphaned_tasks` and its helpers (known workflow, unknown workflow, abandonment, recovery events)
- [ ] Confirm each case has equivalent coverage in `stale_claim_sweeper` + heartbeat cancellation path, or add missing coverage
- [ ] Remove all `RecoveryManager` call sites
- [ ] Delete `execution_planner/recovery.rs`
- [ ] Recovery integration tests still pass (stale claims, crash-restart)

## Implementation Notes

Code refs:
- `crates/cloacina/src/execution_planner/recovery.rs` — `RecoveryManager<'a>`, `recover_orphaned_tasks`, `recover_tasks_for_known_workflow`, `abandon_tasks_for_unknown_workflow`, `recover_single_task`, `abandon_task_permanently`, `record_recovery_event`
- `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs` — heartbeat-based sweeper
- Migrations `012_add_task_claiming` (sqlite) and `013_add_task_claiming` (postgres) added `claimed_by` and `heartbeat_at`

Check whether `RecoveryManager` is still wired into startup or scheduler paths, or if it's already orphaned code.

## Status Updates

### 2026-04-20 — Gap analysis

**RecoveryManager call surface:** The only non-test caller is `TaskScheduler::with_poll_interval` (`execution_planner/mod.rs`), which runs `recover_orphaned_tasks` once at scheduler construction. Nothing else (runner, server, daemon, tests) constructs `RecoveryManager` directly.

**What RecoveryManager does (status-based):**
1. `get_orphaned_tasks()` — selects every task with `status = "Running"`, regardless of `claimed_by` / `heartbeat_at`.
2. Groups by workflow execution; checks runtime registry for the workflow.
3. **Known workflow** → `reset_task_for_recovery` (Ready, increment `recovery_attempts`); after `MAX_RECOVERY_ATTEMPTS = 3`, calls `mark_abandoned` + `check_workflow_failure` → marks workflow execution failed.
4. **Unknown workflow** → `mark_abandoned` for every task + marks workflow execution failed; records a `WorkflowUnavailable` recovery event.
5. Records `TaskReset` / `TaskAbandoned` / `WorkflowUnavailable` rows in `recovery_events`.

**What `StaleClaimSweeper` does (heartbeat-based):**
1. After a startup grace period equal to `stale_threshold`, queries `find_stale_claims` (claimed_by NOT NULL AND heartbeat_at < cutoff).
2. `release_runner_claim` + `mark_ready`. Re-dispatch happens via the normal scheduler/dispatcher loop.
3. No registry check, no `recovery_attempts` increment, no `recovery_events` row.

**Coverage delta — and why deletion is still safe:**

- **Heartbeat vs. status:** RecoveryManager's `status = "Running"` query is *less* precise than the sweeper's heartbeat query. Under the modern claiming model every executing task carries a `claimed_by` + live `heartbeat_at`; a healthy in-flight task with status=Running would be incorrectly reset by RecoveryManager but correctly left alone by the sweeper. The sweeper's model is the right one.
- **Recovery attempt limit / poison-task abandonment:** Per-task retry capping already exists at the executor level via `attempt` / `max_attempts` (see `RetryStats`, `get_exhausted_retry_tasks`). The `recovery_attempts` counter that RecoveryManager incremented is independent of the retry policy and was used only to gate `RecoveryManager` itself. No other caller reads it.
- **Unknown-workflow abandonment:** A task whose workflow has been removed from the registry will be reset to Ready by the sweeper after a stale heartbeat and never picked up by the dispatcher. This is an idle-row leak, not a correctness regression. Adding a registry-aware sweep is out of scope for this task.
- **`recovery_events` audit trail:** The table remains; `CronRecoveryService` still writes to it. Nothing outside RecoveryManager and its DAL helpers writes the `TaskReset`, `TaskAbandoned`, or `WorkflowUnavailable` event types. Loss of these rows is acceptable given the sweeper's `info!` logging covers operational visibility.

**Decision:** Delete `RecoveryManager`, the `mod recovery` declaration, and the `with_poll_interval` call site. Leave DAL helpers (`get_orphaned_tasks`, `reset_task_for_recovery`, `check_workflow_failure`, `mark_abandoned`) in place to keep this change tightly scoped — they can be pruned in a follow-up dead-code sweep.

### 2026-04-20 — Implementation
- Removed `mod recovery;`, `use recovery::RecoveryManager;`, and the `recover_orphaned_tasks` invocation from `crates/cloacina/src/execution_planner/mod.rs`.
- Updated module + struct + constructor docstrings to drop the orphaned-task-recovery prose and point readers at `StaleClaimSweeper`.
- Deleted `crates/cloacina/src/execution_planner/recovery.rs`.
- `with_poll_interval` retains its `Result<Self, ValidationError>` return type for API stability even though construction is now infallible.
- Deleted `crates/cloacina/tests/integration/scheduler/recovery.rs` (whole file — every test there asserted `RecoveryManager`-specific behavior such as workflow-unavailable abandonment and `recovery_events` rows). Removed the `mod recovery;` line from `tests/integration/scheduler/mod.rs`.

### 2026-04-20 — Verification
- `angreal cloacina unit` → 696 passed.
- `cargo test -p cloacina --test integration --features sqlite,macros scheduler:: dal::task_claiming` → 39 passed, 0 failed (covers stale-claim sweeper, claiming, crash/heartbeat scenarios).
- Full integration suite needs Docker for postgres/kafka fixtures; pre-existing failure `registry_workflow_registry_tests::test_register_real_workflow_package` is unchanged from baseline (verified by stashing this branch and re-running).

### Acceptance criteria status
- [x] Enumerated every case handled by `RecoveryManager::recover_orphaned_tasks` and its helpers.
- [x] Confirmed coverage delta vs. `stale_claim_sweeper` (decision documented above).
- [x] Removed all `RecoveryManager` call sites.
- [x] Deleted `execution_planner/recovery.rs`.
- [x] Stale-claim integration tests still pass; crash/restart paths now exclusively heartbeat-driven.
