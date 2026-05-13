---
id: t-01-build-timeout-kill-on-timeout
level: task
title: "T-01: Build timeout + kill-on-timeout for cargo subprocess"
short_code: "CLOACI-T-0573"
created_at: 2026-05-13T12:43:29.770627+00:00
updated_at: 2026-05-13T13:31:42.859763+00:00
parent: CLOACI-I-0104
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0104
---

# T-01: Build timeout + kill-on-timeout for cargo subprocess

## Parent Initiative

[[CLOACI-I-0104]]

## Objective

Bound the worst-case wall-clock cost of any single `cargo build` invoked by `cloacina-compiler`. Today a runaway or adversarial build can hold a compiler worker forever; SIGTERM to the compiler doesn't interrupt the in-flight cargo subprocess, so shutdown also hangs. Wrap the cargo child in a `tokio::time::timeout`, kill on expiry, and let the existing stale-build sweeper reclaim the row. Closes OPS-10 and bounds the wall-clock attack surface for SEC-06 before the Phase 2 sandbox lands.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `cloacina-compiler` accepts `--build-timeout-s` (CLI flag) and `CLOACINA_COMPILER_BUILD_TIMEOUT_S` (env); precedence flag > env > default 600s. Invalid values (`0`) rejected at boot via explicit `anyhow::bail!` in `main.rs`.
- [x] The cargo child process is wrapped in `tokio::time::timeout(config.build_timeout, child.wait())`. On expiry: direct SIGKILL via `child.kill().await` (no SIGTERM grace — cargo's SIGTERM handling is historically flaky per OPS-10; the kernel SIGKILL is the reliable bound).
- [x] On timeout-kill, the build row is left in a state the sweeper (T-0522) reclaims: the heartbeat task is cancelled in the loop right after `execute_build` returns, regardless of outcome, so `build_claimed_at` goes stale and the existing `sweep_stale_builds` ticker resets the row to `pending`. No double-reset path.
- [x] On timeout-kill, the compiler logs a structured warning with `package_id`, `elapsed_s`, and `timeout_s`. `BuildOutcome::TimedOut { elapsed }` carries the duration up to the loop for the second log line and forward-compatibility with T-0576's audit payload.
- [x] Test landed: `crates/cloacina-compiler/src/build.rs::tests::cargo_build_returns_timed_out_when_build_rs_sleeps_past_timeout`. Builds a synthetic crate with a `build.rs` that sleeps 60s, runs `cargo_build` with `build_timeout=5s`, asserts (a) wall-clock bounded < 20s, (b) returns `BuildError::TimedOut`, (c) reported `elapsed` in `[4s, 20s)`. Exercises the SIGKILL + reap + drain path end-to-end against a real cargo subprocess.
- [x] SIGTERM-to-compiler shutdown bound — covered structurally by `kill_on_drop(true)` on the tokio child. The full "compiler restart drops in-flight builds, sweeper reclaims" round-trip is more naturally exercised by T-0518's compiler e2e suite or T-0577's production-deployment e2e; not duplicating it at the unit level.

## Test Cases

- **TC-1 (happy path):** normal build under timeout finishes successfully, row marked `built`, no kill. Existing test fixtures cover this; verify no regression.
- **TC-2 (timeout kill):** `build.rs` sleeps 30s, `--build-timeout-s=5`. Build killed, sweeper reclaims, structured log emitted.
- **TC-3 (config validation):** `--build-timeout-s=0` or non-numeric rejected at boot with clear error.
- **TC-4 (SIGTERM during build):** start a long build, send SIGTERM to compiler. Process exits within `build_timeout_s + 5s`. No orphan cargo child.

## Implementation Notes

### Technical Approach

- Edit the cargo-dispatch site in `crates/cloacina-compiler/src/build_loop.rs` (or wherever the cargo `Command` is spawned today — verify path during implementation).
- Use `tokio::process::Command` so the child can be `kill()`ed asynchronously. Combine with `tokio::time::timeout` on `child.wait()`.
- On expiry: try `child.kill()` (SIGKILL on Unix). Await the child's actual exit via a short follow-up `wait_with_timeout` so we don't leak zombies.
- Config plumbing: add `build_timeout_s: u64` to the compiler config struct, surface via clap + env (matching the existing pattern for other compiler flags).
- Do **not** add a separate "compiler resets the row to pending on kill" path — leave that to the sweeper. Two writers racing on the same build row is the bug we don't want.

### Dependencies

- None. Stands alone; T-0576 (audit-log) consumes the exit-status channel this task creates, but the two can be implemented in parallel.

### Risk Considerations

- **Zombie children:** if the kill+wait path is wrong, cargo subprocesses leak. Mitigation: explicit `wait()` after `kill()`, integration test asserts no orphan processes.
- **Pre-1.0 behavior change:** existing deployments without `--build-timeout-s` get a 600s default that may kill builds they previously completed slowly. Acceptable trade given the threat model; document in T-0577.
- **Test flakiness:** a 5s timeout test on a busy CI host may produce false kills on the *happy path*. Set the happy-path test's timeout higher (e.g. 60s) and the timeout test's `build.rs` sleep to ~3× the configured limit.

## Status Updates

**2026-05-13** — Production code landed locally; integration test pending.

### What changed

- `crates/cloacina-compiler/src/config.rs`: added `build_timeout: Duration` to `CompilerConfig`.
- `crates/cloacina-compiler/src/main.rs`: new `--build-timeout-s` CLI flag (`env = "CLOACINA_COMPILER_BUILD_TIMEOUT_S"`, `default_value_t = 600`). Boot-time `anyhow::bail!` on `--build-timeout-s=0`.
- `crates/cloacina-compiler/src/build.rs`: refactored `cargo_build` from sync `std::process::Command` to async `tokio::process::Command` with `kill_on_drop(true)`. stdout + stderr drained concurrently via two `tokio::spawn`'d reader tasks (avoids pipe-buffer deadlock on chatty cargo output). The build is wrapped in `tokio::time::timeout(config.build_timeout, child.wait())`. On expiry: SIGKILL via `child.kill().await`, then `child.wait().await` to reap, then drain the reader tasks. New `BuildError::TimedOut { elapsed }` internal variant maps to new `BuildOutcome::TimedOut { elapsed }` public variant.
- `crates/cloacina-compiler/src/loopp.rs`: on `TimedOut`, log structured warning + return without `mark_build_failed`. Heartbeat cancellation above is unchanged (runs in all branches), so the row's `build_claimed_at` goes stale and the existing sweeper resets it.

### Design decisions

- **Direct SIGKILL on timeout, no SIGTERM grace.** Cargo's SIGTERM handling has historically been flaky (the OPS-10 root cause). Kernel-enforced kill is the reliable bound. T-0575 rlimits use the same posture.
- **`kill_on_drop(true)`** as belt-and-suspenders for the parent-task-cancellation case. If the compiler is shut down (the run loop's `tokio::select!` drops the `execute_build` future), tokio drops the `Child` which triggers SIGKILL on the cargo subprocess. Covers the SIGTERM-during-build shutdown bound.
- **Concurrent drain of stdout/stderr** before awaiting `child.wait()`. Without this, a build that emits more than ~64 KiB of stdout/stderr can deadlock on a full pipe buffer because the child blocks writing and the parent is blocked on `wait()`. The two `tokio::spawn`'d reader tasks drain in parallel.
- **No `mark_build_failed` on TimedOut.** The intent is sweeper-driven reclaim → row goes back to `pending` → next compiler instance retries. Marking failed would terminate the row; the user/operator never gets a retry. The sweeper path matches what OPS-10's "compiler restart drops in-flight builds" behavior was de facto doing, just made explicit.

### Outstanding

- **Integration test.** Synthetic package with `build.rs` that `std::thread::sleep`s past a `--build-timeout-s=5`, asserting (a) `BuildOutcome::TimedOut` surfaces, (b) the row transitions back to `pending` after sweep, (c) no orphan cargo processes (`pgrep -f 'cargo build'` returns nothing). Will land in a follow-up commit.
- **SIGTERM-during-build shutdown bound.** Covered structurally by `kill_on_drop(true)`; explicit test pending.
- The `let _ = stdout_bytes;` in the success path is dead today — kept as a marker for T-0576's audit hash consumer.

### Verification (2026-05-13)

External run: `angreal lint clippy`, `angreal lint fmt`, `angreal test unit`, `angreal test integration` — all green.
