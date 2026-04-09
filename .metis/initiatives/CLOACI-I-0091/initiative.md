---
id: architectural-scoped-registries
level: initiative
title: "Architectural — Scoped Registries and DAL Consolidation Investigation"
short_code: "CLOACI-I-0091"
created_at: 2026-04-08T10:46:55.307166+00:00
updated_at: 2026-04-09T16:59:22.486908+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: architectural-scoped-registries
---

# Architectural — Scoped Registries and DAL Consolidation Investigation Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 7: Architectural*

## Context

The architecture review identified two root causes (RC-01, RC-03) that are the highest-leverage but highest-cost improvements. Process-global static registries (RC-01) drive 160 `#[serial]` test annotations, prevent crate decomposition, and block multi-instance isolation. The dual-backend DAL duplication (RC-03) doubles every schema change but is an inherent cost of the Diesel MultiConnection strategy.

## Goals & Non-Goals

**Goals:**
- Introduce scoped registries via a `Runtime` struct (EVO-02, EVO-10, PERF-11)
- Enable parallel test execution by removing need for `#[serial]`

**Non-Goals:**
- Removing `#[ctor]` entirely (keep as convenience, add explicit alternative)
- DAL consolidation (deferred to separate initiative — known Diesel constraint, see ADR-0001)

## Detailed Design

### REC-21: Scoped Registries via Runtime Struct (EVO-02) — 2-4 weeks, phased

**Phase 1 (1 week)**: Create a `Runtime` struct wrapping `HashMap` registries for tasks, workflows, triggers, CG, and stream backends. Add `Runtime::from_global()` factory that copies current global registries into a scoped instance. `DefaultRunner::with_config()` accepts a `Runtime`. Existing code works via `from_global()`.

**Phase 2 (1 week)**: Update tests to create per-test `Runtime` instances. Remove `#[serial]` from tests that no longer share global state. This immediately enables parallel test execution.

**Phase 3 (future)**: Introduce `RuntimeBuilder` for explicit `runtime.register_task(...)` calls. The `#[ctor]` path remains as convenience for the simple embedded case.

### REC-23: DAL Consolidation — DEFERRED (known Diesel constraint)

**Status**: Investigated and deferred. Diesel's `MultiConnection` does not support generic connection abstractions that would allow writing DAL methods once. This is an inherent limitation of the Diesel ORM design (documented in ADR-0001). The dual-backend `_postgres`/`_sqlite` pattern is the necessary cost of runtime backend selection.

**Mitigations already applied (I-0089)**:
1. Consolidated on single `dispatch_backend!` macro (removed `backend_dispatch!` and `connection_match!`)

**Remaining mitigations (backlog)**:
- CI equivalence check verifying both backends produce identical results for a standard test suite
- This initiative stays on the backlog for future reconsideration if Diesel adds generic connection support

## Implementation Plan

REC-21 is the highest-leverage long-term investment. Start Phase 1 after Phases 1-2 of the roadmap are complete (security + reliability). REC-23 investigation can run in parallel. Target: 3-6 weeks total.
