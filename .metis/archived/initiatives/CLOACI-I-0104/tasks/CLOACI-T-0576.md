---
id: t-04-compiler-audit-log
level: task
title: "T-04: Compiler audit-log integration via I-0103 facility"
short_code: "CLOACI-T-0576"
created_at: 2026-05-13T12:43:33.606844+00:00
updated_at: 2026-05-13T17:03:21.801606+00:00
parent: CLOACI-I-0104
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0104
---

# T-04: Compiler audit-log integration via I-0103 facility

## Parent Initiative

[[CLOACI-I-0104]]

## Objective

Emit structured audit events for every compiler build through the existing I-0103 audit facility — `compiler_build_started` at claim time and `compiler_build_finished` at exit (success, failure, timeout-kill, rlimit-kill). Payload includes build-claim id, package identity, dep-graph hash for forensic traceability, exit signal/status, wall-clock duration. Operators can reconstruct what was built, when, and how it ended without digging through ephemeral logs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Scoping note (Path B, decided 2026-05-13):** The I-0103 audit facility (`crates/cloacina/src/security/audit.rs`) is **tracing-only** — it emits structured `tracing::info!/warn!/error!` records, not DB rows. There is no `audit_events` table. T-0576 adds compiler events on the same tracing surface, matching the design intent of ADR-0005 ("structured audit events go to the operator's observability pipeline"). DB persistence of audit events, if ever wanted, is a separate cross-cutting initiative that would upgrade all 16 audit kinds at once.

- [ ] Two new event-kind constants in `cloacina::security::audit::events`: `COMPILER_BUILD_STARTED = "compiler.build.started"` and `COMPILER_BUILD_FINISHED = "compiler.build.finished"`. Naming matches the existing dot-notation convention.
- [ ] Two new `pub fn log_compiler_build_started(...)` / `log_compiler_build_finished(...)` in `audit.rs`, emitting via `tracing::info!` with structured fields. Follow the pattern of the existing 14 audit emit functions.
- [ ] `started` fields: `build_claim_id`, `package_name`, `package_version`, `cargo_toml_hash` (SHA-256 hex), `cargo_lock_hash` (Option), `compiler_instance_id`.
- [ ] `finished` fields: all of the above plus `outcome` ∈ {`success`, `failed`, `timeout_killed`, `internal_error`} (rlimit-kill collapses into `failed` for this pass — see Outstanding), `exit_status` (i32 if cargo exited normally), `exit_signal` (string name if signal-terminated), `wall_clock_ms`, `failure_reason` (`<none>` on success).
- [ ] `compiler_instance_id`: `UniversalUuid` generated once in `lib.rs::run` at startup, threaded onto `CompilerConfig`.
- [ ] `cargo_toml_hash` + `cargo_lock_hash`: SHA-256 of file bytes, computed in `run_build` after unpack.
- [ ] Emit sites wired: `started` once at the top of `run_build` after hashes are computed; `finished` once per outcome path before `run_build` returns.
- [ ] Unit tests in `audit.rs::tests` using `with_captured_logs`: at least one each for `_started` and `_finished` covering the field-shape and the `<none>` paths.
- [ ] End-to-end "run_build emits the pair" assertion — **deferred to T-0518 compiler e2e**. The unit-level run_build path needs a `WorkflowRegistryImpl` + DB, which is heavier than the per-fn unit tests above. T-0518's existing live compiler harness exercises run_build for every build and inspecting captured tracing there gives the same coverage with less duplicate plumbing.

## Test Cases

