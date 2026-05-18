---
id: t-05-python-trigger-reactor-filter
level: task
title: "T-05: Python @trigger(reactor=...) filter callback dispatch"
short_code: "CLOACI-T-0602"
created_at: 2026-05-14T21:44:32.032324+00:00
updated_at: 2026-05-18T12:37:50.330764+00:00
parent: CLOACI-I-0100
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0100
---

# T-05: Python @trigger(reactor=...) filter callback dispatch

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0100]]

## Objective **[REQUIRED]**

Extend the existing `@cloaca.trigger` decorator to accept a `reactor=` kwarg that wires a Python-side filter callable into the unified scheduler's reactor poll dispatch path (CLOACI-T-0599). T-0600 shipped the unfiltered `subscribe_workflow_to_reactor` registration; this task adds the filtered variant.

```python
@cloaca.trigger(reactor="pricing_reactor")
def on_pricing_firing(ctx):
    if ctx["price"] > 100:
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

Behaviour:
- `TriggerResult.fire(ctx)` → poller dispatches with the (possibly mutated) ctx
- `TriggerResult.skip()` → no dispatch; watermark still advances (no retry)
- Filter panic/exception → log warn, treat as skip, advance watermark

Why this is its own task (deferred from T-0600):
1. Needs a subscription→Python-callable registry that survives runner restart.
2. Needs GIL acquire from the tokio scheduler tick (`Python::with_gil` plus error isolation).
3. Needs payload→PyAny marshalling so the filter can mutate ctx before dispatch.
4. Design pass should pick: sync vs async filter, registry scope (runner vs runtime), and how filter exceptions interact with the at-least-once contract.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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