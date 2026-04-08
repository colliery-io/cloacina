---
id: security-foundation-authorization
level: initiative
title: "Security Foundation — Authorization, Signature Verification, and Credential Leaks"
short_code: "CLOACI-I-0085"
created_at: 2026-04-08T10:46:46.962392+00:00
updated_at: 2026-04-08T13:34:56.632416+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: security-foundation-authorization
---

# Security Foundation — Authorization, Signature Verification, and Credential Leaks Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 1: Immediate*

## Context

The architecture review identified three Critical security findings (SEC-01, SEC-02, SEC-03) and one Major operability finding (OPS-03) that together represent the highest-priority work. SEC-01 + SEC-02 + SEC-04 form a complete privilege escalation chain: any authenticated user can mint admin keys, enumerate all tenants, and access any tenant's data. SEC-03 means unsigned package uploads allow arbitrary code execution. OPS-03 leaks database credentials in Python binding logs.

## Goals & Non-Goals

**Goals:**
- Close the privilege escalation chain (SEC-01, SEC-02, SEC-04)
- Enforce package signature verification in server mode (SEC-03)
- Fix pipeline completion status to reflect task failures (COR-01)
- Remove credential logging from Python bindings (OPS-03)

**Non-Goals:**
- Full multi-tenant schema isolation redesign (separate initiative)
- TLS support (API Hardening initiative)
- Rate limiting (API Hardening initiative)

## Detailed Design

### REC-01: Lock Down Server Authorization (SEC-01, SEC-02, SEC-04)
**Effort**: 2-3 days

1. In `create_key` (`keys.rs:50`), extract `Extension(auth): Extension<AuthenticatedKey>` and require `auth.can_admin()` before creating keys.
2. In `list_tenants` (`tenants.rs:122`), extract `Extension(auth)` and require `auth.is_admin`.
3. For tenant data isolation, add per-request middleware that sets PostgreSQL `search_path` to the authenticated tenant's schema.
4. Integration tests: read-only key cannot create admin key, tenant-scoped key cannot list other tenants or query other tenant's data.

### REC-02: Enforce Package Signature Verification (SEC-03, SEC-09)
**Effort**: 1-2 days

1. Default to `require_signatures: true` in `cloacinactl serve`.
2. In `upload_workflow` handler, verify package signature before `register_workflow_package()`.
3. Add CLI flags `--require-signatures` / `--no-require-signatures`.
4. Keep daemon defaults permissive for local development.

### REC-03: Fix Pipeline Completion Status (COR-01)
**Effort**: 2-4 hours

In `complete_pipeline()` (`task_scheduler/scheduler_loop.rs`), check task statuses after completion. If any task failed, mark pipeline as "Failed" not "Completed".

### REC-04: Remove Credential Logging (OPS-03)
**Effort**: 1-2 hours

1. Remove all `eprintln!("THREAD: ...")` debug statements from `python/bindings/runner.rs`.
2. Move `mask_db_url()` to a shared utility and apply in Python bindings.

## Implementation Plan

All four items are independent and can be parallelized. Target: 1 week total.
