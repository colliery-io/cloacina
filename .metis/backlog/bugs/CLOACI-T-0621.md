---
id: fix-poisonerror-panic-in-signing
level: task
title: "Fix PoisonError panic in signing::reconciler_did_check::postgres_tests::test_find_signature_present_and_absent"
short_code: "CLOACI-T-0621"
created_at: 2026-05-19T14:26:00+00:00
updated_at: 2026-05-19T14:42:45.008741+00:00
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

# Fix PoisonError panic in signing reconciler did_check test

## Objective

Stop `signing::reconciler_did_check::postgres_tests::test_find_signature_present_and_absent` from panicking with `PoisonError` at `crates/cloacina/tests/integration/signing/reconciler_did_check.rs:71`. The panic site is `let fixture = fixture.lock().unwrap();` — a `std::sync::Mutex` whose guard was poisoned by an earlier panicking test.

## Backlog Item Details

### Type
- [x] Bug

### Priority
- [x] P2 — likely a downstream symptom of [[CLOACI-T-0620]], not a standalone defect

### Impact
- **Affected**: nightly CI postgres integration job (same job that fails 0620).
- **Symptom**: cascade — any test that locks the shared fixture Mutex after the first panic poisons and re-panics.
- **Note**: in the failing run, the immediately-prior failure is `scheduler::reactor_predicate::test_predicate_filters_dispatch_and_advances_watermark_for_skips`. If those two tests share a poisonable fixture (or if both share a single global postgres fixture Mutex), 0620 may be the sole root cause.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Verify whether this test fails in isolation. Confirmed via [[CLOACI-T-0620]] fix — full `angreal test integration` now passes including this test, so it was indeed a poison cascade.
- [ ] (Deferred) Broader hardening: 14 `.lock().unwrap()` call sites across `tests/integration/{signing,dal}/` are still poison-fragile. Not done here — would be a standalone tech-debt task if cascades reappear.
- [ ] Nightly CI postgres integration job is green. (pending next nightly; closed in [[CLOACI-T-0620]])

## Implementation Notes

### Plan

1. Confirm 0620 is the root cause by running this test in isolation against postgres.
2. Regardless of outcome, harden the fixture-Mutex unwrap sites in `tests/integration/signing/` (and any other suites that share the pattern) to recover from poisoning so future cascading failures are easier to read.
3. Re-run nightly after fixes.

## Status Updates

- 2026-05-19: Filed from nightly run 26080699054 triage. Likely blocked by [[CLOACI-T-0620]].
