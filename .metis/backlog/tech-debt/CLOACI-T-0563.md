---
id: audit-t4a-drop-confirmed-orphan
level: task
title: "Audit T4a: drop confirmed orphan code (zero callers, no public-API rationale)"
short_code: "CLOACI-T-0563"
created_at: 2026-05-04T20:19:12.054404+00:00
updated_at: 2026-05-04T22:30:20.475171+00:00
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

# Audit T4a: drop confirmed orphan code (zero callers, no public-API rationale)

Replaces the deletion-bucket of T-0558. Items here have **verified zero callers** AND **no plausible public-API or admin-tooling rationale** — they are dead islands left behind by past migrations.

## Objective

Delete each orphan and the `#[allow(dead_code)]` annotation that protects it.

## Backlog Item Details

### Type
- [x] Tech Debt — orphan removal.

### Priority
- [x] P3 — Low. Cumulative grep noise + landmine annotations.

### Technical Debt Impact
- **Current Problems**: Each `#[allow(dead_code)]` is a landmine that defeats the compiler's dead-code lint for surrounding real code. Future audit agents must re-prove these are dead.
- **Benefits of Fixing**: Compiler dead-code lint becomes trustworthy in these modules again.
- **Risk Assessment**: Low — every item is verified by grep across `crates/`, `examples/`, and `tests/`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Production code orphans

- [ ] `crates/cloacina/src/computation_graph/scheduler.rs` — `RunningGraph.manual_tx` field (~L251). Written but never read; restart at L928 mints a fresh channel. Delete the field; correct doc comment at L248.
- [ ] `crates/cloacina/src/dal/unified/workflow_executions.rs:687` — `WorkflowExecutionDAL::increment_recovery_attempts`. Orphaned by T-0502 RecoveryManager removal.
- [ ] `crates/cloacina/src/executor/thread_task_executor.rs:427-588` — `handle_task_result` + `mark_task_failed` pair (both `#[allow(dead_code)]`, only call each other).
- [ ] `crates/cloacina/src/executor/task_handle.rs` — `TaskHandle::new` (L121), `with_dal` (L132), `into_slot_token` (L304). Production uses `with_dal_and_cancel`.
- [ ] `crates/cloacina/src/execution_planner/scheduler_loop.rs:62` — `SchedulerLoop::new` is `#[allow(dead_code)]` with no callers. Audit whether `SchedulerLoop` itself is orphaned and delete the file if so.
- [ ] `crates/cloacina/src/registry/loader/package_loader.rs:543` — `temp_dir()` helper, zero callers.
- [ ] `crates/cloacina/src/registry/loader/task_registrar.rs:61` — `TaskRegistrar::new()`, zero callers.
- [ ] `crates/cloacina-server/src/routes/error.rs:47` — `ApiError::new`, file-local with no callers.
- [ ] `crates/cloacina-server/src/lib.rs:354` — `RequestId` extension is set on requests but no handler reads it. Drop the struct + middleware that sets it (or wire a consumer; if no consumer is planned, delete).

### Verify-then-delete

- [ ] `crates/cloacina/src/dispatcher/work_distributor.rs::PostgresDistributor` — only the file's own factory at L339 references it (and that factory errors with "use directly"). Re-grep before deleting; if confirmed orphan, delete the type and the factory branch.
- [ ] `crates/cloacina/src/executor/workflow_executor.rs:510` — `WorkflowStatus::from_str`. Used only by tests in the same file. Either gate behind `#[cfg(test)]` or delete with the tests.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` green.
- [ ] `angreal lint all` green (no new dead-code lints).

## Implementation Notes

### Technical Approach

One commit per logical group (DAL / executor / scheduler / server). Each commit verifies the test gates before moving to the next.

### Dependencies

None. T-0555 already shipped, so the upstream split it depends on is in place.

### Risk Considerations

- Pre-removal re-grep is required for `PostgresDistributor` and `WorkflowStatus::from_str` — these are the borderline items.
- If a `pub` symbol turns out to have an out-of-tree consumer, the user will surface that in PR review; revert to `pub(crate)` rather than `pub` deletion.

## Status Updates

### 2026-05-04 — Completed

All deletions landed; tests + lint verified.

**Deleted:**
- `RunningGraph.manual_tx` field + 2 write sites (scheduler.rs).
- `WorkflowExecutionDAL::increment_recovery_attempts` + sqlite/postgres helpers.
- `handle_task_result` + `mark_task_failed` dead pair (~85 LOC, thread_task_executor.rs).
- `TaskHandle::with_dal`, `TaskHandle::into_slot_token`. `TaskHandle::new` gated to `#[cfg(test)]`.
- `SchedulerLoop::new` (kept `with_dispatcher` — production constructor).
- `RequestId` struct + extension insert (kept middleware tracing/header work).
- Entire `crates/cloacina/src/dispatcher/work_distributor.rs` module — `WorkDistributor` trait, `PostgresDistributor`, `SqliteDistributor`, `create_work_distributor` all had zero callers workspace-wide. Originally added in commit b10719f (Feb 2026) as PG LISTEN/NOTIFY scaffolding for executors; the wire-up never happened, executors poll the outbox directly via `DefaultDispatcher`.

**Gated to `#[cfg(test)]`:**
- `WorkflowStatus::from_str` (workflow_executor.rs).
- `PackageLoader::temp_dir()` (package_loader.rs — used by tests in same file).

**Skipped (audit was wrong):**
- `TaskRegistrar::new()` — production caller at `workflow_registry/mod.rs:72`.
- `ApiError::new` — used by sibling constructors `bad_request`/`not_found`.

**Test gates:**
- `cargo check --workspace --all-features` green.
- `angreal test all` green (45 macros, 657 cloacina lib, 295 integration, 6 additional).
- `angreal lint all` failed but failures are 15 pre-existing clippy errors in `cloacina-macros` (parser.rs, reactor_attr.rs, graph_ir.rs) — verified via `git stash` that they fail on the unmodified branch. Not introduced by this ticket.

**Out-of-scope follow-up:**
- `failure_reason` in `thread_task_executor.rs` is now test-only after the dead-pair removal. Left as-is; gating to `#[cfg(test)]` is a follow-up if desired.
- The 15 clippy errors in `cloacina-macros` are a pre-existing P3 cleanup — fold into T-0562 or a new lint-cleanup ticket.
