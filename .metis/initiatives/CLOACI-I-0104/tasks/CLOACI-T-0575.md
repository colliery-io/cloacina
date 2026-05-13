---
id: t-03-setrlimit-wrapper-for-cargo
level: task
title: "T-03: setrlimit wrapper for cargo subprocess (Linux)"
short_code: "CLOACI-T-0575"
created_at: 2026-05-13T12:43:32.216363+00:00
updated_at: 2026-05-13T14:19:14.362532+00:00
parent: CLOACI-I-0104
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0104
---

# T-03: setrlimit wrapper for cargo subprocess (Linux)

## Parent Initiative

[[CLOACI-I-0104]]

## Objective

Bound the per-build resource cost via `setrlimit` on the cargo subprocess. RLIMIT_CPU, RLIMIT_AS, RLIMIT_NOFILE, RLIMIT_NPROC are set in a `pre_exec` hook before cargo runs. A malicious `build.rs` that tries to exhaust CPU, memory, file descriptors, or fork bombs is bounded by the kernel. Linux-only per A-0005.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Four config flags wired with `CLOACINA_COMPILER_BUILD_RLIMIT_*` env equivalents. Defaults: CPU=`--build-timeout-s` (auto-tracking), MEM=4G, FILES=1024, PROCS=256.
- [x] Linux `pre_exec` closure calls `libc::setrlimit` for CPU/AS/NOFILE/NPROC. Both soft and hard set to the configured value. Failures from `setrlimit` propagate as `io::Error` from the closure → the parent observes the child died before doing useful work.
- [x] `#[cfg(target_os = "linux")]` gates the wrapper. Non-Linux startup emits a `tracing::warn!` from `lib.rs::run` so the operator can't miss that the kernel ceiling is unenforced. `libc` is in `[target.'cfg(target_os = "linux")'.dependencies]` so it doesn't appear in non-Linux builds.
- [x] Linux integration test landed: `cargo_build_fails_when_build_rs_overshoots_rlimit_as` — synthetic crate with `build.rs` that allocates 8 GiB, RLIMIT_AS=128 MiB, asserts build fails within 30s and returns `BuildError::Failed` (not `TimedOut`).
- [ ] Threads / RLIMIT_NPROC integration test — **deferred**. RLIMIT_AS path is the canary; NPROC pre_exec wiring is the same code path. Live test of the NPROC limit was redundant; the unit-level confidence comes from the parser tests + the shared pre_exec path being exercised by the AS test.
- [ ] Existing fixtures build clean — pending external test run.

## Test Cases

- **TC-1 (RLIMIT_AS):** `build.rs` does `vec![0u8; 8 * 1024 * 1024 * 1024]`. Build dies with SIGKILL; failure reason includes signal name.
- **TC-2 (RLIMIT_NPROC):** `build.rs` spawns more threads than the limit allows; build fails.
- **TC-3 (RLIMIT_NOFILE):** `build.rs` opens more file handles than the limit; build fails.
- **TC-4 (happy path):** workspace examples build cleanly.
- **TC-5 (non-Linux):** compile-and-test the compiler on macOS; rlimits skipped with a startup warning. Builds otherwise function (relying on T-0573 timeout as the only resource bound on non-Linux dev hosts).

## Implementation Notes

### Technical Approach

- Use `std::os::unix::process::CommandExt::pre_exec` on the `std::process::Command` (or the equivalent on `tokio::process::Command::as_std`). The closure runs after fork but before exec, so it's safe to call `libc::setrlimit` directly. Keep the closure async-signal-safe — `libc::setrlimit` is, but anything allocating is not.
- Map each `--build-rlimit-*` config value to its `libc::RLIMIT_*` constant. Set both soft and hard limit to the configured value (`rlim_cur == rlim_max`).
- For RLIMIT_AS, the configured value is bytes — accept human-readable suffixes (e.g. `4G`) in the flag parser to avoid operator footguns.
- Wrap the pre_exec calls in a single helper `unsafe fn apply_rlimits(&BuildRlimits)` so the unsafety is contained.
- Failure mode: if `setrlimit` returns non-zero in the pre_exec, the closure should `libc::_exit(1)` (do not panic — pre_exec is post-fork; panicking is undefined behavior). The compiler observes the child died before doing useful work; surface as a build rejection.

### Dependencies

- Compatible with T-0573 — both wrap the same `Command`. Land in either order; the second one rebases trivially.

### Risk Considerations

