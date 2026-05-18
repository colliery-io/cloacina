---
id: reconciler-did-existence-check-for
level: task
title: "Reconciler DiD existence check for signed packages (defense-in-depth)"
short_code: "CLOACI-T-0571"
created_at: 2026-05-07T03:50:15.943463+00:00
updated_at: 2026-05-15T13:05:10.206466+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Reconciler DiD existence check for signed packages (defense-in-depth)

## Objective

Add a defense-in-depth signature existence check at the reconciler load path so that, when `--require-signatures` is on, a `workflow_packages` row written directly to the DB (bypassing the upload route) cannot be loaded if it has no companion `package_signatures` row.

Spun out of CLOACI-I-0103 / CLOACI-T-0568 — the upload-route verification gate (T-0557 + T-0568 audit-log wiring) is the strong gate. This DiD adds a second check at the reconciler. Deferred from the I-0103 close-out by user direction.

## Type
- [x] Tech Debt — defense-in-depth hardening

## Priority
- [ ] P3 — Low. The threat window is narrow: any attacker with DB-write access can also insert a fake `package_signatures` row to bypass this check, so the realistic adversary is "DB-write attacker who doesn't know about the signatures table." Picks up when threat-model assumptions tighten.

## Technical Debt Impact

- **Current problem**: the upload route is the only check enforcing signature presence when `--require-signatures` is on. A direct `INSERT INTO workflow_packages ...` against the DB will be picked up by the reconciler and loaded with no verification.
- **Benefits of fixing**: closes the bypass for the narrow case described above; matches the documented "verify in the upload path + DiD at reconciler" contract from CLOACI-I-0103 D1.
- **Risk of not fixing**: low; documented in the I-0103 close-out as accepted risk.

## What to do

Plumbing-heavy task spanning `cloacina` engine + `cloacina-server` config:

1. **`ReconcilerConfig`** (`crates/cloacina/src/registry/reconciler/mod.rs`) — add `require_signatures: bool` and `verification_org_id: Option<UniversalUuid>` fields with sane defaults (false / None).
2. **`RegistryReconciler`** — accept a `Database` (or DAL handle) at construction so `load_package` can run the existence query. Two options:
   - Add `database: Option<Database>` field with a `with_database(db)` builder.
   - Or extend the `WorkflowRegistry` trait with an `async fn find_signature(&self, package_hash: &str) -> Option<...>` method that the registry impls forward to the underlying DAL.
   - Recommend the trait-method approach — keeps the reconciler from holding two handles to DB-shaped things.
3. **`load_package` in `loading.rs`** — at the top, after fetching `loaded_workflow.package_data`, when `self.config.require_signatures` is true:
   - Compute `package_hash = sha256(package_data)`.
   - Query for an existing `package_signatures` row.
   - On no row: return `RegistryError::RegistrationFailed { message: "no signature row present for this package; refusing to load (require_signatures=true)" }`. Log a warning and call `audit::log_package_load_failure` with the org_id + a clear `failure_reason`.
4. **`DefaultRunnerConfig`** (`crates/cloacina/src/runner/default_runner/config.rs`) — add `require_signatures: bool` and `verification_org_id: Option<UniversalUuid>` fields + getters + builder methods. Forward to `ReconcilerConfig` at `services.rs:181`.
5. **`cloacina-server`** (`lib.rs`) — when constructing `DefaultRunnerConfig`, set the new fields from the same `require_signatures` + `verification_org_id` already passed to `run()`. Already plumbed end-to-end via T-0567 — just needs forwarding.

## Acceptance

- [ ] When `require_signatures = true` and a `workflow_packages` row exists with no matching `package_signatures` row, the reconciler refuses to load it (logs a warning, calls `audit::log_package_load_failure`, marks the package row as failed/quarantined per the existing failure pattern).
- [ ] When `require_signatures = false`, behavior is unchanged.
- [ ] Regression test: write a `workflow_packages` row directly via DAL (no `package_signatures`), trigger the reconciler, assert the package isn't loaded and the failure is audited. (Pairs with the deferred portion of CLOACI-T-0570.)
- [ ] `angreal lint clippy` clean.
- [ ] `angreal test integration` green on both backends.

## Dependencies

- CLOACI-T-0567 (verification config plumbing) — done; `verification_org_id` already reaches `SecurityConfig`.
- CLOACI-T-0566 (org_id column) — done; query target exists.

## References

- Parent of origin: CLOACI-I-0103 (active → closed in same close-out)
- Predecessor: CLOACI-T-0568 status update, "Deferred — reconciler DiD existence check needs scope decision"
- Predecessor: CLOACI-T-0570 status update, "Deferred — DiD regression test"
- ADR: CLOACI-A-0005 (deployment-mode trust model)
- Code anchors: `crates/cloacina/src/registry/reconciler/loading.rs` (load_package), `crates/cloacina/src/runner/default_runner/services.rs:181` (reconciler construction)

## Status Updates

*To be added when picked up*