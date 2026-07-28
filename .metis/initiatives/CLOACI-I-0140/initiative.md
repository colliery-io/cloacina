---
id: root-cause-the-pyo3-tokio-gil
level: initiative
title: "Root-cause the PyO3-tokio GIL instability — end the rotating Python scenario flake class"
short_code: "CLOACI-I-0140"
created_at: 2026-07-27T01:12:10.161328+00:00
updated_at: 2026-07-28T20:15:17.011269+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: root-cause-the-pyo3-tokio-gil
---

# Root-cause the PyO3-tokio GIL instability — end the rotating Python scenario flake class Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

The Python scenario integration tests fail intermittently in CI — and the failure has ROTATED across **at least eight different scenario files in one week** (July 2026 nightlies + PR lanes): scenarios 15, 16, 20, 24, 25, 26, 30, 32, 33. Three manifestations, one underlying class:

1. **Hangs** — the scenario never completes (scenarios 30/32/33, CI-mitigated by the `KNOWN_FLAKY_HANG` xfail allowlist in `.angreal/test/_python_utils.py`, T-0622).
2. **Segfaults** — `Fatal Python error: Segmentation fault`, core dumped on a **`tokio-rt-worker` thread** (scenario 20, July 21 nightly; CI now captures core dumps but the `.so` is cleaned before gdb runs → symbol-less backtraces).
3. **Assertion failures** — workflow ends `'Failed' == 'Completed'` (scenarios 15/16/24/25/26) — most likely the same crash landing in a task thread, failing the workflow instead of the process.

This is the documented PyO3↔tokio GIL interaction problem (memory: `project_scenario32_cg_invocation_deadlock`; T-0622): "never `with_gil` in an async executor body" is the standing rule; `spawn_blocking` isolation REDUCED but did not eliminate it. The allowlist mitigation structurally cannot keep up: it keys on filename and only covers hang/crash modes, while the class now manifests as ordinary assertion failures in arbitrary scenarios.

**Decision (user, 2026-07-26): root-cause it rather than widen the mitigation.**

