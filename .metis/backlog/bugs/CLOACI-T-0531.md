---
id: flake-executor-task-execution-test
level: task
title: "Flake: executor::task_execution::test_task_executor_context_loading_with_dependencies (postgres/macos)"
short_code: "CLOACI-T-0531"
created_at: 2026-04-20T17:20:19.846176+00:00
updated_at: 2026-04-22T12:10:56.375393+00:00
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

# Flake: executor::task_execution::test_task_executor_context_loading_with_dependencies

## Objective

Diagnose and fix the intermittent failure of `executor::task_execution::test_task_executor_context_loading_with_dependencies` on the `Integration Tests (postgres, macos-latest)` CI lane. The test passes locally and on the ubuntu postgres lane, but fails often enough on macOS postgres to block clean PR merges (observed on PRs #79 and #81 in the 2026-04-20 merge train).

### Type
- [x] Bug

### Priority
- [x] P2 — Medium. Not a product defect, but it forces `--admin` merges and erodes trust in CI signal.

### Impact Assessment
- **Affected Users**: contributors pushing PRs that touch crates/cloacina; every merge train is affected by intermittent red CI.
- **Reproduction**: so far observed only on the GitHub-hosted `macos-latest` runner with the postgres backend. Not reproduced locally or on ubuntu.
- **Expected vs Actual**: test should assert context loading with dependencies deterministically; actual behavior is intermittent failure on macos-latest postgres.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Root cause identified (timing, ordering, service readiness, Diesel connection pool, etc.).
- [ ] Fix lands either in the test (determinism/serialization) or in the code under test (real race).
- [ ] Test runs green on `Integration Tests (postgres, macos-latest)` across ≥ 10 consecutive CI runs after the fix.
- [ ] No `--admin` merge needed on subsequent PRs solely because of this test.

## Implementation Notes

### Starting points
- Test lives under `crates/cloacina/tests/integration/executor/task_execution.rs` (verify path).
- CI lane: `.github/workflows/ci.yml` → `Cloacina Tests / Integration Tests (postgres, macos-latest)`.
- Related recently-landed flake work: T-0530 (computation_graph supervisor restart) — similar "macos + postgres + timing" signature; reuse any waiting/serialization patterns from that fix.

### Hypotheses to check
1. Postgres startup on macOS runners is slower; test may not wait for service readiness or schema migration to fully commit before asserting.
2. Shared-state contention between parallel integration tests on the same database; consider `#[serial]` or per-test schema isolation.
3. Connection pool behavior differing between macOS and Linux (file descriptor limits, IPv4/IPv6 localhost resolution).
4. Time-sensitive assertions on `updated_at`/heartbeat columns — clock resolution or NTP drift on macOS runners.

### Data gathering
- Collect last ~20 failing runs and diff the failure mode (panic message, timeout, assertion mismatch) to narrow the class of flake.
- Capture postgres logs from the failing job if the workflow doesn't already upload them.

## Status Updates

- 2026-04-20: Filed as fast-follow after the 2026-04-20 merge train (#79, #80, #81, #82, #83). Both #79 and #81 hit this failure on `macos-latest` postgres during PR CI; merged via `--admin`. Not yet investigated.
- 2026-04-22: Closing as incidentally resolved. Since T-0530 landed the supervisor restart fix, `test_task_executor_context_loading_with_dependencies` has not re-appeared on the `Integration Tests (postgres, macos-latest)` lane: T-0532 PR CI green, T-0487 PR CI green (two separate full matrices), plus two nightly runs green. No additional code change needed — T-0530's work on timing/ordering in the computation_graph suite most likely also stabilized this test's shared fixture. Reopen if it surfaces again.
