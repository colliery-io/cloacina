---
id: ws-5-node-task-detail-drawer-code
level: task
title: "WS-5 — Node/task detail drawer (code/logic, I/O, retry, routing)"
short_code: "CLOACI-T-0707"
created_at: 2026-06-16T01:50:17.243165+00:00
updated_at: 2026-06-16T03:52:55.454513+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-5 — Node/task detail drawer (code/logic, I/O, retry, routing)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P1) Give every graph/DAG node a **detail drawer** on click — today clicking a node
only outlines it (no information). One shared component across workflow DAGs and
computation graphs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Clicking a node opens a drawer with: **source/logic**, **inputs/outputs**, **dependencies**, **retry policy** (tasks), and **routing rule** (CG branch nodes).
- [ ] Works for both workflow task nodes and CG nodes; degrades gracefully when a field is unavailable.
- [ ] Reuses one shared drawer component; re-passes the Playwright walk.

## Dependencies

Depends on [[CLOACI-T-0702]] (node source/IO retrievability); composes with
[[CLOACI-T-0706]] (node rendering).

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

## Status Updates **[REQUIRED]**

### 2026-06-15 — DONE (CG node drawer shipped + verified)

Shipped a shared node-detail drawer on the CG view (`ui/src/routes/GraphDetail.tsx`,
`ui/src/components/Dag.tsx`):

- `Dag` now takes an `onNodeClick(id)` prop wired to ReactFlow's `onNodeClick`.
- Clicking any node (compute / accumulator / reactor) opens a right-side Mantine
  `Drawer`. `describeNode()` builds the rows per kind:
  - **Accumulator** — role.
  - **Reactor** — criteria (`reaction_mode`), input strategy, bound accumulators, role.
  - **Compute node** — inputs, upstream deps, routes-to (with routing-variant labels).
- Degrades gracefully: every field falls back to "—" / "entry node" / "terminal".
- Drawer footer states plainly that node **source isn't shipped in compiled
  `.cloacina` packages**, so the function body can't be shown — the honest answer
  to the "view into node code" feedback rather than a fake code panel.

Verified live via Playwright (`ui/e2e/ws5.spec.ts`) against the seeded
`demo_kafka_graph`: `01-reactor-drawer.png` shows "Reactor: demo_kafka_rx"
(when_any / latest / kafka_alpha); `02-node-drawer.png` shows "Node: process"
(inputs kafka_alpha / upstream — entry node / routes to output) plus the
source-unavailable note.

**Scope landed vs. deferred:** the CG node drawer (the P1 of this ticket) is done.
Two pieces are explicitly deferred to backlog because they need server enrichment
that doesn't exist yet:
- **Workflow task-node drawer** sharing the same component — needs the workflow DAG
  route to pass node metadata; tracked alongside WS-6/WS-7 polish.
- **Per-task output/context** in the execution TaskTable drawer — needs a
  `task_execution_metadata` join the read API doesn't expose yet.

Committed on `feat/ui-0124-server-read-endpoints`.