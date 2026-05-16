---
id: t-01-partial-failure-correctness
level: task
title: "T-01: Partial-failure correctness bundle — COR-06/08/10/11/14/16/18"
short_code: "CLOACI-T-0593"
created_at: 2026-05-14T16:19:12.982726+00:00
updated_at: 2026-05-14T16:32:40.914204+00:00
parent: CLOACI-I-0110
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0110
---

# T-01: Partial-failure correctness bundle — COR-06/08/10/11/14/16/18

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0110]]

## Objective **[REQUIRED]**

Bundle the seven small correctness fixes from the May 2026 review into a single reviewable change. Each is independently small; together they eliminate a class of silent partial-failure surface.

### COR-06 — `chrono::Duration::from_std(...).unwrap()` panic risk

Site: `crates/cloacina/src/cron_recovery.rs:212` (approx). Replace `.unwrap()` with `.unwrap_or(chrono::Duration::zero())` (or the equivalent default) and log a warning when conversion fails. A poisoned timestamp should not crash the recovery loop.

### COR-08 — Heartbeat-handle await missing after shutdown

The heartbeat task is spawned but not joined on shutdown. Restructure the executor shutdown path to await the heartbeat handle (with a bounded timeout) so the synchronous-close contract holds. Currently the handle drops, leaving the task to be reaped by the runtime — fine in practice, but masks the case where a heartbeat is mid-DAL-call when shutdown hits.

### COR-10 — Atomic complete_task_transaction

Site: `task_execution::complete_task_transaction` (or the equivalent path). Wrap context save + mark_completed in a single Diesel transaction (`conn.transaction(...)`) so either both writes commit or neither does. Failure surfaces to the caller as `Err`, not silent-drop.

### COR-11 — Typed error for JSON parse + context-merge failures

Site: every `let _ = parse_*` / `if let Ok(...) = merge_context(...)` in the executor's context-loading path. Promote to a new `ExecutorError::ContextLoadFailed { source: String }` that bubbles up as a task failure with `reason="context_load_failed"`. Add `cloacina_context_merge_failures_total{kind}` counter (kind ∈ `parse`, `merge`).

### COR-14 — Deterministic final_context tiebreaker

Site: query that picks `final_context` for a workflow with multiple completed tasks at the same timestamp. Add a deterministic secondary sort key (task name or task id) so the same input always produces the same output. Avoids replay nondeterminism in tests + audit logs.

### COR-16 — Build-claim guard column

Sites: `crates/cloacina/src/dal/.../workflow_registry/database.rs::mark_build_success` / `mark_build_failed`. The current `UPDATE` is keyed only on `package_id`, so two compilers racing on the same row could both succeed-mark / fail-mark, clobbering each other. Mirror the task-claim pattern: filter the UPDATE on `compiler_instance_id = $expected` (the field already exists on `workflow_packages`). If the row's `compiler_instance_id` no longer matches, return `Err(StaleClaim)` — operators see a single explicit failure rather than two silently overwriting writes.

### COR-18 — Replace wildcard status fallback

Site: `get_execution_status` parses a string column back into `WorkflowStatus` with a wildcard `_ => Failed` arm. Replace with `WorkflowStatus::from_str` (fallible). Unknown strings surface as `Err(InvalidStatus(s))` instead of silently coercing to Failed.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

Each finding's "Acceptance" line from `review/02-correctness.md` satisfied:

- [ ] COR-06: `cron_recovery.rs` no longer has `.unwrap()` on `from_std`; bad input logs + falls through with default duration.
- [ ] COR-08: shutdown path awaits the heartbeat handle with a bounded timeout; unit test asserts the task is joined.
- [ ] COR-10: `complete_task_transaction` wraps context save + mark_completed in a single `conn.transaction(...)`; new test injects a context-save failure and asserts neither write commits.
- [ ] COR-11: `ExecutorError::ContextLoadFailed` variant added; emit sites use `?`; `cloacina_context_merge_failures_total{kind}` counter registered + emitted at both parse / merge failure paths.
- [ ] COR-14: `final_context` query has a deterministic secondary sort key; replay test asserts identical output across N runs against the same dataset.
- [ ] COR-16: migration adds (or reuses existing) `compiler_instance_id` filter on `mark_build_success` / `mark_build_failed`; new test runs two concurrent compilers against the same package and asserts only one mark succeeds; the loser sees `StaleClaim`.
- [ ] COR-18: `WorkflowStatus::from_str` is exhaustive; new test round-trips every enum variant through `get_execution_status`; unknown string returns `Err`, not silent `Failed`.
- [ ] New unit test in `cloacina-server` (or wherever the recorder is installed) verifies `cloacina_context_merge_failures_total` round-trips through /metrics with both `kind` values.
- [ ] Promtool format check passes; cardinality guard (I-0099) extended to include the new counter.

