---
id: investigate-flake-computation
level: task
title: "Investigate flake: computation_graph::resilience_tests::test_supervisor_individual_accumulator_restart"
short_code: "CLOACI-T-0530"
created_at: 2026-04-19T02:47:48.132108+00:00
updated_at: 2026-04-21T02:45:20.923636+00:00
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

# Investigate flake: `test_supervisor_individual_accumulator_restart`

## Objective

`angreal cloacina integration` intermittently fails in
`computation_graph::resilience_tests::test_supervisor_individual_accumulator_restart`
at `crates/cloacina/tests/integration/computation_graph.rs:1822:29`.
Observed during I-0097 e2e development:

- Full run summary: `294 passed; 1 failed; 6 ignored; 12 filtered out` —
  that 1 is this test.
- No correlation with the changes in the surrounding work (registry /
  reconciler / compiler refactor) — test touches the reactor's
  per-accumulator supervisor, not the package pipeline.

## Impact

- **Users affected**: contributors running full integration locally.
  Retry usually hides it; CI can flake on an unrelated PR.
- **Reproduction**: run `angreal cloacina integration`. First hit is
  intermittent; no known steady-state repro.

## Acceptance criteria

- [ ] Root cause identified and documented (race, timing assumption,
  DB state leak, pool exhaustion — unknown today).
- [ ] Test either passes deterministically or is marked `#[ignore]`
  with a linked follow-up issue if the underlying work is scoped out.
- [ ] Full `angreal cloacina integration` is green 5 runs in a row
  locally + 3 CI jobs in a row.

## Implementation notes

Starting places to poke at:

- `tests/integration/computation_graph.rs:1822` — the assertion site.
  Note what state is being asserted and what's supposed to have
  recovered by then.
- Accumulator supervisor logic in
  `crates/cloacina/src/computation_graph/accumulator/` — specifically
  the per-accumulator restart path.
- Health-channel + shutdown-signal interactions — prior incidents in
  this area have been races around the reactor's health warming-to-
  live transition.

No dependencies on other tasks. Standalone cleanup — pairs well with
CLOACI-T-0528 (reactor naming drift audit) since both touch the same
subsystem but can be done independently.

## Status Updates

### 2026-04-20 — Root cause & fix

**Root cause: fixed-duration `tokio::time::sleep` waits that are tight under
CPU contention from parallel test execution.** The test assumed specific
wall-clock timings for three async state transitions that are not guaranteed
under load:

1. The reactor's **warming-gate polling loop** sleeps 100ms between checks of
   accumulator health (`reactor.rs::run` near line 424). The test waited
   250ms after event 1 before asserting `fires_before >= 1`. Under load,
   reactor can take longer than 100ms to go Live, and the queued boundary
   isn't drained until after Live, so `fires_before` could be 0.
2. **Panic propagation through the spawned `accumulator_runtime` task is
   async.** The test waited a fixed 200ms after event 2 and then called
   `check_and_restart_failed` exactly once, asserting `restarted == 1`. If
   the task hadn't finished unwinding, `handle.is_finished()` returned
   `false` and the supervisor saw no crash to restart.
3. After restart, the test waited 200ms before asserting `fires_after >
   fires_before`. The restart path includes a 1s backoff
   (`BACKOFF_BASE_SECS = 1`) plus V2 spawn + event 3 processing; 200ms
   extra on top is usually enough but not guaranteed.

Sequential single-test reruns don't reproduce (30/30 and 20/20 passed
locally in both single-test and resilience_tests-module runs). The flake
surfaces inside the full `angreal cloacina integration` run where many
tests share a tokio worker pool and wall-clock timings slip.

**Fix (test-only):** replaced each fixed `sleep` with polling against the
actual observable state:

- Poll `fire_count >= 1` with a 5s deadline before reading `fires_before`.
- Poll `check_and_restart_failed()` in a loop (20ms cadence, 5s deadline),
  break when it returns ≥1 — this waits for the supervisor to actually see
  the panic rather than guessing how long unwinding takes.
- Poll `fire_count > fires_before` with a 5s deadline after event 3.

The production code is unchanged — only the test was racing its own
asynchronous subject.

**Verification:**
- Pre-fix: 30/30 + 20/20 sequential passes locally (flake only visible
  under full-suite contention).
- Post-fix: 30/30 sequential passes on the edited test locally.

**Related observation (not fixed here):** `registry.rs::register_accumulator`
pushes onto a `Vec` instead of replacing, so after an individual restart
the registry holds both the dead V1 socket and the live V2 socket for the
same accumulator name. `send_to_accumulator` still reaches V2, but stale
senders accumulate across restarts. Worth a follow-up task; out of scope
here since it wasn't the flake trigger.

**Acceptance criteria status:**
- [x] Root cause identified and documented (polling race against fixed
  sleeps, visible only under parallel-suite CPU load).
- [x] Test passes deterministically (polling replaces fixed sleeps).
- [ ] "5 runs in a row locally + 3 CI jobs in a row" — 30 local single-test
  runs pass; user chose to stop here rather than run the full integration
  suite 5× + CI 3×. CI verification still pending at reviewer's discretion.