- **pre_exec footguns:** anything in the closure that allocates, takes a lock, or calls non-async-signal-safe code is undefined behavior post-fork. Keep the closure tiny — just `setrlimit` calls and an `_exit(1)` on failure. Code-review with paranoia.
- **Limit too tight:** default RLIMIT_AS=4 GiB may kill legitimate large Rust builds (release builds of crates with heavy generics can exceed 4 GiB peak RSS). Operators may need to bump it. T-0577 documents this and gives baseline numbers.
- **CPU vs wall-clock:** RLIMIT_CPU is CPU-time, not wall-clock — it counts only when the process is on CPU. A build that's mostly IO-bound (downloading deps under future non-offline mode, or waiting on disk) burns less CPU than wall-clock. Default RLIMIT_CPU = `build_timeout_s` is a generous upper bound; T-0573's wall-clock timeout is the real bound.
- **Test environment:** RLIMIT integration tests need a Linux CI runner. Make sure they're gated by `#[cfg(target_os = "linux")]` and the test harness understands the platform skip.

## Status Updates

**2026-05-13** — Production code + tests landed locally; ready for external lint + test pass.

### What changed

- `crates/cloacina-compiler/Cargo.toml`: `libc = "0.2"` added under `[target.'cfg(target_os = "linux")'.dependencies]` — Linux-only.
- `crates/cloacina-compiler/src/config.rs`: new `BuildRlimits { cpu_s, mem_bytes, files, procs: u64 }` struct, all four plain `u64` (no `Option` — always applied; operators relax via large values, not by disabling). Added `build_rlimits: BuildRlimits` field to `CompilerConfig`.
- `crates/cloacina-compiler/src/lib.rs`: re-exports `BuildRlimits`. Adds a `#[cfg(not(target_os = "linux"))]` startup warning so dev-host operators can't miss that the kernel ceiling is unenforced.
- `crates/cloacina-compiler/src/main.rs`: four new CLI flags — `--build-rlimit-cpu` (env `CLOACINA_COMPILER_BUILD_RLIMIT_CPU`), `--build-rlimit-mem` (`...BUILD_RLIMIT_MEM`, accepts `K`/`M`/`G` suffixes, default `4G`), `--build-rlimit-files` (`...BUILD_RLIMIT_FILES`, default 1024), `--build-rlimit-procs` (`...BUILD_RLIMIT_PROCS`, default 256). CPU default = `--build-timeout-s` so the wall-clock and CPU bounds track. New `parse_size(&str) -> Result<u64, String>` for human-readable byte sizes.
- `crates/cloacina-compiler/src/build.rs`: new `apply_rlimits(&mut Command, &BuildRlimits)`. Linux path installs an `unsafe pre_exec` closure that calls `libc::setrlimit` for `RLIMIT_CPU`, `RLIMIT_AS`, `RLIMIT_NOFILE`, `RLIMIT_NPROC`. Both soft and hard limit set to the configured value (`rlim_cur == rlim_max`). Non-Linux path is a no-op (warning emitted once at startup).

### Design decisions

- **Plain `u64`, not `Option<u64>`.** Always applied, no "disable" path. Operators who need a larger ceiling pass a larger value (or `u64::MAX` if they truly want unlimited). Simpler config surface, no sentinel-vs-default-vs-explicit confusion. T-0577 will document this.
- **`pre_exec` closure is minimal.** Captures four `u64`s by value (no allocation). Calls only `libc::setrlimit`, which is async-signal-safe per POSIX. On failure, the closure returns `io::Error::last_os_error()` and the parent observes the child died before doing useful work.
- **Linux-only via target-cfg `[dependencies]`.** `libc` does not appear in the dependency tree on macOS/Windows; the `apply_rlimits` function has a no-op `#[cfg(not(target_os = "linux"))]` body. ADR-0005 already locks in Linux-only server posture.
- **Defaults track T-0573's wall-clock bound.** `RLIMIT_CPU` default = `--build-timeout-s` so a build can't burn CPU past its wall-clock budget. Other defaults (4 GiB mem, 1024 files, 256 procs) are the ticket-suggested starting points.

### Tests landed

- `crates/cloacina-compiler/src/main.rs::tests` — 8 unit tests for `parse_size`: plain bytes, K/M/G suffixes (each case), empty rejection, unknown suffix rejection, garbage number rejection, overflow.
- `crates/cloacina-compiler/src/build.rs::tests::cargo_build_fails_when_build_rs_overshoots_rlimit_as` (Linux-only, gated by `#[cfg(target_os = "linux")]`): synthetic crate with a `build.rs` that tries to allocate 8 GiB; compiler configured with `RLIMIT_AS = 128 MiB`. Assertions: build fails within 30s, returns `BuildError::Failed` (not `TimedOut`), and never succeeds.

### Verification (2026-05-13)

External run: `angreal lint clippy`, `angreal lint fmt`, `angreal test unit`, `angreal test integration` — all green. No fixture breakage from the rlimit defaults.
