---
id: recovery-sweeper-service-periodic
level: specification
title: "Recovery Sweeper Service — Periodic Orphan Detection and Task Re-queuing"
short_code: "CLOACI-S-0013"
created_at: 2026-03-23T02:23:40.436751+00:00
updated_at: 2026-03-23T02:23:40.436751+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Recovery Sweeper Service — Periodic Orphan Detection and Task Re-queuing

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

Background service that periodically scans for orphaned tasks (stale heartbeats) and resets them for re-execution. Replaces the dead `RecoveryManager` code which only ran once at startup.

## Sweeper Loop

Runs every 30s (configurable):

```
1. Query: SELECT * FROM task_executions WHERE status='Running' AND heartbeat_at < NOW() - 60s
2. For each orphaned task:
   a. If recovery_attempts >= max_recovery_attempts (default 3):
      → mark_failed("Abandoned after N recovery attempts")
   b. Else:
      → UPDATE status='Ready', claimed_by=NULL, heartbeat_at=NULL, recovery_attempts+=1
      → INSERT INTO task_outbox (task_execution_id, created_at) — re-queues for dispatch
      → INSERT INTO recovery_events — audit trail
3. Log: "Recovered N tasks, abandoned M tasks"
```

## Idempotency

Recovery is idempotent because:
- The `WHERE status='Running'` filter only matches tasks not yet recovered
- Once reset to Ready, the task won't match on the next sweep
- The outbox insert uses the task's existing ID (no duplicate pipelines created)

## Integration with Cron Recovery

The Recovery Sweeper and `CronRecoveryService` handle different layers:
- **CronRecoveryService**: Detects cron schedules that were claimed but never produced a pipeline (schedule → pipeline handoff gap)
- **Recovery Sweeper**: Detects tasks within a pipeline that were claimed by an executor but never completed (task execution gap)

They are complementary and don't overlap. Both should run.

## Startup Recovery

On startup, the sweeper runs immediately (not waiting for the first interval). This handles the common case: daemon/server crashed, restarted, orphaned tasks from previous session need immediate recovery.

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `recovery_sweep_interval` | 30s | How often to scan for orphans |
| `recovery_orphan_threshold` | 60s | Heartbeat age to consider orphaned |
| `recovery_max_attempts` | 3 | Max recovery attempts before abandoning |
| `enable_recovery_sweeper` | true | Enable/disable the sweeper |

## What This Replaces

- **Dead code**: `task_scheduler/recovery.rs::RecoveryManager` (runs once at startup, never again)
- **Missing functionality**: Continuous orphan detection during operation
- **Missing functionality**: Outbox re-insertion on recovery (current `reset_task_for_recovery` doesn't do this)

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
