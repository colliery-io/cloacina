---
id: python-parity-type-hints-to-json
level: task
title: "Python parity — type-hints to JSON Schema for params and boundaries"
short_code: "CLOACI-T-0760"
created_at: 2026-06-20T16:46:03.673640+00:00
updated_at: 2026-06-21T00:48:51.868186+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Python parity — type-hints to JSON Schema for params and boundaries

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0128]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

### 2026-06-20 — Scoped; BLOCKED on a Python authoring-model decision (+ D for boundaries)

Task F of [[CLOACI-I-0128]] — Python parity for declared params (and boundaries).
Recon of the Python authoring path:
- Python workflows are built via `PyWorkflowBuilder`
  (`crates/cloacina-python/src/workflow.rs`): `new(name, *, tenant, package,
  workflow)`, `.description()`, `.tag()`, `.add_task()`, `.build()`. It wraps the
  **embedded** `cloacina::Workflow::builder` — NOT the packaged cdylib FFI, so
  Python params can't reuse B's `get_input_interface` entrypoint; they need their
  own carry into `WorkflowMetadata.declared_params`.
- There is **no param-declaration surface** today, and the builder has no
  type-hinted input signature to read (Python workflows operate on a context
  dict). So "type-hints → JSON Schema" needs a concrete decision on the
  declaration mechanism + the schema source:

**Decision needed (Python authoring-model):**
1. How params are declared — a builder method (`.param(name, schema, default)` /
   `params=` kwarg) vs a typed input model the author attaches.
2. The type→JSON-Schema mechanism — **pydantic** (`model_json_schema()`, natural
   + rich, adds a dep/expectation) vs **dataclass + a hand-rolled mapping**
   (lean, limited) vs author-supplied raw JSON Schema (no inference).

This is a real design call, and Python is a core capability ([[project_python_is_core]]),
so it shouldn't be invented unilaterally. The **boundaries** half additionally
depends on D ([[CLOACI-T-0758]], blocked). The Rust workflow-params path (B/C) is
the reference; once the Python model is chosen, the carry into `declared_params`
+ the execute-time validation (mirroring T-0757) port directly.

**Blocked** pending the authoring-model decision (params half) and D (boundaries
half).

### 2026-06-20 — DECISION: Python decorator parsed from source; DONE + VERIFIED

Maintainer chose the **decorator-parsed-from-source** mechanism. Implemented +
committed (`feat: Python parity for declared workflow params`):
- **Authoring**: `@cloaca.workflow_params(name=type, name=(type, default), …)` —
  a runtime no-op pass-through decorator (`cloacina-python`, registered in both
  the maturin pymodule and the synthetic loader module).
- **Build-time parse**: `cloacina-compiler/src/param_parse.rs` parses the
  decorator from Python source (scalar map str/int/float/bool/list/dict →
  JSON Schema; `(type, default)` → optional), the same source-parse approach as
  the T-0752 doc extractor.
- **Carry**: threaded build → loopp → `mark_build_success_with_docs`;
  `extract_and_merge` writes the params onto the stored metadata in the
  **empty-artifact (Python) branch** — Python's only carry path (no cdylib FFI).
- Reuses B/C: the params then surface on `WorkflowDetail.declared_params` and are
  validated at execute exactly like Rust.
- `demo-py-workflow` fixture declares params as the worked example.

Verified: 5 param_parse unit tests + integration carry test
(`test_python_declared_params_persisted_on_empty_artifact_build`) + full lanes
green (315+100+6, 0 failed).

Scope: workflow params (the parity ask). Python accumulator/reactor boundary
typing would follow the same source-parse pattern if needed later (Rust's D is
opt-in typed; Python CG authoring is a separate surface). Done.