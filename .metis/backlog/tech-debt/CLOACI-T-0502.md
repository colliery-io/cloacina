---
id: remove-recoverymanager-verify
level: task
title: "Remove RecoveryManager — verify stale_claim_sweeper fully subsumes it"
short_code: "CLOACI-T-0502"
created_at: 2026-04-16T17:26:59.912489+00:00
updated_at: 2026-04-16T17:26:59.912489+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

*To be added during implementation*
