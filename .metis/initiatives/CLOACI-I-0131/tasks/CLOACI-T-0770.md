---
id: python-cg-boundary-typing-declare
level: task
title: "Python CG boundary typing — declare accumulator boundary schemas for typed inject/fire"
short_code: "CLOACI-T-0770"
created_at: 2026-06-21T23:02:58.439389+00:00
updated_at: 2026-06-21T23:20:24.290051+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Python CG boundary typing — declare accumulator boundary schemas for typed inject/fire

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Follow-up to T-0768: Rust CG accumulators get typed inject/fire forms by deriving
`schemars::JsonSchema` on the boundary type, but Python CG accumulators (untyped
dicts) had no schema → the inject/fire modal fell back to a raw-JSON textarea
("doesn't seem MUCH better"). Give Python parity.

## Status Updates **[REQUIRED]**

### 2026-06-21 — implemented (2fc26bf8), demo rebuild verifying
Approach mirrors the @cloaca.workflow_params → param_parse precedent (the
authoritative declaration is parsed from Python source at build time; runtime is
a no-op):
- cloaca: `@cloaca.boundary_schema(field=type, ...)` no-op decorator on the
  accumulator fn (workflow.rs; dual-registered lib.rs + loader.rs).
- compiler: `param_parse::parse_boundary_schemas` finds the decorator + the `def`
  it sits on → DeclaredSurface{kind:"accumulator", name:<fn>, one object slot}.
  Unit-tested (incl. the case where state_accumulator(...) sits between).
- plumbing: declared_surfaces threaded run_build → BuildOutcome::Success → loopp
  → mark_build_success_with_docs → extract_and_merge_build_metadata (Python
  empty-cdylib branch merges into package metadata, same path as declared_params).
  cloacina-compiler += cloacina-api-types dep.
- fixtures: demo-py-graph py_alpha + demo-py-state py_window declare {bid, ask}.

cargo check (compiler/cloacina/python) + param_parse tests green.

### 2026-06-21 — VERIFIED DONE (2fc26bf8 + docs)
Live on the rebuilt demo: py_alpha + py_window /interface now return typed object
schemas `{bid, ask: number}` (were `[]`). The demo_py_graph inject modal renders
py_alpha.ask / py_alpha.bid NumberInputs — full parity with the Rust graphs.
Documented @cloaca.boundary_schema in boundary.md (Python-parity section).
All four I-0131 follow-on tasks (T-0768/0769/0770) complete.

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