---
id: t-02-reactor-firing-row-write
level: task
title: "T-02: Reactor firing-row write + subscription poller in unified scheduler"
short_code: "CLOACI-T-0599"
created_at: 2026-05-14T20:16:47.972854+00:00
updated_at: 2026-05-14T20:58:54.197788+00:00
parent: CLOACI-I-0100
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0100
---

# T-02: Reactor firing-row write + subscription poller in unified scheduler

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0100]]

## Objective **[REQUIRED]**

Two changes that connect the reactor to the new subscription path landed in T-0598.

### Part 1: Reactor firing-row write

In `crates/cloacina/src/computation_graph/reactor.rs`, at the fire site that calls `(graph)(snapshot)`, also write a `reactor_firings` row via the DAL. This is the new side effect — the existing CG dispatch is unchanged.

- Reactor needs a `dal: Option<DAL>` field (already exists for resilience persistence).
- Reactor needs a `tenant_id: Option<String>` field for the row's tenant scope. Sourced from the `ComputationGraphDeclaration::tenant_id` set by the scheduler at `load_graph` time.
- The `payload` field carries the serialized boundary cache — the same data the CG traversal consumed. Use `bincode` (matches T-0413's existing serialization).
- Best-effort write: a DAL failure logs a warning but does not fail the CG fire. The supervisor's existing persist-failure path (I-0108 / T-0590) catches sustained DAL trouble.

### Part 2: Subscription poller

Extend `cron_trigger_scheduler.rs` (the "unified scheduler") with a new poll tick for reactor subscriptions:

```
every <poll_interval, default 1s>:
  for each enabled subscription in this tenant:
    firings = dal.poll_unconsumed(tenant, reactor, last_seen_fired_at, limit=100)
    for each firing:
      dispatch workflow with firing.payload as input context
      dal.advance_watermark(subscription.id, firing.fired_at)
```

Dispatch goes through the existing workflow-execution path — same as cron triggers do today. Use `DefaultRunner::execute_workflow` (or whatever the existing trigger dispatch helper is).

At-least-once semantics: if the process crashes between dispatch and watermark advance, the next poll re-dispatches. Workflow idempotency is the user's concern, same as cron-triggered workflows.

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

- [ ] Every reactor fire writes one `reactor_firings` row (best-effort; supervisor persist counter catches failures).
- [ ] Unified scheduler's poll tick dispatches workflows for unconsumed firings.
- [ ] Watermark advance is atomic with dispatch attempt (at-least-once on crash).
- [ ] Two subscriptions on the same reactor each drive independent workflow executions (fan-out works).
- [ ] Tenant isolation: subscription in tenant A sees only tenant-A firings.
- [ ] New metric: `cloacina_reactor_firings_total{graph,reactor}` counts firing-row writes. Update I-0099 cardinality guard.

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
