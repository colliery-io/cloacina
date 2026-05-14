---
id: t-03-trigger-reactor-decorator
level: task
title: "T-03: @trigger(reactor=...) decorator + subscribe_workflow_to_reactor() API"
short_code: "CLOACI-T-0600"
created_at: 2026-05-14T20:16:49.304852+00:00
updated_at: 2026-05-14T22:09:27.709746+00:00
parent: CLOACI-I-0100
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0100
---

# T-03: @trigger(reactor=...) decorator + subscribe_workflow_to_reactor() API

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0100]]

## Objective **[REQUIRED]**

User-facing surface for reactor subscriptions: a `@trigger(reactor=...)` decorator extension plus an explicit `subscribe_workflow_to_reactor()` registration API. Both forms target the same DAL `subscribe()` call from T-0598.

### Python decorator extension

In `cloacina-python/src/bindings/triggers.rs` (or equivalent), extend the existing `@cloaca.trigger` decorator to accept a `reactor=` kwarg:

```python
@cloaca.trigger(reactor="pricing_reactor")
def on_pricing_firing(ctx):
    # optional filter — return Fire or Skip
    return cloaca.TriggerResult.fire(ctx)
```

The decorated function becomes a Python-side filter that runs in the subscription poller's dispatch path. Return value:
- `TriggerResult.fire(ctx)` → dispatch workflow with `ctx` as input
- `TriggerResult.skip()` → skip this firing (but watermark still advances)

The filter receives a Python object built from the firing's `payload` field (deserialized via the existing `Context::from_json` path).

### Explicit registration API

For the trivial "fire on every reactor firing, no filter" case, expose a direct API:

```python
cloaca.subscribe_workflow_to_reactor(
    reactor="pricing_reactor",
    workflow="incident_response",
    tenant="my-tenant",  # optional, defaults to runner's configured tenant
)
```

Maps directly to `dal.reactor_subscriptions().subscribe(reactor, workflow, tenant)`. Idempotent — calling twice with the same args returns the same subscription id without error (matches T-0598 upsert).

### Rust surface

Mirror API on the Rust side: `DefaultRunner::subscribe_workflow_to_reactor(reactor, workflow)` for users authoring in Rust directly.

### Filter dispatch in poller (T-0599 connection)

The poller from T-0599 already dispatches workflows. With the decorator's filter present, the poller calls the Python filter first, dispatches only on `Fire`, but always advances the watermark. Skip events are not retried.

If a subscription has no Python filter (created via `subscribe_workflow_to_reactor()`), the poller dispatches unconditionally.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] `@cloaca.trigger(reactor="...")` registers a subscription with a Python filter.
- [ ] `cloaca.subscribe_workflow_to_reactor(reactor, workflow)` registers a subscription without a filter.
- [ ] Decorator and registration API both upsert by `(reactor, workflow, tenant)` — calling twice doesn't duplicate.
- [ ] Filter `TriggerResult.skip()` advances the watermark but does not dispatch.
- [ ] Filter mutation of `ctx` propagates to the dispatched workflow's input context.
- [ ] Rust mirror API `DefaultRunner::subscribe_workflow_to_reactor` works equivalently.

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
