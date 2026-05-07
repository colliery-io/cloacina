---
id: wire-signature-verification-on-the
level: initiative
title: "Wire signature verification on the server upload + load path (opt-in)"
short_code: "CLOACI-I-0103"
created_at: 2026-05-06T11:05:29.968756+00:00
updated_at: 2026-05-07T03:52:04.084137+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: wire-signature-verification-on-the
---

# Wire signature verification on the server upload + load path (opt-in) Initiative

## Context

The system has comprehensive package-signing infrastructure — Ed25519 signing/verification primitives in `crates/cloacina/src/security/package_signer.rs`, `package_signers` and `package_signatures` DAL tables, and an `audit::log_package_load_*` API in `crates/cloacina/src/security/audit.rs:188,207` — but the trust gate is unwired in the default deployment.

Specific gaps surfaced by the May 2026 review:

1. **Configuration plumbing terminates short of the verifier.** `cloacina-server --require-signatures` cannot be activated because the verifier's `verification_org_id` has no CLI flag or env var. The flag is accepted but the precondition silently fails-closed against any non-empty signed payload.
2. **Audit logging is dead code in production.** `audit::log_package_load_success` and `audit::log_package_load_failure` are defined but never called from the load path.
3. **Trust-chain tests don't run by default.** The six integration tests under `crates/cloacina/tests/integration/signing/` are gated by `#[ignore = "Requires database connection"]` so they don't run in `angreal test`.

Combined effect: the documented defense exists in code but cannot be turned on through the documented config surface, and the only tests of the path do not run by default.

**Scope** (per CLOACI-A-0005): this initiative covers the **server** upload + load path only. The daemon is high-trust by design; daemon-side verification is out of scope. Verification is **opt-in via configuration** — default server behavior accepts unsigned uploads from authorized keys; operators choose their own posture.

## Goals & Non-Goals

**Goals:**
- `cloacina-server --require-signatures` + `--verification-org-id <UUID>` becomes a usable, fail-closed pair of flags.
- When configured, verification runs at the canonical server load path so every register-package code path enforces it (upload route, reconciler, future ingest paths).
- Successful and failed verifications are recorded in the audit log when verification is enabled.
- Trust-chain integration tests run by default under `angreal test integration`, with clean auto-skip when no DB fixture is present.
- `package_signatures` rows carry an `org_id` scope, on both backends.
- Default server behavior is unchanged: unsigned uploads from authorized keys still accepted unless the operator opts in.

**Non-Goals:**
- Daemon-side verification. Daemon is high-trust by design (CLOACI-A-0005).
- Default-on signature enforcement. Verification is opt-in; operators pick their posture.
- Interim "admin-only upload" restriction or other paternalistic mitigations. Operators who care can require sigs or restrict key issuance themselves.
- Sandboxing of `cargo build` at the compiler service. Tracked under CLOACI-I-0104 / CLOACI-I-0105.
- Designing a new verification protocol or key format. Use the existing Ed25519 path.
- Multi-org trust hierarchies; cross-org signing chains. Defer.

## Source Findings (May 2026 review)

- **SEC-01 (Critical)** — Plugin loading is in-process arbitrary code execution; signature verification is unwired on default-config load.
- **SEC-04 (Critical-adjusted)** — `verification_org_id` has no CLI/env knob.
- **COR-15 (Critical-adjusted)** — Six trust-chain integration tests `#[ignore]`'d.
- **SEC-13 (Minor)** — `package_signatures` has no `org_id` scope.
- Audit gap — `audit::log_package_load_success`/`log_package_load_failure` defined but unreferenced from the load path.

## Decisions (locked during discovery review)

- **D1 — Verification chokepoint**: verification happens in the **upload route** of `cloacina-server`. Source-archive signature is validated at upload time; on failure (with `--require-signatures` set), the upload is rejected with a 403 before any bytes are stored. Compiler service and reconciler trust the upload-side gate.
  - **Defense-in-depth**: at reconciler load time, a cheap SQL existence check refuses to load a package whose `package_id` has no corresponding `package_signatures` row when `require_signatures` is on. Guards against direct DB writes that bypass the upload route.

- **D2 — `verification_org_id` shape**: server-wide today (one trusted org per server, configured via `--verification-org-id <UUID>` / `CLOACINA_VERIFICATION_ORG_ID`). The config schema is designed so per-tenant overrides are an *additive* future change. Per-tenant trust is the longer-term direction for SaaS-style multi-org deployments; server-wide fits the "platform / enterprise teams" audience where tenants are organizationally related, and is gated by I-0106's multi-tenant maturation.

- **D3 — Bootstrap admin bypass**: **no bypass.** When `--require-signatures` is on, the requirement applies uniformly — bootstrap admin signs like everyone else. Emergency overrides happen at the operator level (temporarily disable the flag), not the code level.

- **D4 — `org_id` migration**: non-destructive `ALTER TABLE package_signatures ADD COLUMN org_id <T>` on both backends. Existing rows get NULL → won't pass verification when `--require-signatures` is on. Document the re-sign requirement as part of the upgrade path. Pre-1.0 status accepted — no backward-compat hedging.

## Initial Sketch

(Subject to refinement during design phase. Decisions in §"Decisions" are locked.)

- **Phase A — surfacing**: add `--verification-org-id <UUID>` and `CLOACINA_VERIFICATION_ORG_ID` to `cloacina-server`. Refuse to start when `--require-signatures` is set without `--verification-org-id`. Wire `audit::log_package_load_*` on both success and failure when verification is configured.
- **Phase B — verification in upload + DiD at reconciler (per D1)**: implement source-archive signature verification inside the `cloacina-server` upload route handler — verify against `verification_org_id` when `--require-signatures` is set; reject with 403 + audit log on failure. Add a cheap SQL existence check to `RegistryReconciler::load_package` that refuses to load a package whose `package_id` has no corresponding `package_signatures` row when `require_signatures` is on.
- **Phase C — test gating**: switch the six signing tests from `#[ignore]` to the `get_all_fixtures()` auto-skip pattern other integration tests use.
- **Phase D — schema scoping**: add `org_id` to `package_signatures` via non-destructive `ALTER TABLE ADD COLUMN` on both backends (per D4). Document the re-sign requirement for upgraders.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `cloacina-server --require-signatures` without `--verification-org-id` exits with a clear error.
- With `--require-signatures` set: uploading an unsigned package fails with a 403 and the failure is logged via `audit::log_package_load_failure`.
- Without `--require-signatures`: default behavior is unchanged — unsigned uploads from authorized keys are accepted (no regression for existing operators).
- All six signing trust-chain tests pass in `angreal test integration` with no `#[ignore]` annotations.
- A new contract test exercises sign → upload → verify → load against a fixture-trusted key.
- With `--require-signatures` on: a regression test that injects a bare `workflow_packages` row directly into the DB (no `package_signatures` companion) confirms the reconciler refuses to load it (DiD per D1).

## References

- ADR: CLOACI-A-0005 (deployment-mode trust model — defines server-only scoping; sig verification is opt-in)
- `review/07-security.md` — SEC-01, SEC-04, SEC-13
- `review/02-correctness.md` — COR-15
- `review/10-recommendations.md` — REC-01
- Existing infrastructure: `crates/cloacina/src/security/audit.rs:188,207`, `crates/cloacina/src/security/package_signer.rs`
- Prior task: CLOACI-T-0440 (server-mode signature enforcement, completed)
- Prior task: CLOACI-T-0475 (signature verification in upload handler, completed)
