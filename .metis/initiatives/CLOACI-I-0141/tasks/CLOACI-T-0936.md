---
id: wave-5-parity-gate-and-release
level: task
title: "Wave 5 parity gate and release — full e2e/visual run, CI lane swap, lockstep, 0.11.0 resumes"
short_code: "CLOACI-T-0936"
created_at: 2026-08-30T11:38:06.699741+00:00
updated_at: 2026-08-30T11:38:06.699741+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0141
---

# Wave 5 parity gate and release — full e2e/visual run, CI lane swap, lockstep, 0.11.0 resumes

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0141]]

## Objective **[REQUIRED]**

Close the migration and un-park the release train:

1. Full Playwright e2e + visual parity run against the demo stack; fix what
   falls out.
2. CI: ui lanes swap node/vite for rust/trunk (UI Checks, ui-visual,
   nightly ui-e2e); wasm32 target where needed.
3. Version lockstep: `ui/package.json`/harness touchpoints drop out of
   `angreal release bump` + the drift guard + the UI Checks REQ-008 gate;
   the Leptos crate version becomes a touchpoint.
4. Docs: embedded-UI pages updated (build prereqs: trunk, wasm target);
   npm removed from the release path.
5. 0.11.0 resumes: rebase release/v0.11.0 (PR #264), add the UI-migration
   changelog entry (breaking: node no longer needed to build the server UI),
   fix the parked `docs/static/openapi.json` info.version touchpoint
   (regenerate via emit-openapi), tag v0.11.0 on merge (maintainer-approved
   2026-08-30).

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

- [ ] Full e2e + visual suites green against the demo stack serving the
      Leptos UI.
- [ ] All CI ui lanes green on the trunk build; no node step remains in any
      server/UI build path.
- [ ] `angreal release check` passes with the updated touchpoint set.
- [ ] v0.11.0 tagged; unified_release's nightly-suite gate passes and the
      release publishes.

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