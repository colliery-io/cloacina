---
id: close-sdk-coverage-gate-wrap-10
level: task
title: "Close SDK coverage gate — wrap 10 operator/health endpoints in TS/Rust/Python SDKs"
short_code: "CLOACI-T-0772"
created_at: 2026-06-22T12:01:37.808473+00:00
updated_at: 2026-06-22T12:43:40.694300+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Close SDK coverage gate — wrap 10 operator/health endpoints in TS/Rust/Python SDKs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Nightly's `sdk-contract` job (release blocker) fails on `check_sdk_coverage.py`:
the coverage rule requires every OpenAPI operation reachable from all 3 SDKs, and
11 operator/health endpoints were uncovered (7 pre-existing from I-0126/I-0128
operator controls; 2 fires from T-0766; inject). Wrap them all in TS/Rust/Python.

## Status Updates **[REQUIRED]**

### 2026-06-22 — DONE (3ea85ac3)
Wrapped all 11 in the three SDKs:
- TS `client.ts` + Rust `cloacina-client/lib.rs`: fireReactor, injectAccumulator,
  listReactorFires, reactorFireTimeseries, reactor/accumulator interface,
  pause/resume workflow + trigger, workflow source.
- Python: regenerated `_generated` from the current spec
  (`uvx openapi-python-client@0.29.0`, the README-pinned command) so the
  operation modules exist, then imported + wrapped them in `_client.py`.

`scripts/check_sdk_coverage.py`: 38/38 operations reachable from all 3 SDKs
(was 30 violations → 0). TS build + `cargo check -p cloacina-client` +
`_client.py` parse green. The live contract suites have no coverage-enforcement
(hand-written "hit at least once"), so they pass; adding explicit contract cases
for the new endpoints is an optional follow-up.

### 2026-06-22 — VERIFIED GREEN (6bc09d6c)
First nightly verify exposed a regen knock-on: `list_agents` moved api/agents →
api/fleet (the spec's tag) but `_client.py` still imported api.agents →
ModuleNotFoundError in the python contract suite. Fixed + now verify every
`_client.py` import resolves. SDK Contract Matrix is **green** on the nightly
(run 27952950851), along with Code Coverage + UI Acceptance E2E. Done.

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