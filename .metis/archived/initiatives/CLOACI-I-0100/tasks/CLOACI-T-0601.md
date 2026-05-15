---
id: t-04-ttl-prune-service-integration
level: task
title: "T-04: TTL prune service + integration tests + tutorial"
short_code: "CLOACI-T-0601"
created_at: 2026-05-14T20:16:50.239269+00:00
updated_at: 2026-05-14T22:28:57.413942+00:00
parent: CLOACI-I-0100
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0100
---

# T-04: TTL prune service + integration tests + tutorial

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0100]]

## Objective **[REQUIRED]**

Close out I-0100 with the operational pieces and validation.

### TTL prune service

Background sweep in the unified scheduler that calls `dal.reactor_subscriptions().prune_firings_older_than(cutoff)` on a fixed cadence (default 1 hour). TTL configurable via `DefaultRunnerConfig::reactor_firings_retention` (default 7 days).

Pruning is not coupled to subscription watermarks — a subscription whose watermark predates the TTL will miss firings. Document this as a known gotcha.

Metric: `cloacina_reactor_firings_pruned_total` counter (no labels — single global counter is enough).

### Integration tests

In `crates/cloacina/tests/integration/` add a new `reactor_subscriptions.rs`:

1. **End-to-end**: reactor fires → firing row written → subscription polls → workflow executes with payload as context. Postgres + sqlite.
2. **Fan-out**: two subscriptions on the same reactor each drive independent workflow executions.
3. **Tenancy**: subscription in tenant A cannot see firings from tenant B.
4. **At-least-once on crash**: simulate dispatcher exit between dispatch and watermark advance; next poll re-dispatches.
5. **TTL prune**: firings older than the cutoff are deleted; subscriptions whose watermark is past the cutoff miss those firings (documented gotcha).
6. **Filter Skip**: `@trigger(reactor=...)` returning `Skip` advances the watermark without dispatching.

### Tutorial doc

New file `docs/tutorials/event-driven-workflows.md` (or similar — check existing docs structure for the right path) walking through:
- Why event-driven workflows (vs. cron, vs. CG-only)
- The two registration paths (decorator vs. explicit subscribe)
- Filter semantics (Fire/Skip)
- At-least-once delivery + idempotency
- TTL gotcha for long-paused subscribers

Reference from the main docs index.

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

- [ ] TTL prune service runs on a fixed cadence; configurable retention.
- [ ] `cloacina_reactor_firings_pruned_total` counter increments per prune sweep.
- [ ] All six integration tests pass on both backends.
- [ ] Tutorial doc renders + links from the main docs index.
- [ ] Release notes line written for the I-0100 feature.

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
