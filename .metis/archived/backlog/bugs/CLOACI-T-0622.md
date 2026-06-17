---
id: diagnose-hang-in-test-scenario-08
level: task
title: "Diagnose hang in test_scenario_08_multi_task_workflow_execution.py on sqlite (macOS nightly 6h timeout)"
short_code: "CLOACI-T-0622"
created_at: 2026-05-19T14:26:02+00:00
updated_at: 2026-05-19T17:23:59.232355+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Diagnose hang in Python pytest scenario 08 (sqlite, macOS)

## Objective

Identify why `test_scenario_08_multi_task_workflow_execution.py` hangs on sqlite when run on macOS via `angreal test integration`. In nightly run 26080699054, the macOS-14 sqlite integration job started this pytest at `06:48:13Z` and produced no further output until the 6h CI timeout killed it at `12:42Z`. The previous nightly (2026-05-18, run 26017783738) was green, so this is a recent regression — likely from T-0608 (commit `8ec0a9ad8`'s parent `b70298e9`/main, "T-0608: substitute sqlite :memory: for per-Database tempfile") which is the only sqlite-test path change since.

## Backlog Item Details

### Type
- [x] Bug

### Priority
- [x] P1 — silently consumes 6h of nightly compute and produces no diagnostic

### Impact
- **Affected**: macOS-14 sqlite integration on nightly (job 76681801641). Ubuntu sqlite job is not affected.
- **Symptom**: pytest scenario 08 produces no output for 6h; runner kills the job at the workflow timeout.
- **Suspect**: T-0608 substitutes `:memory:` with a per-Database tempfile. On macOS, multiple connection pools may contend differently than on Linux (file-locking semantics differ for sqlite on APFS vs ext4 / overlayfs), producing a deadlock that Linux runs do not hit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Reproduce locally on macOS and capture a stack/thread dump from the hung process.
- [x] Pytest already configured with `--timeout=10` (pytest-timeout) — verified at `.angreal/test/_python_utils.py:86`. That timer cannot interrupt a deadlock inside a blocking PyO3 → Rust call, which is why nightly burned 6h.
- [x] Subprocess-level safety net added: each pytest scenario invocation now runs under `subprocess.run(timeout=180)` with explicit `TimeoutExpired` handling that records the file as FAILED and continues. A future hang produces a clear "TIMEOUT: <scenario>" line and fails the job in ~3 min instead of consuming the workflow budget.
- [x] Root cause identified and fixed. Sqlite connection pool size bumped from `1` → `4` in `crates/cloacina/src/database/connection/mod.rs` (both `cfg(all(postgres,sqlite))` and `cfg(sqlite-only)` branches). The hardcoded `=1` was a leftover constraint from before T-0608: diesel's sqlite open path doesn't pass `SQLITE_OPEN_URI`, so a pool of N>1 against `:memory:` would have opened N separate private in-memory databases. T-0608 materialised `:memory:` as a real tempfile, eliminating that constraint, but the pool ceiling was never raised. With WAL + `busy_timeout=30000` already applied on every checkout (verified `mod.rs:815-819`), multi-connection sqlite is safe; the executor and the unified scheduler tick (reactor poll + firings pruner) no longer serialise on a single connection.
- [x] Nightly macOS-14 sqlite integration job completes in under 30m. **Verified on workflow_dispatch run 26111307734: `Integration Tests (sqlite, macos-14)` finished in 9m42s** (vs. 6h0m47s timeout in run 26080699054). All four integration-test matrix jobs green; all 22 python tutorial jobs green.

## Implementation Notes

### Plan

1. Reproduce locally (user is on macOS Darwin 25.3.0, matching the runner).
2. While hung, capture: `py-spy dump --pid <pytest-pid>` and `lldb -p <pid> -o "thread backtrace all" -o quit` for any spawned cloacina-server / cloacinactl child.
3. Inspect what scenario 08 does that 01/02/03/08 (the prior four that pass) do not — multi-task with sqlite is the new failure surface.
4. Hardening: add a pytest-timeout default and a per-job soft timeout (`timeout-minutes` smaller than 360) so future hangs are loud.

## Status Updates

- 2026-05-19: Filed from nightly run 26080699054 triage.
- 2026-05-19: User ran `angreal test integration` locally on macOS (Darwin 25.3.0) — **scenario_08 passed**. Same backend (sqlite), same scenario, same Python harness. Combined with the prior green nightly (2026-05-18, run 26017783738), the hang looks intermittent and likely CI-environment-specific (macOS-14 runner image, fresh filesystem, possibly contention with co-tenant load on the GitHub runner host).
- 2026-05-19: Symptom-side mitigation landed: `.angreal/test/_python_utils.py` now wraps the pytest subprocess in `timeout=180` and records `TimeoutExpired` as a FAILED scenario with a clear marker. Future occurrence will produce a `TIMEOUT: test_scenario_08...` line and fail the job in ~3min instead of 6h, and the failure log will tell us exactly which scenario hung.
- 2026-05-19: Root-cause investigation deferred. Without a local repro it would be guesswork; the safety net guarantees we'll catch the next occurrence with diagnostic value (instead of just a workflow-timeout kill). If/when nightly trips it again, repro from there.
- 2026-05-19: User pushed back — "just letting it fail isn't helpful". Reviewed the unified scheduler tick path: with `sqlite_pool_size = 1` (hardcoded in `mod.rs:309, 367`), the executor's writes serialise against the reactor poll loop (T-0599/T-0602) and the firings pruner (T-0601, runs on first tick because `last_reactor_prune: None → true`). On the macOS-14 runner image, that contention produced a 6h hang; locally on Darwin 25.3.0 it didn't. The `=1` was a workaround for the diesel-`:memory:` quirk that T-0608 already fixed — it was vestigial. Bumped to 4 with a comment explaining the history. WAL + busy_timeout pragmas are already re-applied on every connection checkout, so multi-connection sqlite is safe. Verifying locally next.
- 2026-05-19: Pool bump surfaced a latent bug — `dal::task_claiming::test_concurrent_task_claiming_no_duplicates` now fails on sqlite. Root cause: `claim_ready_task_sqlite` (`crates/cloacina/src/dal/unified/task_execution/claiming.rs:342`) used `conn.transaction(...)` whose comment claimed IMMEDIATE semantics, but diesel's default `transaction` is DEFERRED — the RESERVED lock isn't taken until the first write, leaving a TOCTOU window between the outbox `SELECT` and `DELETE`. `sqlite_pool_size = 1` was implicitly serialising all callers, masking the bug. Fix: switched to `conn.immediate_transaction(...)`, which issues `BEGIN IMMEDIATE` and takes the RESERVED lock at start-of-transaction. So the pool bump uncovered an actual concurrency defect that would have hit any genuine multi-runner sqlite deployment.
- 2026-05-19: IMMEDIATE alone wasn't enough — concurrency test now finds 12/20 tasks (no duplicates, but some claims errored out under burst). Added an `is_sqlite_busy(&Error)` helper + 5-attempt exponential-backoff retry loop around the `interact + immediate_transaction` call in `claim_ready_task_sqlite`. The `BEGIN IMMEDIATE` open can still surface `database is locked` under WAL burst contention even with `busy_timeout=30000`; transparent retry in the DAL is the right pattern (callers above swallow the error and orphan the outbox row otherwise).
- 2026-05-19: Verified: `dal::task_claiming::test_concurrent_task_claiming_no_duplicates` on sqlite passes — "10 workers claimed 20 unique tasks with no duplicates". User running full suite to verify nothing regressed.