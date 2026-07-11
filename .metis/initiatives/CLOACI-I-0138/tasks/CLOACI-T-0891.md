---
id: gold-path-example-computation
level: task
title: "Gold-path example: computation-graph feature tour — stream/batch/polling accumulators, task-to-CG invocation, boundary_schema"
short_code: "CLOACI-T-0891"
created_at: 2026-07-11T22:03:26.284573+00:00
updated_at: 2026-07-11T22:03:26.284573+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Gold-path example: computation-graph feature tour — stream/batch/polling accumulators, task-to-CG invocation, boundary_schema

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

The reactive layer's advanced surface is fixtures-only. Tutorials + `packaged-graph` cover CG basics (passthrough/state accumulators, reactor criteria, routing); NOT covered anywhere user-facing:
- **`#[stream_accumulator]`** (kafka: `type/topic/group/state` — accumulator_macros.rs:46-58) — only `demo-kafka-stream-rust` fixture
- **`#[batch_accumulator]`** (`flush_interval/max_buffer_size` — :342-346) and **`#[polling_accumulator]`** (`interval` — :240) — fixtures only
- **Task→CG invocation**: `#[task(invokes = computation_graph("name"), post_invocation = …)]` (tasks.rs:150/184) — the workflow↔CG bridge, fixtures only
- **`@cloaca.boundary_schema(**kwargs)`** (python typed CG surfaces — workflow.rs:516) — fixtures only

**Build:** one richer gold-path CG example (extend `packaged-graph` or new `examples/features/computation-graphs/cg-feature-tour/`): a kafka stream accumulator feeding a reactor (demo stack has kafka), a polling accumulator, a batch accumulator, a workflow task that `invokes` the CG with `post_invocation`, and — python side — `boundary_schema` on the python-packaged-graph example (or the T-0885 python canonical). Split into two examples if one becomes unreadable; teaching clarity beats completionism.

**Shape:** T-0886 standard; runs on the demo stack (kafka available); demos-features runner; auto-joins CI.

**Acceptance:** each of the four uncovered surfaces appears in a user-facing example with a verified README step showing it working on the demo stack (accumulator ingest visible via `cloacinactl graph accumulators` / UI; task→CG invocation reaches Completed).

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

*To be added during implementation*
