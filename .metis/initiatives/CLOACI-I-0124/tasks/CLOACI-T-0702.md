---
id: ws-0-server-data-audit-spike
level: task
title: "WS-0 — Server-data audit spike (execution / ops / node-code surfaces)"
short_code: "CLOACI-T-0702"
created_at: 2026-06-16T01:50:10.269078+00:00
updated_at: 2026-06-16T02:03:52.687134+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-0 — Server-data audit spike (execution / ops / node-code surfaces)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

**Gating spike** — before any UI work, inventory what the server already exposes for
the three data-hungry surfaces, so each UI task knows what's "available now" vs. a
scoped server dependency (initiative non-goal: no new server capabilities assumed).
Audit source: `/tmp/cloacina-ui-uat/ux-report.md`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] **Execution drill-down:** determine whether per-task rows (status, start/end, attempts, output/context, error/reason) are derivable from the existing execution + events API; if not, specify the missing endpoint + response shape.
- [ ] **Ops/health:** map `/health`, `/ready`, `/metrics`, the fleet agent roster + heartbeat endpoints, and compiler status to concrete UI tiles/tables; note any gap.
- [ ] **Node code/IO:** determine whether node/task source + inputs/outputs/retry/routing metadata is retrievable (package introspection or API); flag gaps.
- [ ] Deliverable: a per-workstream "available now vs. server dependency (proposed endpoint)" note recorded here and linked from [[CLOACI-I-0124]].

## Dependencies

Blocks [[CLOACI-T-0703]] (execution drill-down), [[CLOACI-T-0704]] (ops health),
[[CLOACI-T-0707]] (node drawer).

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

## Findings — server-data audit (2026-06-16)

Format: **available now** vs **server dependency (proposed endpoint)**. Verdict: the
underlying data largely exists; the gaps are a few thin **read** endpoints (no new
engine capability), plus one hard constraint (node source).

### Surface 1 — Execution drill-down ([[CLOACI-T-0703]])
- **Available now:** `GET /v1/tenants/{t}/executions` (list), `GET …/executions/{id}`
  (returns **status only** — `executions.rs:306` `ExecutionDetail{tenant_id,execution_id,status}`),
  `GET …/executions/{id}/events` (the flat event log the UI shows; payloads empty).
- **Data exists in DB:** `task_execution` model (`crates/cloacina/src/models/task_execution.rs:31-43`)
  carries `status, started_at, completed_at, attempt, max_attempts, error_details,
  last_error, recovery_attempts, sub_status`. DAL: `dal.task_execution()` +
  `dal.task_execution_metadata()` (output/context) + `execution_event.list_by_task`.
- **GAP → server dependency:** add `GET /v1/tenants/{t}/executions/{id}/tasks`
  returning the per-task rows + metadata (output/context). Thin additive endpoint
  over existing DAL — **no new capability**.
- **Verdict:** WS-1's task table / status-colored DAG / durations can ship as soon as
  this endpoint lands; the per-task **output/context/logs** drawer needs the metadata
  included in it. WS-1 is **UI + 1 server endpoint**.

### Surface 2 — Ops/health ([[CLOACI-T-0704]])
- **Available now:** `/health`, `/ready` (DB pool + crashed-graph reason),
  `/metrics` (scheduler + fleet counters incl. `cloacina_fleet_agents_evicted_total`,
  `cloacina_fleet_work_reassigned_total`). → server + scheduler tiles ship now.
- **GAP → server dependency (×2):**
  1. **Fleet roster** is **in-memory only** (`cloacina-server/src/lib.rs:203`); the
     `/v1/agent/*` routes are agent-facing (register/heartbeat/result/artifact) — there
     is **no operator GET to list agents**. Add `GET /v1/agents` (or `/v1/fleet`)
     exposing roster: agent id, target triple, capacity, last heartbeat, state.
  2. **Compiler status**: no server endpoint; the compiler is a separate service
     (:9000) and only per-package `build_status` exists. Either the UI queries the
     compiler's own status, or add a server proxy/health-aggregate.
- **Verdict:** WS-2 ships server/scheduler tiles immediately; **fleet roster + compiler
  status are the two endpoints to add**.

### Surface 3 — Node code/IO ([[CLOACI-T-0707]])
- **Available now:** tasks + **dependencies** (workflows API, `workflows.rs:263/340`),
  graph **topology/routing** edges (graph-detail API), retry policy in task metadata.
  → node IO / deps / retry / routing are all available.
- **GAP / hard constraint:** **node source code is NOT in the package** — packages are
  compiled cdylibs + manifest; no source ships. A literal "view the code" tab would
  require shipping source in the package or a separate source store (maybe feasible
  for Python packages; not for compiled Rust).
- **Recommendation:** scope WS-5 to the available metadata (signatures, IO, deps,
  retry, routing rule) now; treat raw-source view as a separate decision, likely
  deferred for compiled packages.

### Net critical-path for the P0/P1 UI work
Three thin **read** endpoints unblock the rich surfaces, all over existing data:
1. `GET …/executions/{id}/tasks` (per-task rows + metadata) → WS-1.
2. `GET /v1/agents` (fleet roster) → WS-2.
3. compiler status (proxy or direct) → WS-2.
WS-3 (overview lists), WS-6 (trigger types — already in the triggers payload), and
the metadata half of WS-4/WS-5 need **no** server change. Node **source** is the only
genuine "can't do from current data" item.

## Status Updates

- 2026-06-16: Audit complete (grounded in `crates/cloacina-server/src/routes/executions.rs`, `routes/agent.rs`, `lib.rs`, `crates/cloacina/src/models/task_execution.rs`, `dal/unified/*`, `routes/workflows.rs`). Findings above. Recommend filing the 3 read endpoints as scoped server sub-tasks (or fold into WS-1/WS-2). Node-source flagged as a constraint on WS-5. Ready to complete WS-0.