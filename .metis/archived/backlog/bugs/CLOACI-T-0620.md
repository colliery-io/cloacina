---
id: fix-reactor-predicate-test-panic
level: task
title: "Fix reactor predicate test panic in scheduler::reactor_predicate::test_predicate_filters_dispatch_and_advances_watermark_for_skips"
short_code: "CLOACI-T-0620"
created_at: 2026-05-19T14:25:58.723609+00:00
updated_at: 2026-05-19T14:42:02.931811+00:00
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

# Fix reactor predicate test panic

## Objective

Fix the panicking integration test `scheduler::reactor_predicate::test_predicate_filters_dispatch_and_advances_watermark_for_skips` that broke nightly CI on 2026-05-19. The test was added in the T-0602 follow-up PR (commit `8ec0a9ad`, "T-0602 followup: scheduler integration test + filtered-reactor example"). The PR's own CI passed (run 26033911223) but the test fails on main and in nightly.

## Backlog Item Details

### Type
- [x] Bug

### Priority
- [x] P1 — blocks nightly CI and `angreal test integration` on postgres

### Impact
- **Affected**: nightly CI postgres integration (job 76681443811 / 76681801677), main-branch CI on T-0610 push (run 26040170775).
- **Symptom**: panic at `crates/cloacina/tests/integration/scheduler/reactor_predicate.rs:305` (the `assert!(watermark.0 >= ts_second.0, …)` assertion — watermark did not advance past the filtered firing).
- **Repro**: `angreal test integration` against postgres.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Test passes deterministically in `angreal test integration` (postgres + sqlite) — user confirmed full integration suite passed.
- [ ] Nightly CI postgres integration job is green for at least one full run. (pending next nightly)
- [x] No new `--ignored` test added; root cause is fixed, not skipped.

## Implementation Notes

### Hypothesis

PR-only run passed; main + nightly fail. Likely candidates:
1. Test-ordering interaction — when run alongside other reactor-suite tests on the shared postgres fixture, watermark state for the same `(reactor, workflow)` row leaks across tests.
2. Time-source skew — if the watermark write path uses a re-fetched `now()` rather than the firing's stored `fired_at`, the comparison `watermark >= ts_second` can fail by sub-microsecond on postgres.

### Plan

1. Reproduce locally with `angreal test integration` against postgres.
2. Rerun with `RUST_LOG=cloacina::scheduler=debug,cloacina::reactor=debug` and `--nocapture` to inspect actual watermark vs `ts_second` values.
3. Likely fix: tighten the watermark write to use the firing's own timestamp (no re-`now()`), or relax the assertion to `>= ts_first.0` since the contract is "watermark advances past the observed firing(s)".

## Status Updates

- 2026-05-19: Filed from nightly run 26080699054 triage.
- 2026-05-19: Root cause identified. CI panic message: `watermark 2026-05-19T06:53:32.333473Z did not advance past the second firing 2026-05-19T06:53:32.333473856Z`. Postgres `TIMESTAMP` stores μs precision; `UniversalTimestamp::now()` returns ns precision (`chrono::Utc::now()`). The firing's `fired_at` is μs-truncated on the postgres roundtrip, so the watermark we read back is μs-aligned, but the test compared it to the original ns-precision `ts_second` — `.333473 < .333473856` by 856ns. The reason the original PR CI was green and main/nightly red is luck: it depends on whether `Utc::now()`'s sub-μs tail happens to be ≈0 at the call.
- 2026-05-19: Fix applied: in the test, μs-truncate `ts_first` before use (and derive `ts_second = ts_first + 1ms` so it stays μs-aligned). Production code is correct — only the test's precision assumption was wrong.
- 2026-05-19: User ran `angreal test integration` — full suite passed. Postgres + sqlite both green. Transitioning to completed; nightly verification will close the last criterion. Expecting [[CLOACI-T-0621]] (signing PoisonError cascade) to also clear since this was its likely root cause.