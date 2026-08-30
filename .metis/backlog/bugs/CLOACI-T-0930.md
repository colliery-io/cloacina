---
id: build-retry-policy-silently
level: task
title: "build_retry_policy silently swallows typo'd retry_backoff/retry_condition strings"
short_code: "CLOACI-T-0930"
created_at: 2026-08-14T03:07:30.077563+00:00
updated_at: 2026-08-14T03:07:30.077563+00:00
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

# build_retry_policy silently swallows typo'd retry_backoff/retry_condition strings

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`build_retry_policy` (`crates/cloacina-python/src/task.rs`) maps its string
kwargs through catch-all arms:

```rust
let strategy = match backoff.as_str() {
    "fixed" => BackoffStrategy::Fixed,
    "linear" => BackoffStrategy::Linear { multiplier: 1.0 },
    "exponential" => BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 },
    _ => BackoffStrategy::Fixed,          // <-- silent
};
...
let retry_cond = match condition.as_str() {
    "never" => RetryCondition::Never,
    "transient" => RetryCondition::TransientOnly,
    "all" => RetryCondition::AllErrors,
    _ => RetryCondition::AllErrors,       // <-- silent
};
```

So `@cloaca.task(retry_backoff="exponentail")` (typo) silently configures
**Fixed** backoff, and an unrecognized `retry_condition` silently becomes
**AllErrors** — retrying errors the author may have meant never to retry. The
author gets no warning; the workflow runs with retry semantics they never
asked for, and the misconfiguration stays invisible until production behaves
oddly.

Found while implementing [[CLOACI-T-0882]], which wired the typed `RetryPolicy`
object as an alternative surface. The typed path cannot express this typo —
`BackoffStrategy.exponential(...)` is a method, so a misspelling is an
immediate `AttributeError`. T-0882 deliberately did NOT fix this, because it
is a behavior change with its own blast radius.

**Fix:** raise `ValueError` listing the accepted values instead of falling
through. **This is a breaking change** for any workflow currently passing an
unrecognized string and silently getting the default — which is exactly the
population most likely to be misconfigured, so failing loudly is the point.
Worth a release-notes line.

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

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (silent misconfiguration, not a crash; the typed
      `RetryPolicy` surface added in T-0882 is a safe alternative today)

## Acceptance Criteria **[REQUIRED]**

- [x] An unrecognized `retry_backoff` raises `ValueError` naming the valid values.
- [x] An unrecognized `retry_condition` raises `ValueError` naming the valid values.
- [x] Python tests cover both rejections AND confirm every valid value still works
      (the regression risk is over-tightening and breaking a legitimate string).
      → 6 passed in test_scenario_11_retry_mechanisms.py; rejection messages
      contain both the typo AND the nearest valid value, so the error teaches
      the fix.
- [x] Release-notes entry flagging the behavior change.
      → CHANGELOG [Unreleased], BREAKING entry.

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

- 2026-08-30 — FIXED AND VERIFIED. `build_retry_policy` returns `PyResult`;
  both `_ =>` fallthroughs raise `ValueError` naming the accepted values.
  Verified via pytest against a rebuilt wheel: both typo rejections plus the
  full valid-value sweep (fixed/linear/exponential, never/transient/all) —
  6 passed. Test-authoring note: valid-value decorations must sit inside a
  `WorkflowBuilder` context, because valid strings get PAST retry parsing and
  reach the workflow-context check that typo'd ones never do.