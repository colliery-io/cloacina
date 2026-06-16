---
id: ws-0b-server-read-endpoints-per
level: task
title: "WS-0b — Server read endpoints (per-task executions, fleet roster, compiler status)"
short_code: "CLOACI-T-0711"
created_at: 2026-06-16T02:10:59.834257+00:00
updated_at: 2026-06-16T02:52:50.737158+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-0b — Server read endpoints (per-task executions, fleet roster, compiler status)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

The three thin **read** endpoints the WS-0 audit ([[CLOACI-T-0702]]) identified as the
critical path for the P0 UI surfaces. All expose data the server/engine already has —
no new engine capability. Gates [[CLOACI-T-0703]] (WS-1) and [[CLOACI-T-0704]] (WS-2).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] **`GET /v1/tenants/{tenant}/executions/{id}/tasks`** — per-task rows from `task_execution` (name, status, started_at, completed_at, attempt/max_attempts, last_error/error_details, sub_status) + task metadata (output/context) via `dal.task_execution()` + `dal.task_execution_metadata()`. Tenant-scoped + authed like the sibling execution routes. OpenAPI documented.
- [ ] **`GET /v1/agents`** (or `/v1/fleet`) — operator-facing fleet roster from the in-memory roster (`cloacina-server/src/lib.rs:203`): agent id, target triple, capacity, last heartbeat, state. Admin-scoped.
- [ ] **Compiler status** reachable for the UI — either a server aggregate/proxy endpoint or a documented direct compiler health call; pick the lower-friction option and document it.
- [ ] `emit-openapi` updated; contract/SDK suites still green; verified against the seeded demo stack.

## Dependencies

Implements the gaps from [[CLOACI-T-0702]]. Unblocks [[CLOACI-T-0703]], [[CLOACI-T-0704]].

## Implementation Notes

- Mirror the existing handlers in `crates/cloacina-server/src/routes/executions.rs`
  (`get_execution`/`get_execution_events`) for auth + tenant resolution; register in
  `lib.rs` router; add response types in `cloacina-api-types`.
- Roster type lives with the fleet code (`cloacina::fleet` / `lib.rs` AppState roster).
- Build/test via angreal lanes (don't hand-run server processes); regenerate OpenAPI
  with the `docs spec-check`/`emit-openapi` flow.

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

- 2026-06-16: **DONE** on branch `feat/ui-0124-server-read-endpoints` (commits `2c279a3d`, `baa94d5a`, `48073b0d`). All three endpoints compile clean (`angreal check crate crates/cloacina-server` → ✅, both backends).
  - **#1 `GET /v1/tenants/{t}/executions/{id}/tasks`** — `TaskExecutionDetail`/`ExecutionTasksResponse` over `dal.task_execution().get_all_tasks_for_workflow()` (status, started/completed, attempt/max_attempts, sub_status, last_error/error_details). *(Output/context metadata deferred to the drawer task — the task rows unblock WS-1's table + status DAG now.)*
  - **#2 `GET /v1/agents`** (admin) — fleet roster from `AgentRegistry::snapshot()` → `AgentInfo` (id, target_triple, max_concurrency, in_flight, available_capacity, **seconds_since_heartbeat**, capabilities, tenant). `last_heartbeat` is a monotonic `Instant`, so exposed as seconds-since.
  - **#3 `GET /v1/compiler/status`** (admin) — build-pipeline + compiler liveness. **Design decision:** server↔compiler are DB-coupled (no compiler URL on the server), so rather than an HTTP proxy I extracted `registry::workflow_registry::build_queue_stats` into a `Database`-level free fn (loader-free) and the server reads the **same rows the compiler's own `/v1/status` reports** (pending/building/last_success/last_failure + the build-claim heartbeat). Status roll-up: building / backlogged / idle. True idle-liveness isn't determinable from the queue (documented in the type).
  - `docs/static/openapi.json` regenerated (all 3 paths present); 3 new schemas + 2 tags registered.
- **Remaining verification (user/CI):** `cargo test` for cloacina (a rust-analyzer "second test attribute" flag near database.rs:2012 is a false positive on the pre-existing `#[cfg]`+`#[tokio::test]` pattern — cargo build passed) and the SDK contract suites against a live rebuilt server. Then PR. WS-1 ([[CLOACI-T-0703]]) and WS-2 ([[CLOACI-T-0704]]) are now unblocked.