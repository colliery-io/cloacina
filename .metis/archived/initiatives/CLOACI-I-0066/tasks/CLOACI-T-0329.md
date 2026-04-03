---
id: write-new-explanation-docs-2-docs
level: task
title: "Write new Explanation docs (2 docs)"
short_code: "CLOACI-T-0329"
created_at: 2026-04-02T22:51:45.157188+00:00
updated_at: 2026-04-02T23:39:07.318084+00:00
parent: CLOACI-I-0066
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0066
---

# Write new Explanation docs (2 docs)

## Parent Initiative
[[CLOACI-I-0066]]

## Objective
Write 2 new Explanation docs covering undocumented architectural concepts.

## Documents to Write

### 1. explanation/architecture-overview.md
- Three deployment modes: embedded library, daemon, API server
- Component map: Runner, Scheduler, Executor, Dispatcher, Registry, DAL
- Data flow: workflow definition → scheduling → execution → persistence
- How components compose differently per deployment mode
- Mermaid diagrams for each mode
- Source: crates/cloacina/src/lib.rs, runner/, scheduler.rs, crates/cloacinactl/

### 2. explanation/horizontal-scaling.md
- Task claiming: atomic database locking for distributed execution
- Heartbeat mechanism: interval, stale detection
- Stale claim sweeping: threshold, automatic reassignment
- Runner identification: runner_id, runner_name
- Routing configuration: pattern-based task dispatch
- How multiple runners share a PostgreSQL database
- Failure scenarios and recovery
- Source: crates/cloacina/src/runner/default_runner/config.rs (claiming fields), task_scheduler/stale_claim_sweeper.rs, dispatcher/

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Both docs complete with no placeholders
- [ ] Mermaid diagrams for key concepts
- [ ] Cross-links to configuration reference and how-to guides

## Status Updates
*To be added during implementation*
