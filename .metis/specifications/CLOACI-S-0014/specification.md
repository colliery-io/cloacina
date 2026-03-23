---
id: pipeline-status-derivation-correct
level: specification
title: "Pipeline Status Derivation — Correct Terminal State Detection from Task States"
short_code: "CLOACI-S-0014"
created_at: 2026-03-23T02:23:41.596177+00:00
updated_at: 2026-03-23T02:23:41.596177+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Pipeline Status Derivation — Correct Terminal State Detection from Task States

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

Pipelines are containers — they don't execute, only tasks do. Pipeline status must be correctly derived from task states. Currently `complete_pipeline()` marks pipelines "Completed" without checking if any tasks failed.

## Current Bug

`scheduler_loop.rs::process_pipelines_batch()` calls `check_pipeline_completion()` which checks if all tasks are in terminal states. If true, it calls `complete_pipeline()` which unconditionally sets status="Completed". A pipeline with 3 Completed tasks and 1 Failed task is marked "Completed."

## Correct Derivation

```
IF all tasks in [Completed, Skipped]:
  → pipeline status = "Completed"
ELIF any task in [Failed] with recovery_attempts >= max:
  → pipeline status = "Failed" (with error details listing failed tasks)
ELIF any task still active (NotStarted, Pending, Ready, Running):
  → pipeline status unchanged (still active)
```

## Task Terminal States

| State | Terminal? | Notes |
|-------|----------|-------|
| NotStarted | No | Initial state |
| Pending | No | Dependencies not met |
| Ready | No | Ready for claiming |
| Running | No | Claimed by executor, heartbeating |
| Completed | Yes (success) | Task finished successfully |
| Failed | Yes (failure) | Permanently failed (max retries or abandoned) |
| Skipped | Yes (neutral) | Trigger rule evaluated to skip |

"Abandoned" is not a separate state — it's `Failed` with `error_details LIKE 'ABANDONED:%'`. This is intentional: abandoned tasks are permanently failed, the distinction is in the reason not the state.

## Pipeline Terminal States

| State | Meaning | Derived from |
|-------|---------|-------------|
| Pending | Created, tasks being scheduled | Initial |
| Running | At least one task executing | Has Running tasks |
| Completed | All tasks succeeded or skipped | All tasks Completed/Skipped |
| Failed | At least one task permanently failed | Any task Failed |
| Cancelled | Manually cancelled | User action |
| Paused | Manually paused | User action |

## Fix Required

In `scheduler_loop.rs::process_pipelines_batch()`:

```rust
// After checking all tasks are terminal:
let has_failures = tasks.iter().any(|t| t.status == "Failed");
if has_failures {
    let failed_tasks: Vec<_> = tasks.iter()
        .filter(|t| t.status == "Failed")
        .map(|t| t.task_name.clone())
        .collect();
    dal.pipeline_execution().mark_failed(
        pipeline.id,
        &format!("Tasks failed: {}", failed_tasks.join(", "))
    ).await?;
} else {
    dal.pipeline_execution().mark_completed(pipeline.id).await?;
}
```

## System Context **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Actors
- **{Actor 1}**: {Role and interaction pattern}
- **{Actor 2}**: {Role and interaction pattern}

### External Systems
- **{System 1}**: {Integration description}
- **{System 2}**: {Integration description}

### Boundaries
{What is inside vs outside the system scope}

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | {Requirement description} | {Why this is needed} |
| REQ-1.1.2 | {Requirement description} | {Why this is needed} |
| REQ-1.2.1 | {Requirement description} | {Why this is needed} |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | {Requirement description} | {Why this is needed} |
| NFR-1.1.2 | {Requirement description} | {Why this is needed} |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Decision Area: {Area Name}
- **Context**: {What needs to be decided}
- **Constraints**: {Hard constraints that bound the decision}
- **Required Capabilities**: {What the solution must support}
- **ADR**: {Link to ADR when decision is made, e.g., PROJ-A-0001}

## Decision Log **[CONDITIONAL: Has ADRs]**

{Delete if no architectural decisions have been made yet}

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| {PROJ-A-0001} | {Decision title} | {decided/superseded} | {One-line summary} |

## Constraints **[CONDITIONAL: Has Constraints]**

{Delete if no hard constraints exist}

### Technical Constraints
- {Constraint 1}
- {Constraint 2}

### Organizational Constraints
- {Constraint 1}

### Regulatory Constraints
- {Constraint 1}

## Changelog **[REQUIRED after publication]**

{Track significant changes after initial publication. Delete this section until the specification is published.}

| Date | Change | Rationale |
|------|--------|-----------|
| {YYYY-MM-DD} | {What changed} | {Why it changed} |
