---
id: bug-polling-and-batch-accumulators
level: task
title: "BUG: polling and batch accumulators silently degrade to passthrough in packaged graphs"
short_code: "CLOACI-T-0896"
created_at: 2026-07-12T01:36:33.300520+00:00
updated_at: 2026-07-12T01:36:33.300520+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# BUG: polling and batch accumulators silently degrade to passthrough in packaged graphs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0891 (2026-07-12, I-0138 feature-coverage push):** the packaged-graph accumulator factory (`computation_graph/packaging_bridge.rs:225`) matches only `"stream"` and `"state"`; everything else — including `"polling"` and `"batch"`, which have real macros (`#[polling_accumulator(interval)]`, `#[batch_accumulator(flush_interval, max_buffer_size)]`) and real python builders (`cloaca.polling_accumulator/batch_accumulator`) — hits the `_ =>` arm and **silently becomes a passthrough accumulator**. A user who declares a batch accumulator in a packaged graph gets per-event firing with no warning: worse than unsupported, it's silently wrong. (Same silent-degradation class as the pre-T-0839 bug for authored specs.)

**Fix:**
1. `packaging_bridge.rs` factory match gains `"polling"` and `"batch"` arms wired to their real factories (they exist for the embedded path — locate `PollingAccumulatorFactory`/`BatchAccumulatorFactory` or the runtime equivalents and thread their configs: `interval`; `flush_interval`/`max_buffer_size`).
2. The `_ =>` fallback should WARN loudly (or fail the load) instead of silently passthrough-ing an unknown declared type.
3. Extend the T-0891 CG feature-tour example to cover polling + batch once functional; its harness lane is the regression net.

**Acceptance:** a packaged graph declaring polling and batch accumulators (macro or `[[metadata.accumulators]]` override) exhibits real polling/batching behavior on the demo stack; unknown types are loud. Related: [[CLOACI-T-0891]], T-0839 (the authored-spec analog of this bug).

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
