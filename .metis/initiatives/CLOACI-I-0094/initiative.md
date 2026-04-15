---
id: complete-pipeline-to-workflow
level: initiative
title: "Complete pipeline-to-workflow terminology migration"
short_code: "CLOACI-I-0094"
created_at: 2026-04-11T14:01:08.184572+00:00
updated_at: 2026-04-14T00:59:26.622543+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: complete-pipeline-to-workflow
---

# Complete pipeline-to-workflow terminology migration Initiative

## Context

The codebase underwent a conceptual rename from "pipeline" to "workflow" at the public API layer but did not propagate it to the database schema, internal models, error messages, or Python bindings. There are 539+ occurrences of `pipeline_exec` in the data layer. DB tables are `pipeline_executions`, Python config exposes `pipeline_timeout_seconds`, error messages say "Pipeline execution failed." This is the most pervasive single issue identified in the architecture review (LEG-001, API-002, API-008, cross-cutting CLF-01).

## Review Finding References

LEG-001, API-002, API-008, CLF-01 (from architecture review `review/10-recommendations.md` REC-007)

## Goals & Non-Goals

**Goals:**
- Eliminate all "pipeline" references in favor of "workflow" across code, DB, Python bindings, error messages, and tests
- Provide backward-compatible DB migration path
- Ensure no consumer-facing breakage

**Non-Goals:**
- Changing the conceptual model (pipelines and workflows are the same thing, just renamed)
- Touching the computation graph system (separate concept)

## Detailed Design

### Phase 1: Low-risk, high-impact (no DB changes)
- Rename Python config property `pipeline_timeout_seconds` → `workflow_timeout_seconds`
- Update all error message strings from "Pipeline" to "Workflow"
- Rename `pipeline_executor.rs` → `workflow_executor.rs`
- Rename internal variables: `pipeline_execution_id` → `workflow_execution_id`, etc.

### Phase 2: Database migration
- Create migration renaming `pipeline_executions` → `workflow_executions`
- Rename columns: `pipeline_name` → `workflow_name`, `pipeline_version` → `workflow_version`
- Create backward-compat view aliasing old table name during rollout
- Update Diesel schema file and all DAL code

### Phase 3: Cleanup
- Rename test functions from `test_pipeline_*` → `test_workflow_*`
- Case-insensitive search for remaining `pipeline` references
- Remove backward-compat view after one release cycle

## Alternatives Considered

- **Leave as-is**: Rejected — the naming collision increases incident diagnosis time and confuses every new contributor
- **Alias only (views/type aliases)**: Rejected — adds a layer of indirection without resolving the root cause

## Implementation Plan

Phase 1 → Phase 2 → Phase 3, sequentially. CLOACI-T-0474 (double state-update fix) should be done first since it touches the same executor/dispatcher code paths.
