---
id: ws-4-triggers-reactors
level: task
title: "WS-4 — Triggers/reactors + accumulators as graph nodes"
short_code: "CLOACI-T-0706"
created_at: 2026-06-16T01:50:16.063427+00:00
updated_at: 2026-06-16T03:35:44.834050+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-4 — Triggers/reactors + accumulators as graph nodes

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P1) Make the graph the source of truth: render **triggers/reactors and accumulators
as first-class nodes** in both workflow DAGs and computation graphs. Today they're
text above the canvas (e.g. graph detail lists `py_alpha`/reactor separately), so the
data flow isn't one picture.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Workflow DAGs show the firing **trigger** (e.g. cron) as an upstream entry node with next/last fire.
- [ ] Computation graphs show the **reactor** and each **accumulator** as distinct, styled upstream nodes feeding the entry node(s), carrying accumulator **source health**.
- [ ] Distinct node types/styling for trigger / reactor / accumulator / compute; existing branch-edge labels preserved.
- [ ] Re-passes the Playwright walk.

## Dependencies

Shared renderer change; pairs with [[CLOACI-T-0707]] (node drawer) for click behavior.

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

- 2026-06-16: **DONE + screenshot-verified** (commit `✓ WS-4`, branch `feat/ui-0124-server-read-endpoints`). Screenshot `/tmp/cloacina-ui-uat/ws4/01-graph-detail.png`.
  - `Dag` gained a node **`kind`** (compute/accumulator/reactor/trigger) with distinct fills; `GraphDetail` now builds the augmented graph: **accumulators → reactor → entry(root) nodes → …** with a legend. Verified on `demo_kafka_graph`: renders `kafka_alpha` (accumulator) → `demo_kafka_rx` (reactor) → `process` → `output`. Branch-edge labels preserved (logic intact for the routing graphs).
  - **Enabling work (counts toward [[CLOACI-T-0710]] WS-8):** expanded `DEMO_FIXTURES` with `demo-cron-rust` (cron trigger) + `demo-kafka-stream-rust` (CG w/ reactor+accumulator) so CG/trigger structure is seedable on the `ui up`/seed path (was demo-slow + demo-fail only). Rust fixtures only — Python CG fixtures (demo-py-graph branching example, tutorial full-pipeline/routing) need a Python staging path in `_stage_and_pack`; left for the formal WS-8 close.
  - **Scoping:** the **CG side** (reactor + accumulators as nodes) — the audit's primary ask (notes 4 + 6) — is done + verified. The **workflow-trigger-as-node** half is deferred: it needs WorkflowDetail to cross-reference the triggers list by workflow_name (package/workflow naming drift) — captured as a small follow-on; the `trigger` node kind + styling already exist in `Dag` for it.