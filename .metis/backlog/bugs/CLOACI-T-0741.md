---
id: root-cause-the-flaky-sqlite
level: task
title: "Root-cause the flaky sqlite integration hang (not pool-checkout; retry-on-timeout is only a mitigation)"
short_code: "CLOACI-T-0741"
created_at: 2026-06-17T18:07:21.071791+00:00
updated_at: 2026-06-17T18:07:21.071791+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Root-cause the flaky sqlite integration hang (not pool-checkout; retry-on-timeout is only a mitigation)

## Objective

Find and fix the real cause of the chronically-flaky **sqlite** Python-scenario
integration hang, so the lane is reliably green without the retry crutch.

## Type / Priority
- [x] Bug — flaky CI (pre-existing; blocks/【blocked】 PR merges intermittently)
- [x] P2 — not data-corrupting, but it gates every PR and erodes trust in CI.

## Symptom
Python integration scenarios on the **sqlite** backend nondeterministically hang
and get killed at the 180s per-scenario timeout. The scenario varies run to run
(observed: `test_scenario_30_task_callbacks`, `_32_task_invokes_computation_graph`,
`_33_retry_condition`), and it hits both `ubuntu-latest` and `macos-latest`.
It is **pre-existing on main** (the #130 merge run was red on the same lane; the
#127 run before it was green — so it set in around #130 / a runner-image bump).
The postgres lane does **not** exhibit it.

## What it is NOT (ruled out)
- **Not a deadpool connection-checkout deadlock.** A bounded pool `wait_timeout`
  was added (`database/connection/mod.rs`); it **never fires** — scenarios hang
  the full 180s rather than erroring at the 30s pool wait. So the stall is not a
  pool-exhaustion wait.
- **Not the sqlite pool size.** The CI scenarios build with both features
  (postgres+sqlite), whose sqlite pool was already 4 (CLOACI-T-0622); unifying
  the sqlite-only path to 4 did not change their behaviour.
- **Not a specific scenario's logic** — it roams across unrelated scenarios.

## Leading hypotheses (to confirm with a captured stack)
- A **GIL ↔ tokio** interaction: a Python callback / PyO3 call holding the GIL on
  a blocking thread while the runtime needs it (the scenarios that hang involve
  callbacks, CG invocation, retries — all cross the Rust↔Python boundary under
  the in-process `shared_runner`).
- A **sqlite WAL / busy_timeout** stall under the multi-connection pool (size 4)
  with the `:memory:`-as-tempfile materialisation.
- A scheduler/await that never resolves outside the per-task `task_timeout`.

## How to actually fix (needs a repro + stack — the missing ingredient)
The hang does **not** reproduce locally on demand (passes locally + on most CI
runs), and the CI kill produces no core dump (killed, not crashed). So:
1. **Stress-repro:** loop the suspect sqlite scenarios (30/32/33) under the
   in-process runner until one hangs; or run them concurrently to raise
   contention.
2. **Capture the blocked stack** (`lldb`/`gdb` attach, or `py-spy dump` for the
   Python side + a Rust thread dump) to see exactly where it's parked.
3. Fix at the source (tighten connection/GIL scoping, or the await that stalls),
   then remove/relax the retry crutch.

## Current mitigation (already landed in #131 / commit on main)
`.angreal/test/_python_utils.py` retries a scenario **only on timeout** (3
attempts, logged); a non-zero return code fails fast (no retry) and a scenario
that hangs on every attempt still fails. This keeps real-regression signal while
absorbing the transient hang — but it is a **mitigation, not a fix**.

## Acceptance Criteria
- [ ] A reliable (even if slow/looped) local repro of the sqlite hang.
- [ ] A captured blocked stack identifying the stall site.
- [ ] A source fix; the sqlite lane is green **without** relying on the
      retry-on-timeout crutch (which can then be tightened or removed).
- [ ] No regression on postgres or the per-task timeout behaviour.

## Related
- Mitigation commit (retry-on-timeout) + pool `wait_timeout`/size unification landed in #131.
- [[CLOACI-T-0622]] — earlier sqlite-hang fix (pool 1→4 in the both-features path); this is its unfinished tail across the sqlite-only/scenario surface.

## Status Updates
- 2026-06-17: Filed after #131. Sharpened diagnosis: the hang is **not**
  pool-checkout (the added pool `wait_timeout` never fires). Retry-on-timeout
  landed as a mitigation so the lane stops gating PRs; the real fix is blocked on
  a captured stack from a stress-repro.
- 2026-06-17: **Experiment: sqlite pool=1 — DISPROVED.** Set SQLITE_POOL_SIZE
  back to 1 to serialize and kill the contention. Result: **27 executor
  integration tests fail deterministically** (pause_resume, task_execution,
  retry_condition). The engine holds multiple sqlite connections concurrently
  (scheduler loop + executor + heartbeat), so pool=1 starves/deadlocks the
  executor — which is exactly why T-0622 bumped it to 4. Reverted to 4.
  **Takeaway:** the flake is NOT fixable by pool size. Real avenues: WAL
  checkpoint/`wal_autocheckpoint` tuning, tightening connection scope so writers
  don't overlap (the `database is locked` came from concurrent context-saves),
  or accepting pool=4 + the retry-on-timeout mitigation. Also confirms a clue:
  the contention is concurrent WRITES (context saves) under WAL, not reads.
