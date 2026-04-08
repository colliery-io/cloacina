---
id: architectural-scoped-registries
level: initiative
title: "Architectural — Scoped Registries and DAL Consolidation Investigation"
short_code: "CLOACI-I-0091"
created_at: 2026-04-08T10:46:55.307166+00:00
updated_at: 2026-04-08T10:46:55.307166+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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
- Investigate whether DAL duplication can be consolidated (EVO-03, LEG-03)

**Non-Goals:**
- Removing `#[ctor]` entirely (keep as convenience, add explicit alternative)
- Adding a third database backend
- Full DAL rewrite

## Detailed Design

### REC-21: Scoped Registries via Runtime Struct (EVO-02) — 2-4 weeks, phased

**Phase 1 (1 week)**: Create a `Runtime` struct wrapping `HashMap` registries for tasks, workflows, triggers, CG, and stream backends. Add `Runtime::from_global()` factory that copies current global registries into a scoped instance. `DefaultRunner::with_config()` accepts a `Runtime`. Existing code works via `from_global()`.

**Phase 2 (1 week)**: Update tests to create per-test `Runtime` instances. Remove `#[serial]` from tests that no longer share global state. This immediately enables parallel test execution.

**Phase 3 (future)**: Introduce `RuntimeBuilder` for explicit `runtime.register_task(...)` calls. The `#[ctor]` path remains as convenience for the simple embedded case.

### REC-23: DAL Consolidation Investigation (EVO-03) — Investigation: Days, Implementation: Weeks if feasible

Investigate whether Diesel's `MultiConnection` or generic connection abstractions can write DAL methods once rather than as `_postgres`/`_sqlite` pairs. Pilot on one simple module (`checkpoint.rs`).

If not feasible, apply mitigations:
1. Consolidate on single `dispatch_backend!` macro (remove `backend_dispatch!`, `connection_match!`)
2. Add CI check verifying both backends produce equivalent results
3. Document the architectural constraint

## Implementation Plan

REC-21 is the highest-leverage long-term investment. Start Phase 1 after Phases 1-2 of the roadmap are complete (security + reliability). REC-23 investigation can run in parallel. Target: 3-6 weeks total.