- **TC-1 (success):** happy build emits started→finished pair with `outcome=success`, `exit_status=0`, `failure_reason=null`.
- **TC-2 (timeout kill, depends on T-0573):** build exceeds `--build-timeout-s`, finished event has `outcome=timeout_killed`, `exit_signal=SIGKILL`, `wall_clock_ms ≈ build_timeout_s * 1000`.
- **TC-3 (rlimit kill, depends on T-0575):** build hits RLIMIT_AS, finished event has `outcome=rlimit_killed`, `exit_signal=SIGKILL` (or SIGSEGV, kernel-dependent — accept either, capture which).
- **TC-4 (cargo failure):** build fails because of a compile error in the user code, `outcome=failed`, `exit_status=101` (cargo's standard error exit), `failure_reason` includes the cargo error excerpt.
- **TC-5 (replay/forensics):** after a series of builds, an operator running `SELECT * FROM audit_events WHERE event_kind LIKE 'compiler_%' ORDER BY ts` can reconstruct each build's identity, dep-graph hash, and outcome.

## Implementation Notes

### Technical Approach

- Locate the I-0103 audit facility's public emit surface (likely `audit::log_package_load_success` and friends in `crates/cloacina/src/audit/`). Add two new variants/functions: `log_compiler_build_started(...)`, `log_compiler_build_finished(...)`. Keep the variant naming consistent with existing audit kinds.
- Cargo.toml hash: hash the on-disk `Cargo.toml` content from the unpacked build claim. SHA-256 in hex. Cargo.lock hash: if `Cargo.lock` exists after a successful build, hash it too; null otherwise.
- Exit-status capture: depends on T-0573's child handle. The compiler-side exit-status struct (`std::process::ExitStatus`) carries `.code()` and `.signal()` — translate the signal number to a name (`SIGKILL`, `SIGTERM`, `SIGSEGV`, etc.) for readability. A small mapping table is fine.
- Outcome classification: the compiler already knows *why* it's calling `mark_build_failed` vs `mark_build_success` — same classification produces `outcome`. Don't try to infer outcome from exit status alone; the in-process state machine is the source of truth.
- `compiler_instance_id`: a uuid generated at compiler startup and held in the build loop's context. Doesn't need to persist across restarts.

### Dependencies

- **T-0573** for the timeout-kill outcome and exit-status channel.
- **T-0575** for the rlimit-kill outcome (same channel, different cause).
- T-0574 (offline rejection) classifies as `outcome=failed` with `failure_reason` naming missing crates — no new event-shape changes needed.

### Risk Considerations

- **Audit-table cardinality:** every build emits two rows. High-volume deployments may see audit tables grow fast. Cross-reference the I-0103 retention story; if no retention exists yet, file a follow-up but don't add it here.
- **PII concerns:** `package_name`, `package_version`, `tenant_id` are not PII by Cloacina's data model, but if the audit table is exposed beyond ops, double-check. Out of scope for this task.
- **Schema drift with I-0103:** if the audit facility's emit API expects a fixed enum of event kinds, adding two new kinds may require a schema migration (e.g. dropping a CHECK constraint or extending a Diesel-side enum). Verify against the actual I-0103 implementation before estimating size.

## Status Updates

**2026-05-13** — Path B implementation landed locally; ready for external lint + test pass.

### What changed

- `crates/cloacina/src/security/audit.rs`: two new event-kind constants (`COMPILER_BUILD_STARTED`, `COMPILER_BUILD_FINISHED`) under `events::`. Two new emit functions following the existing 14's pattern (sync, `tracing::info!`, structured fields, no DAL). 5 new `with_captured_logs` unit tests covering the field shape, the `<none>` placeholders for `Option` fields, and three outcome variants (success / timeout_killed / failed).
- `crates/cloacina-compiler/Cargo.toml`: added `sha2 = "0.10"` and `hex = "0.4"` for source-file content hashing.
- `crates/cloacina-compiler/src/config.rs`: added `compiler_instance_id: UniversalUuid` field. `use cloacina::UniversalUuid;` added.
- `crates/cloacina-compiler/src/main.rs`: instance id generated at startup via `cloacina::UniversalUuid::new_v4()` and stamped on the config.
- `crates/cloacina-compiler/src/lib.rs`: instance id logged in the "compiler-starting" `info!` line.
- `crates/cloacina-compiler/src/build.rs`:
  - `BuildError::Failed` refactored from `Failed(String)` to `Failed { reason, exit_status, exit_signal }` so audit emit has the cargo exit info. Added `BuildError::internal(...)` constructor for pre-spawn / non-cargo failures.
  - New `CargoBuildSuccess { artifact, exit_status }` return struct from `cargo_build`.
  - New helpers `sha256_hex_if_present(Path)` and `signal_name(i32)` (unix-only).
  - `run_build` now: tracks `build_started_at`, hashes Cargo.toml + Cargo.lock after unpack, emits `audit::log_compiler_build_started`, runs cargo_build, emits `audit::log_compiler_build_finished` on every outcome path (Success / Failed / TimedOut) with computed `wall_clock_ms`. Single emit-finished site, no duplication.
  - `cargo_build` captures cargo `ExitStatus::code()` and `ExitStatus::signal()` (unix) and threads them through `BuildError::Failed` / `CargoBuildSuccess`.

### Design decisions

- **Path B (tracing-only emit), per design decision earlier today.** No DB writes, no schema migration, no audit_events table. Compiler events flow to the operator's tracing sink (file/journald/Loki/SIEM) exactly like the existing 14 audit kinds. Aligns with ADR-0005.
- **Outcome bucket: 4 values, not 5.** `success`, `failed`, `timeout_killed`, `internal_error`. Rlimit-kills collapse into `failed` for this pass (the exit signal is populated, so operators can still distinguish rlimit kills from clean failures by inspecting `exit_signal=SIGKILL` etc.). A dedicated `rlimit_killed` outcome would require inferring from exit signal alone, which is fragile across kernels. Documented in Outstanding.
- **Single emit-finished site in run_build.** Every exit path from run_build passes through the same `match result { ... }` block. Refactor risk minimized; one place to add a new outcome.
- **Cargo.toml hash always populated, `<absent>` sentinel if missing.** Python packages have no Cargo.toml; we still emit a started event with `cargo_toml_hash = "<absent>"`. Forensic continuity beats schema purity.

### Tests landed

- 5 new unit tests in `crates/cloacina/src/security/audit.rs::tests`: started full-payload, started lockfile-none, finished success (with multi-`<none>` assertion), finished timeout_killed, finished clean failure.
- Existing T-0573 timeout test unchanged; T-0575 rlimit test pattern-match updated for the new `BuildError::Failed { .. }` shape.

### Outstanding

- **`rlimit_killed` outcome.** Currently collapsed into `failed`. To distinguish, we'd need a heuristic on `exit_signal` (`SIGKILL`/`SIGSEGV`/`SIGABRT` likely indicate kernel kill rather than user code panic, but it's not airtight — a user panic can produce SIGABRT too). Practical workaround: operators query the audit stream for `outcome=failed AND exit_signal IN ('SIGKILL','SIGSEGV','SIGABRT')` to filter rlimit-like kills. Documented.
- **End-to-end "run_build emits the pair" assertion.** Deferred to T-0518's existing compiler e2e harness. The unit-level audit fn tests + the audit emit being in a single code path in run_build give sufficient confidence at this layer.

### Verification (2026-05-13)

External run: `angreal lint clippy`, `angreal lint fmt`, `angreal test unit`, `angreal test integration` — all green after a `#[derive(Debug)]` add to `CargoBuildSuccess` (caught by clippy on the rlimit test's `panic!("{other:?}")` formatter).
