---
id: t2-a-surface-verification-config
level: task
title: "T2 (A): Surface verification config — flag, env var, startup validation, audit-log wiring"
short_code: "CLOACI-T-0567"
created_at: 2026-05-06T12:50:45.137+00:00
updated_at: 2026-05-07T02:47:03.623463+00:00
parent: CLOACI-I-0103
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0103
---

# T2 (A): Surface verification config — flag, env var, startup validation, audit-log wiring

## Context

Per CLOACI-I-0103 decision D2: expose `verification_org_id` as a server-wide configuration via CLI flag + env var. This task wires the config plumbing only — the actual verification logic ships in T3.

## What to do

- Add `--verification-org-id <UUID>` flag to `cloacina-server` (clap definition under `crates/cloacina-server/`).
- Add `CLOACINA_VERIFICATION_ORG_ID` env var support (clap `env = "..."` attribute).
- At server startup: refuse to start if `--require-signatures` is set without `--verification-org-id`. Clear error message naming both flags.
- The configured value must be reachable from request handlers (via AppState or equivalent).
- Wire imports / AppState plumbing for `audit::log_package_load_success` and `audit::log_package_load_failure`. Actual call sites land in T3, but the plumbing happens here.

## Acceptance

- `cloacina-server --require-signatures` without `--verification-org-id` exits with a non-zero code and a clear error message naming both flags.
- `cloacina-server --require-signatures --verification-org-id <UUID>` starts cleanly.
- The configured `org_id` is reachable from the upload handler (verify via a debug log or test).

## References

- Parent: CLOACI-I-0103 (D2)
- `crates/cloacina/src/security/audit.rs` — audit log functions

## Status Updates

*To be added during implementation*
