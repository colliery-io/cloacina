---
id: naming-and-structure-pipeline-to
level: initiative
title: "Naming and Structure — Pipeline-to-Workflow Rename and Python Crate Extraction"
short_code: "CLOACI-I-0090"
created_at: 2026-04-08T10:46:54.008177+00:00
updated_at: 2026-04-08T10:46:54.008177+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: naming-and-structure-pipeline-to
---

# Naming and Structure — Pipeline-to-Workflow Rename and Python Crate Extraction Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 6: Structural*

## Context

The Pipeline/Workflow terminology split is the most frequently cited legibility issue, affecting four review lenses (LEG-01, API-01, COR-01 context, OPS visibility). Users define "Workflows" but receive `PipelineResult`, `PipelineStatus`, `PipelineError`. The Python bindings crate (2,888-line `runner.rs`) reimplements coordination logic rather than wrapping the Rust API, causing drift (OPS-03, API-08).

**Dependency**: REC-03 (fix pipeline completion status in I-0085) must be done first to avoid renaming broken code.

## Goals & Non-Goals

**Goals:**
- Rename execution-layer types from Pipeline to Workflow (LEG-01, API-01)
- Clarify scheduler module naming collision (LEG-02): `scheduler.rs` vs `task_scheduler/`
- Extract Python bindings into separate `cloacina-python` crate (EVO-01, EVO-05)

**Non-Goals:**
- Database table renames (keep `pipeline_executions` for migration compat)
- Python bindings rewrite (extraction only, behavior preserved)

## Detailed Design

### REC-16: Pipeline-to-Workflow Rename (LEG-01, LEG-02, API-01) — 1-2 weeks, phased

**Phase A**: Add type aliases (`type WorkflowResult = PipelineResult`, etc.) with deprecation attributes. Update prelude to export new names.

**Phase B**: Migrate internal usages to new names. Rename modules:
- `scheduler.rs` -> `cron_trigger_scheduler.rs`
- `task_scheduler/` -> `execution_planner/`

**Phase C**: Remove deprecated aliases.

High-churn, low-risk refactoring. Run full test suite after each phase.

### REC-22: Python Crate Extraction (EVO-01, EVO-05) — 1-2 weeks

Move `src/python/` to new `crates/cloacina-python/` with crate-type `["cdylib"]`. Core `cloacina` becomes `["lib"]` only, eliminating PyO3 dep for Rust consumers. Maturin build targets the new crate. Mechanical restructuring, no behavior change.

## Implementation Plan

REC-16 first (depends on I-0085 REC-03 being complete). REC-22 can run in parallel with Phase B/C of the rename. Target: 2-4 weeks.