Environment notes: failures concentrate in the **sqlite** lanes (both ubuntu and macos) but have appeared on postgres/ubuntu (scenario 20 segfault, 25); `--test-threads=1` is already in force; pytest runs with `timeout=10s` signal-based timeouts (pytest-timeout — SIGALRM into a process embedding a tokio runtime is itself a suspect); cloaca is the abi3 PyO3 extension (`cloaca.abi3.so`); the runtime bridge is `py_block_on` (I-0136 "GIL-safe py_block_on bridge") + `spawn_blocking` wrappers.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A deterministic (or high-probability) LOCAL reproducer for the crash class — the current once-a-night-somewhere rate is undebuggable.
- Symbolized evidence: backtraces with cloaca/tokio frames (fix CI's core-dump analysis to keep the `.so`; enable `faulthandler` + `PYTHONFAULTHANDLER`; local `lldb`/`gdb` on a repro).
- The actual mechanism identified and fixed (or bounded with a sound argument) — GIL re-entry on an executor thread, teardown-order UAF, signal-into-tokio, whatever it proves to be.
- The `KNOWN_FLAKY_HANG` allowlist SHRINKS (ideally to empty) instead of growing.

**Non-Goals:**
- Widening the xfail/retry mitigation (explicitly rejected in favor of root cause).
- Rewriting the Python authoring surface or replacing PyO3.
- Fixing unrelated UI-e2e nondeterminism (acme-connect readiness flake observed locally 2026-07-26 — separate track if it persists).

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

*(Discovery phase — this section records the investigation design; the fix design lands once the mechanism is known.)*

**Investigation axes (ranked by prior):**
1. **GIL acquisition on tokio worker threads** — audit every `Python::with_gil` / `Python::attach` in the cloaca + cloacina-python crates against the standing rule; special attention to callback paths (task callbacks — scenario 30's subject!), reactor/CG invocation (scenario 32's subject), and error paths.
2. **pytest-timeout SIGALRM into a tokio-embedded process** — signal delivery onto a thread holding the GIL or inside rusqlite/tokio can corrupt state; the 10s timeout co-occurring with the slow scenarios is suspicious. Test: switch a repro to `timeout_method=thread` and see if the class shifts.
3. **Interpreter/runtime teardown ordering** — runner drop vs live Python objects ("DefaultRunner dropping - consider calling shutdown()" appears in failing logs); abi3 `.so` unloaded while tokio threads still hold `PyObject`s.
4. **sqlite concentration** — sqlite's synchronous busy-wait paths hold threads longer (the `database table is locked` co-flake), widening any GIL-vs-runtime race window vs postgres.

**Tooling to build:**
- A stress harness (`.angreal` or a script): run ONE scenario in a loop N times with `PYTHONFAULTHANDLER=1`, core dumps enabled (`ulimit -c unlimited`), and the unstripped `.so` retained — stop on first failure, keep artifacts. Then bisect scenarios by hit-rate.
- CI core-dump fix: keep `cloaca.abi3.so` (and ideally the venv) alive through the gdb step so nightly captures become symbolized evidence even before a local repro exists.

## Alternatives Considered **[REQUIRED]**

- **Widen the mitigation** (retry-per-scenario-file on any failure in Python lanes): rejected by the user (2026-07-26) — masks real regressions, and the class already outgrew the allowlist once.
- **Serialize all Python execution behind one dedicated thread** (actor-style, no Python on tokio threads ever): held as a possible FIX shape, not an investigation shortcut — costs throughput and needs the mechanism confirmed first to know it's sufficient.
- **Drop pytest-timeout signal mode preemptively**: cheap and maybe right, but changing it before reproducing loses the diagnostic signal; folded into axis 2 instead.

## Implementation Plan **[REQUIRED]**

- **Phase 0 (discovery, this phase):** evidence capture + local repro harness + GIL-site audit. Exit: a repro with better than ~1-in-50 hit rate OR a symbolized backtrace pinning the mechanism.
- **Phase 1 (design):** mechanism writeup + fix proposal (small ADR if the fix constrains the bridge architecture).
- **Phase 2 (fix + verify):** land the fix; stress harness runs clean at N ≥ 500 iterations across the previously-worst scenarios; remove entries from `KNOWN_FLAKY_HANG`.
- **Phase 3 (guard):** CI keeps the symbolized-core capability; the stress harness becomes an opt-in angreal task for future regressions.

## Status Updates

### 2026-07-26 — Phase 0: GIL-site audit COMPLETE (subagent, all 121 sites). Mechanism hypothesis formed.

**Audit verdict:** the ~121 `with_gil` hits are mostly `#[cfg(test)]`. On real runtime paths there are **NO (A) clear violations and NO (C) await-while-GIL-held** — the I-0136 invariant holds. The residual risk is concentrated in two findings:

- **(B1) `bindings/runner.rs:250-256` — `impl Drop for AsyncRuntimeHandle` joins the actor thread WITH THE GIL HELD.** The explicit `shutdown` pymethod (runner.rs:980) correctly wraps the join in `py.allow_threads`; the Drop path has no `Python` token and cannot release. "DefaultRunner dropping — consider calling shutdown() explicitly" appears in failing runs' logs. If ANYTHING being joined ever needs the GIL, this deadlocks (→ the HANG mode).
- **(B2) `PythonTaskWrapper` (task.rs:124), `PythonGraphExecutor` (computation_graph.rs:693), both `PythonTriggerWrapper`s hold `PyObject`s with no explicit `Drop`, inside `Arc<dyn Task/Trigger>` owned by the RUNTIME** — final decref runs wherever the runtime drops them (a tokio worker), deferred by PyO3 to "a later GIL acquisition"… which, at TEST TEARDOWN, can be during/after interpreter finalization → **use-after-free → segfault on a `tokio-rt-worker` thread — exactly the July 21 core dump's shape.**
- **(D) fragile-but-safe:** ~14 sites `clone_ref` under `with_gil` directly on async worker threads (task.rs execute/callback clones :157-221, :381; PythonGraphExecutor::clone :717 via :786/:855 — the scenario-32 path; trigger polls) — correct only while they stay refcount-only.
- **`py_block_on` (gil.rs:47):** contract sound (GIL released around block_on, unconditionally); all 5 call sites satisfy the no-worker-thread precondition; convention-enforced only.

**Composed mechanism hypothesis (fits all three failure modes):** at scenario end, Python GC drops `PyDefaultRunner` → (B1) Drop joins runtime threads holding the GIL. Runtime threads still own (B2) PyObjects; their drops queue deferred decrefs. Depending on timing: [hang] a joined thread waits on the GIL the dropper holds → pytest-timeout → the XFAIL'd hang class; [segfault] interpreter finalizes with decrefs still pending on live tokio threads → UAF on a tokio-rt-worker; [assertion] the same crash landing inside a task body mid-run → workflow 'Failed'.

**Sqlite concentration explained:** sqlite's busy-wait/lock retries keep runtime threads alive LONGER at teardown, widening both races.

**Repro rig:** `.angreal/gil_stress.py` (loop one scenario, PYTHONFAULTHANDLER, cores kept, artifacts on first hit; `--timeout-method thread` flag for the axis-2 SIGALRM experiment). Local venv = uv python 3.12.12 (CI parity; NOTE machine has no homebrew — uv only, explicit interpreter paths). Wheel building (sqlite,macros).

**Next:** stress scenario 30 to get a baseline hit-rate → then A/B: (1) explicit `runner.shutdown()` in the failing scenarios' teardown (isolates B1), (2) a patched Drop that routes through `Python::allow_threads` equivalent, (3) `--timeout-method thread`.

### 2026-07-26/27 — REPRO ACHIEVED (1-in-~25); first hypothesis FALSIFIED; hunting with symbols.

- **Local build gotcha (cost an hour):** the first wheel linked `Python3.framework/3.9` (stale PyO3 build-script config from a system-python attempt) → deterministic `PyInterpreterState_Get` abort at import on 3.12. Fix: `PYO3_PYTHON=<venv>/bin/python` + rebuild; `.so` now links the uv 3.12 dylib. (Unrelated to the CI bug; recorded for future local work. Also: machine is uv-only, no homebrew; venv = uv 3.12.12.)
- **Repro:** `.angreal/gil_stress.py` on scenario 30, sqlite, CI-parity pytest flags → hangs at iters 30, 24, 10 (~1-in-20-30). ALWAYS at the start of `test_on_failure_callback_called`. The hang OUTLIVES pytest-timeout's SIGALRM — main thread is C-blocked in a pymethod, never re-enters the eval loop, handler never runs (explains the CI hang class silencing).
- **Stacks captured live (lldb; py-spy needs sudo on macOS):** main thread = pymethod → `pthread_cond_wait` inside cloaca; a second non-tokio cloaca thread also in cond_wait; ALL tokio-rt-workers parked. Release `.so` unsymbolized → shapes only.
- **Fix attempt #1 (falsified as THE mechanism, kept as hygiene):** `spawn_runtime`'s `init_rx.blocking_recv()` was the only GIL-held channel wait (constructor path) — wrapped in `py.allow_threads` (+ threaded `py` through `new`/`with_config`/`with_schema` + tests); Drop-join now routes through `with_gil→allow_threads` (audit B1). **Hang persisted (hit at iter 10)** → the init wait was not the mechanism. Both changes are still correct-by-rule and stay.
- **Revised leading hypothesis:** with `send_and_recv` provably GIL-releasing, a pure GIL story can't explain the persistent hang. New suspect: **circular wait through the actor event loop** — `execute` blocks on `response_rx` while the event loop `block_on`s the workflow; the failing task's `on_failure` PYTHON callback calls back into a runner/context API that `send_and_recv`s into the SAME single-consumer event loop → circular wait, no GIL required. Fits: only callback tests hang; both cloaca threads in cond_wait; workers idle.
- **In flight:** wheel rebuilt with `CARGO_PROFILE_RELEASE_DEBUG=true` (symbolized frames) — next hit's lldb dump will NAME the two condvar waits and settle it.

### 2026-07-27 — MECHANISM CONFIRMED (symbolized): it was never the GIL. One root cause, three symptoms.

**Getting symbols** (for the record): maturin strips via `[tool.maturin] strip = true` in pyproject regardless of cargo profile; direct `cargo build` of the extension fails at link on macOS (maturin injects `-undefined dynamic_lookup`). Working recipe: temporarily flip `strip = false` + `CARGO_PROFILE_RELEASE_DEBUG=true` + maturin → 108k-symbol wheel (flip reverted, not committed).

**The symbolized hang (iter 21) names everything:**
- Main thread: `BlockingRegionGuard::block_on(f = Receiver<Result<WorkflowExecutionResult, …>>)` — the `execute()` pymethod's `send_and_recv`, GIL RELEASED, waiting on the workflow-result oneshot.
- Thread 2: `spawn_runtime` closure → `Runtime::block_on(run_event_loop)` parked — and `run_event_loop` `tokio::spawn`s Execute, so the stalled thing is the spawned `runner.execute` FUTURE, suspended forever.
- Every tokio worker idle; NO thread executing Python; NO thread waiting on the GIL. **Not a GIL deadlock. Not Python-in-Python contention. A lost terminal state.**

**ROOT CAUSE:** `executor/result_handler.rs` — under sqlite write contention (`database is locked`, seen verbatim in the rc1 artifact):
1. Context-save fails → task marked Failed → workflow honestly Failed → the **assertion class** (`'Failed' == 'Completed'`, rotates across any context-writing scenario).
2. `mark_failed` ITSELF fails and was **swallowed** (`let _ =`, two sites) → task row stays Running with a live claim → workflow never terminal → `runner.execute` future never resolves → pymethod blocks on oneshot → main thread C-blocked → pytest-timeout SIGALRM silenced → the **hang class**. Callback scenarios dominate because their tasks fail BY DESIGN — every run drives the swallowed-write path.
3. Segfault class: expected downstream (killed/timed-out process finalizing while leaked runner threads hold PyObjects — audit B2); verify it disappears with the primary fix.

**FIX:** `retry_transient` (5 attempts, linear backoff, transient-error match: sqlite busy/locked + pg deadlock/serialization) around the three terminal writes — `complete_task_transaction` (fixes assertion class) and both `mark_failed` sites (fixes hang class), with a loud ERROR (never a silent drop) on exhaustion + the stale-claim sweeper as the eventual backstop. GIL-hygiene fixes from earlier (init-wait allow_threads, GIL-safe Drop) kept as hardening.

**Verification in flight:** 500-iteration stress on scenario 30 (baseline 1-in-~25; clean run ≈ p<1e-8).

### 2026-07-27 — VERIFIED: 500/500 clean

Rebuilt the wheel with the `retry_transient` fix (sqlite,macros lane) and ran the stress harness 500 iterations on scenario 30 with CI-parity flags: **`outcomes={'ok': 500}`** — zero hangs, zero assertion failures, zero signals. Against the measured 1-in-~25 baseline that's ~p 1e-9 of the fix being a no-op. Remaining: land the fix (branch `fix/i0140-terminal-state-writes`), let nightly soak, then shrink `KNOWN_FLAKY_HANG` and confirm the segfault class died with the hang class.

### 2026-07-27 — Round 2: the retry branch had the same hole (nightly caught it)

PR #201 squash-merged; manual nightly dispatched. **Audit of the job logs before shrinking the allowlist** (user's prompt — right call): 3 of 4 integration legs passed scenarios 30/32/33 outright, but **sqlite/ubuntu scenario 33 (retry_condition) hung 180s on attempt 1** and was rescued by the harness retry loop. Green-by-rescue, not green.

Residual mechanism — same swallowed-terminal-write shape, in the branch round 1 didn't touch:
- `result_handler.rs` retry branch: `schedule_task_retry` failure was warn-and-dropped → task Running forever. (`should_retry_task` is pure — its `unwrap_or(false)` is inert.)
- `thread_task_executor.rs`: three more `let _ = mark_failed(...)` sites (invalid namespace / task not found / context build failed).

Round-2 fix: `retry_transient` around `schedule_task_retry` with a **fail-instead-of-limbo fallback** (if scheduling still fails after retries: mark_failed + return failure — never leave the row Running); `mark_failed_reliably` helper in thread_task_executor for the three swallowed sites; `retry_transient`/`is_transient_db_error` now `pub(crate)`. Swept executor/scheduler/dispatcher for remaining `let _ =` DB writes — none left (rest are channel sends/gauges).

Verification: 300-iter local stress on scenario 33 in flight (regression check — the 33 hang was only ever seen on ubuntu; macOS never repro'd it pre-fix, so the linux nightly is the real arbiter). Lesson: when a mechanism is confirmed, audit EVERY error path owning that state transition in one sweep — not just the site the stack trace names.

### 2026-07-27 — Round 3: the scheduling ENTRY had the same exposure (nightly caught it again)

Round-2 verified locally (scenario 33: **300/300 clean**, ~1.2s/iter vs ~3s pre-fix — retry scheduling visibly healthier), PR #202 squash-merged. Ops footnote: the post-merge nightly initially wedged — the `cloacina-tests-refs/heads/main` concurrency group jammed after the preemption dance (push CI queued 35+ min, runners available, GitHub all-operational); cancelling every run touching the group cleared it. If main CI ever sits queued mysteriously: look for a half-dead run holding `cloacina-tests`.

The round-2 nightly's sqlite/ubuntu leg then failed BEFORE reaching the Python scenarios — in the RUST integration suite: `secret_no_leak`, `execute_async` → `schedule_workflow_execution` → `database table is locked: task_executions` (SQLITE_LOCKED, shared-cache class — busy_timeout does NOT cover it; same family as the July 15 unit-test flake). The disease at the ENTRY write, not the terminal write.

Round-3 fix (`runner/default_runner/workflow_executor_impl.rs`): `retry_transient` around both `schedule_workflow_execution` call sites (execute / execute_async; context re-supplied via `clone_data()` per attempt), the `execute` status-poll read, and `get_execution_status` (backs every `wait_for_completion` loop). The failing test passes locally with the fix.

Running tally of one mechanism, N surfaces: terminal writes (round 1) → retry scheduling + executor error paths (round 2) → workflow scheduling entry + status reads (round 3). The class is "any unretried DB access on the execution path, surfaced wherever a test unwraps or a state machine stalls."

### 2026-07-28 — Segfault class: hunt results + structural derisk (round 4)

**Hunt results:** the segfault class does NOT reproduce locally — 400/400 clean on macOS/postgres (scenario 16) and 400/400 clean on arm64-linux/postgres in docker (scenarios 27+16, unstripped wheel, gdb armed). Two CI kills in two days meanwhile (scenario 16, then 27 — postgres/ubuntu both). Remaining differentiators: x86_64 and GitHub's 2-core runner timing. CI evidence pins the crash *site*: faulthandler shows the main thread inside pytest's `collect_unraisable` — a finalizer raised, and processing the exception touched freed memory → refcount corruption/UAF from the extension during teardown (audit B2 shape). The nightly's gdb core capture is useless today (stripped wheel + gdb loads the wrong binary + it catches the signal re-raise frame).

**Decision (user): derisk the teardown race structurally instead of chasing an environment-specific repro.**

**Round-4 changes (branch fix/i0140-teardown-derisk):**
1. `_shutdown_all_runners` atexit backstop — every live runtime registers in a global `LIVE_RUNNERS` (Weak refs); the wheel's pymodule init registers an atexit hook joining ALL runtime threads before interpreter finalization begins. Invariant: no runtime thread outlives the interpreter; no PyObject decref into a finalizing interpreter. Forgetting `runner.shutdown()` is now SAFE — a product fix, not a test fix.
2. `AsyncRuntimeHandle::drop` guards: no-op (zero GIL) when already joined; after `ATEXIT_FIRED`, degrades to a best-effort shutdown signal WITHOUT join/GIL — leak-on-exit is strictly safer than a decref into a dying interpreter.
3. conftest.py: removed the SIGALRM abandon-on-timeout machinery — it interrupted `shutdown()` mid-join and left runtime threads alive going into finalization (a live instance of the exact race; predates the I-0140 shutdown fixes).
4. gil_stress.py: harvests macOS `.ips` crash reports on signal exits.

**Verified:** leak test (create runner, exit WITHOUT shutdown) → atexit fires, "Received shutdown signal" logged, clean rc 0. Scenarios 30+16 smoke-pass on the new wheel.

**Still open for the segfault class:** CI symbolization fix (unstripped nightly test wheel + gdb pointed at the python binary) so any post-derisk kill self-documents.

### 2026-07-28 — Rounds 5+6 landed; MILESTONE: first nightly with the allowlist EMPTY, genuinely green

- **#206 (round 5):** `KNOWN_FLAKY_HANG` emptied — gate was the 2026-07-28 scheduled nightly: all four legs, 29/29 scenarios, zero rescue markers on rounds 1-3 alone. Comment block in `_python_utils.py` records the story and forbids re-allowlisting over root-causing. Machinery kept but inert (keys off the empty set; rescued infra flakes still print auditable TIMEOUT markers).
- **#207 (round 6):** the #206 merge's push CI flushed out surface #6 — delivery sweeper: `concurrent_sweepers_are_race_safe` failed when BOTH racing sweepers hit `database table is locked: delivery_outbox` and misread the transient lock as a CAS loss → nobody reset the row (prod shape: redelivery silently deferred a sweep interval; likely the July 15 unit-lane flake). Fix: `retry_transient` around `reset_to_pending` — CAS losses are a distinct error shape and still skip. Race test 60/60 clean.
- **MILESTONE nightly (30380037247, full stack + EMPTY allowlist):** all four integration legs 29/29, zero rescue markers, zero segfaults — the first unassisted-green nightly. All initiative goals now met: mechanism identified + fixed (6 surfaces), local repro harness built, allowlist empty, nightly genuinely green.
- Ops noise for the record: #206's `Discover Examples` job was runner-cancelled mid-disk-cleanup and its auto-requeue wedged in `queued` limbo — cancel + fresh rerun fixed it (second wedge instance; pattern noted round 3).

**Remaining before close:** CI symbolization insurance (unstripped nightly wheel + gdb at the python binary), a few scheduled-nightly soaks, then phase transitions + close-out.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}
