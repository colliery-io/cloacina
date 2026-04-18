---
id: t9-end-to-end-compiler-reconciler
level: task
title: "T9: End-to-end compiler + reconciler integration tests"
short_code: "CLOACI-T-0527"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-18T01:50:00+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0526]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T9: End-to-end compiler + reconciler integration tests

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Lock the contract: upload → queue → compile → load → execute must work end-to-end across the three binaries. Also pin the error paths (failed build, stale-heartbeat recovery, content-hash artifact reuse).

## Acceptance Criteria

- [ ] `angreal cloacina compiler-e2e` task that:
  1. Builds `cloacina-server`, `cloacina-compiler`, `cloacinactl` (debug).
  2. Spins Postgres via docker-compose.
  3. Starts `cloacina-server` + `cloacina-compiler` as separate subprocesses sharing the DB.
  4. Drives the full flow through `cloacinactl` subprocess only.
- [ ] Happy path: `cloacinactl package publish <fixture>` → poll `package inspect <ID>` until `build_status = success` → `workflow run <NAME>` → `execution status <ID>` reports completed.
- [ ] Failed-build path: upload a fixture with a deliberate `cargo build` error → poll until `build_status = failed` → `build_error` non-empty and contains the stderr tail.
- [ ] Stale-heartbeat path: claim a row via direct DAL call (simulating a dead compiler), wait past `stale_threshold_s`, assert sweeper resets it to `pending` and the real compiler picks it up.
- [ ] Content-hash reuse path: upload package A, wait for success, upload package B with identical content but different name → B's row has `build_status = success` + non-empty `compiled_data` without ever being in `pending`.
- [ ] Composite `cloacinactl status` reports all three services (daemon optional, server + compiler required) and exits 0 only when all reachable ones are healthy.
- [ ] Wired into CI alongside `cli-e2e` from T-0518.

## Implementation Notes

### Harness shape

Fork `.angreal/cloacina/cloacinactl_e2e.py` into `compiler_e2e.py`. Reuse the server fixture, add a second `subprocess.Popen` for the compiler. Cleanup in `finally` SIGTERMs both.

### Fixtures

Two new fixtures in `examples/fixtures/`:
- `compiler-happy-rust` — minimal Rust package that cargo-builds successfully and has one trivial task.
- `compiler-broken-rust` — same structure but with intentional compile error (e.g. typo in an identifier) so builds fail deterministically.

### Stale-heartbeat simulation

Rather than spawning a fake compiler, call `claim_next_build` directly from Python via a small helper binary or via a SQL shortcut: UPDATE a package's row to `building` with a backdated `build_claimed_at`. Then start the real compiler and let the sweeper recover it.

### Flakiness budget

All polls have timeouts (30s each) and structured error messages ("build stuck in pending for 30s — compiler log excerpt: ..."). CI logs capture both binaries' stderr.

## Status Updates

*To be added during implementation*
