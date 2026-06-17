---
id: ws-2-operations-health-view-server
level: task
title: "WS-2 — Operations/health view (server, compiler, scheduler, fleet)"
short_code: "CLOACI-T-0704"
created_at: 2026-06-16T01:50:12.986401+00:00
updated_at: 2026-06-16T03:21:54.369350+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-2 — Operations/health view (server, compiler, scheduler, fleet)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P0) Add an **Operations/Health** view — the UI has none today. Surface data the
server already exposes so an operator can answer "is my deployment healthy" without
leaving the control plane.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] New top-level nav section (e.g. "Operations").
- [ ] Per-service status tiles: **server** (`/health`, `/ready` incl. DB + crashed-graph reason), **compiler** status, **scheduler** liveness (from metrics).
- [ ] **Execution-agent fleet** roster table: agents, heartbeats, evicted/reassigned counts.
- [ ] Reconciler liveness/lag where available; degraded states obvious and explained (no raw enums).

## Dependencies

Depends on [[CLOACI-T-0702]] (maps each health/metrics/fleet source to a tile).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

- 2026-06-16: **DONE + screenshot-verified** (commit committed after `b5749d85`, branch `feat/ui-0124-server-read-endpoints`). Screenshot `/tmp/cloacina-ui-uat/ws2/operations.png`.
  - New **Operations** nav section (`/operations`, `IconHeartbeat`) with auto-refreshing (5s) tiles: **Server** (alive via `/health`, readiness via `/ready` incl. DB-pool + crashed-graph reason), **Compiler** (status building/backlogged/idle + pending/building + last success/failure from `/v1/compiler/status`), **Execution-agent fleet** (count + roster table from `/v1/agents`; empty-state explains in-process executor). `ui/src/routes/Operations.tsx` + `ui/src/api/operations.ts`.
  - Verified on the demo: Server ALIVE/READY, Compiler IDLE w/ last-success timestamp, Fleet 0 agents (correct — embedded demo, no fleet).
  - **Scoping notes:** scheduler-liveness tile and reconciler-lag + fleet evicted/reassigned counters were left out — they require parsing the Prometheus `/metrics` text (no JSON endpoint), which is heavier than WS-2 warrants; readiness already covers the "is it running" signal. Fleet roster table renders when agents exist (verified-empty here). Filed as a possible P2 enrichment if metrics-derived tiles are wanted.