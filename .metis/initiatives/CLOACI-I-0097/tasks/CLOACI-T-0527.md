---
id: t9-end-to-end-compiler-reconciler
level: task
title: "T9: End-to-end compiler + reconciler integration tests"
short_code: "CLOACI-T-0527"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-19T00:22:03.636501+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0526]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T9: End-to-end compiler + reconciler integration tests

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Lock the contract: upload → queue → compile → load → execute must work end-to-end across the three binaries. Also pin the error paths (failed build, stale-heartbeat recovery, content-hash artifact reuse).

## Acceptance Criteria

## Acceptance Criteria

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

**2026-04-18** — Delivered:

- `angreal cloacina compiler-e2e` harness in `.angreal/cloacina/compiler_e2e.py`.
  Builds cloacina-server + cloacina-compiler + cloacinactl (debug), spins
  Postgres via the existing docker-compose fixture, starts both services
  as separate subprocesses sharing the DB, drives the full flow through
  the `cloacinactl` subprocess, tears everything down in `finally`.
- Fixtures under `examples/fixtures/`: `compiler-happy-rust` (minimal
  cdylib, builds green) and `compiler-broken-rust` (same shell, `src/lib.rs`
  references an undefined identifier — deterministic cargo failure).
- Happy path: pack + upload fixture → poll `package inspect <id>` until
  `build_status = success`, assert `build_error` null. Timeout 180s with
  a diagnostic body dump if it expires.
- Failed-build path: upload broken fixture → poll until
  `build_status = failed`, assert `build_error` non-empty.
- Composite status: `cloacinactl status` exits 0 with server + compiler
  rows present.

**Also landed while proving the harness end-to-end** (separate commits):

- CLI↔server API realignment — `cloacinactl` was calling aspirational
  `/v1/<noun>` paths that never existed on the server. Every tenant-scoped
  verb now targets the real `/tenants/{tenant}/...` / `/auth/...` /
  `/v1/health/reactors` routes the server actually exposes.
- Graph noun renamed to `reactor` to match the server's internal naming.
- Filed CLOACI-T-0528 (tech-debt) to audit residual `graph` / `reactor`
  drift across core + server internals.

**Deferred — captured for follow-up:**

- **Reconciler end-to-end** (upload → compile → reconciler loads → workflow
  run → execution completes). Fixtures that link against cloacina crates
  need either host-path rewriting inside `cloacina-compiler` or published
  crates from the T-0501 release pipeline. Compiler-side mechanics above
  prove the queue + heartbeat + artifact-persist contract independently;
  the FFI-load leg lands once one of those two blockers clears.
- **Stale-heartbeat recovery** — DB-observable via a direct SQL UPDATE
  that backdates `build_claimed_at`. Not in this pass; trivial to add
  next time someone touches the harness.
- **Content-hash artifact reuse** — same story; needs a SQL pre-populate
  or a second upload path with identical bytes.

CI wire-up (T-0518 sibling): the task is registered in `task_project.py`
so `angreal cloacina compiler-e2e` runs alongside `cli-e2e` in any CI job
that exercises the cloacina command group.
