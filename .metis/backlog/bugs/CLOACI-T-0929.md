---
id: trigger-fire-cannot-fire-an-on
level: task
title: "trigger fire cannot fire an `on =` trigger — it only resolves subscription-side targets"
short_code: "CLOACI-T-0929"
created_at: 2026-08-09T21:57:26.593521+00:00
updated_at: 2026-08-09T21:57:26.593521+00:00
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

# trigger fire cannot fire an `on =` trigger — it only resolves subscription-side targets

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`trigger fire` cannot fire a trigger declared with `on = "..."`. It reports

    not found — resource 'trigger 'inbox_poll' has no subscribed workflows'

even though that trigger demonstrably drives a workflow — it fires
`file_processing` on its own every 3s.

Cloacina has TWO ways to wire a trigger to work, and the manual-fire path only
knows about one of them:

| Shape | Wiring | `trigger fire` |
|---|---|---|
| `#[trigger(on = "wf")]` | the trigger names the workflow it drives | **fails** |
| `#[workflow(triggers = ["t"])]` | workflows subscribe to a named trigger | works, fans out |

`routes/triggers.rs::fire_trigger` resolves its fan-out set with
`registry.find_trigger_subscribers(&name)` — purely the subscription side. Its
own comment explains why: "The schedules table carries only the trigger's
primary `on` workflow, so subscriptions — which may live in other packages —
are resolved from the registry's workflow metadata." That reasoning is sound for
finding subscribers; the bug is that the `on` workflow is then **dropped
entirely** rather than being included alongside them.

Found by CLOACI-T-0893's operator-verb assertions: the lane runs each command an
example's "Operate it" section documents, and this one failed on a README I had
just written claiming it worked. Which is the point of those assertions.

## Impact

The manual-fire verb is unavailable for the `on =` trigger shape — the shape the
`packaged-triggers` example teaches. An operator wanting an immediate run has to
know to use `workflow run <name>` instead, and nothing tells them that; the
error message talks about subscribers, which is a concept their package never
used.

`pause` and `resume` work fine for both shapes (they resolve through the
schedules row), so the inconsistency is specifically in `fire`.

## Fix direction

Include the trigger's own `on` workflow in the fan-out set, unioned with any
subscribers and deduped, so both shapes fire. The schedule row already carries
it (`schedules.workflow_name`), and `fire_trigger` already resolves the
tenant-scoped DAL it would need.

Keep the empty case an error — a trigger with neither an `on` workflow nor
subscribers genuinely has nothing to fire — but the message should say so in
terms that fit both shapes rather than only mentioning subscriptions.

## Notes

Documented honestly in `examples/features/workflows/packaged-triggers/README.md`
in the meantime: the README states which shape `fire` applies to, and points at
`workflow run` for an immediate run. Remove that caveat when this lands.

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