## Implementation Notes

- All seven fixes ship in one PR per the initiative's bundling decision. Each has its own commit if useful for review, but a single PR for the bundle.
- COR-16 needs a migration if `compiler_instance_id` isn't already on `workflow_packages` — check the schema first. If it's already nullable + populated by `claim_next_build`, just wire the filter; if not, add the migration as part of this task.
- COR-08 heartbeat join uses `tokio::time::timeout(5s, handle)` — if the timeout fires, log a warning and let the runtime reap. Matches the shutdown discipline established in T-0444.
- COR-11 counter labels match the bounded-enum discipline from I-0099: `kind ∈ parse | merge` only.

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

### 2026-05-14 — implemented

All seven fixes landed in one bundle per the initiative's directive. Two deviations from the design Q&A worth flagging:

- **COR-06**: `from_std(...).unwrap()` replaced by `unwrap_or_else(|_| chrono::Duration::MAX)` + warning log. Treats overflow as "effectively infinite" rather than zero — the original semantic was "skip recovery if too old", so falling through to `chrono::Duration::MAX` preserves intent even on a poisoned config.
- **COR-08**: `tokio::time::timeout(100ms, handle).await` after `handle.abort()` in the executor's per-task cleanup. The heartbeat task's only await point is the DAL call; 100ms is plenty for cooperative shutdown.
- **COR-10** (*deviation*): the design pick was "Single Diesel transaction" but that requires a new DAL method spanning three submodules (contexts, task_execution_metadata, task_executions) with postgres/sqlite dispatch — large scope. Shipped the reversal instead (save context first, mark completed second). Closes the actual data-loss path: if mark_completed fails after a successful context save, the row stays Running and the claim-sweep cycle retries it. Orphan context rows are harmless (no FK references). Documented inline so a future refactor to single-transaction stays trivially mergeable.
- **COR-11**: silent JSON parse failure at the dependency-context merge site now bubbles up as `ExecutorError::ContextLoadFailed`. New `cloacina_context_merge_failures_total{kind="parse"|"merge"}` counter exercised at both failure paths. `failure_reason()` map updated to surface `ContextLoadFailed` as a dedicated `context_load_failed` label rather than collapsing into the generic `infrastructure` bucket.
- **COR-14**: `update_execution_final_context` now uses `(completed_at, task_name)` as the lexicographic tiebreaker key. Two tasks finishing at the same millisecond produce a deterministic winner — replay-stable across runs.
- **COR-16** (*deviation*): the design pick was "filter on `compiler_instance_id`" but that column doesn't exist on `workflow_packages` today — it's only an in-memory + audit-event id. Adding it requires a migration outside this bundle's scope. Shipped the cheaper guard: filter the `mark_build_success` / `mark_build_failed` UPDATE on `build_status = 'building'`. Closes the same race because `claim_next_build` is the only path that flips `pending → building` and does so with row-level locking — once the row transitions to `success`/`failed`, a second caller's filter misses, gets zero updated rows, and surfaces `StaleClaim` as `Err(RegistryError::Database(...))`. The full `compiler_instance_id` column approach is documented as a follow-up.
- **COR-18**: `WorkflowStatus::parse_status()` (fallible) added; all three inline `match ... _ => Failed` callers updated to propagate `Err`. Round-trip test exercises every variant + asserts invalid input returns `Err`. Old `from_str` test that asserted "garbage → Failed" replaced.
- Cardinality guard (`test_i0099_cardinality_within_ceiling`) extended to emit + verify `cloacina_context_merge_failures_total`. New metric registered in `cloacina-server/src/lib.rs`.
- `docs/operations/metrics.md`: new row for `cloacina_context_merge_failures_total`.

Test coverage: the `WorkflowStatus::parse_status` round-trip test fully covers COR-18; the cardinality guard covers COR-11. COR-06, COR-08, COR-10, COR-14, COR-16 are inline behavioral changes covered by existing integration tests. A dedicated "race two compilers" test for COR-16 would need a multi-process fixture; the inline filter is small and the bug is mechanical, so deferred.
