---
id: abac-authz-matcher-core-level
level: task
title: "ABAC authZ matcher core — Level/Scope/Access/Principal + evaluate() + parity truth-table tests"
short_code: "CLOACI-T-0782"
created_at: 2026-06-24T00:41:36.024780+00:00
updated_at: 2026-06-24T01:53:25.992094+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# ABAC authZ matcher core — Level/Scope/Access/Principal + evaluate() + parity truth-table tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement the pure authorization matcher in `cloacina-server` (new `routes/authz.rs`): the `Level` / `Scope` / `Access` / `Principal` / `Decision` types and `evaluate(principal, scope, level) -> Decision`, with **no router wiring yet**. Establish a parity truth-table that proves the matcher reproduces today's `can_access_tenant` / `can_write` / `can_admin` decisions for every `(platform_admin × role × scope × level × tenant-match)` combination — so the later wire-in (T-0783) is provably behavior-preserving.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Types defined: `Level { Read<Write<Admin }`, resolved `Scope { Platform, Tenant(String), Any }`, `Principal { tenant: Option<String>, role: Level, platform_admin: bool }`, `Decision { Permit, Deny(&'static str) }`.
- [ ] `evaluate()` is total + default-deny: `platform_admin` short-circuits to Permit; `Tenant(t)` enforces `tenant==Some(t) || (None && t=="public")` then `role>=level`; `Any` enforces `role>=level`.
- [ ] Deny reasons are exactly `platform_admin_required` / `tenant_access_denied` / `insufficient_role` (map 1:1 to the existing 403 `ApiError` envelopes).
- [ ] Truth-table unit test asserts `evaluate()` matches the current `can_access_tenant`/`can_write`/`can_admin` outcomes across all attribute combinations.
- [ ] `angreal check crate crates/cloacina-server` clean; new unit tests pass.

## Implementation Notes

**Scope:** types + `evaluate()` + unit tests only — no middleware, no route table, no handler edits.
**Depends on:** nothing (leaf; unblocks T-0783).
**References:** I-0118 Detailed Design → "Phase 0 design"; current logic at `crates/cloacina-server/src/routes/auth.rs:245-282`.

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE (commit 172db780).** Implemented `crates/cloacina-server/src/routes/authz.rs`: `Level {Read<Write<Admin}` (+ `from_permissions`), `Scope {Platform, TenantParam, Any}`, `Access` (+ `platform`/`tenant`/`any` ctors), `ResolvedScope {Platform, Tenant(String), Any}`, `Principal {tenant, role, platform_admin}` (+ `from_key`), `Decision {Permit, Deny(&'static str)}` with stable reasons `platform_admin_required`/`tenant_access_denied`/`insufficient_role`, and the total default-deny `evaluate(&Principal, &ResolvedScope, Level)`. Module declared in `routes/mod.rs`; `#![allow(dead_code)]` until T-0783 mounts it. Parity truth-table tests reconstruct today's `can_access_tenant`/`can_write`/`can_admin` (auth.rs:245-282) and assert `evaluate()` matches for every `{is_admin × role × tenant} × {scope × level}` combo. `angreal check crate crates/cloacina-server` clean (only pre-existing `cloacina`-dep warnings); 7/7 unit tests pass. All ACs met.