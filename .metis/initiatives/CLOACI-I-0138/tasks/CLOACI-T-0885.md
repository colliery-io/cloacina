---
id: canonical-python-packaged-example
level: task
title: "Canonical Python packaged example — promote + gold-path demo-stack README"
short_code: "CLOACI-T-0885"
created_at: 2026-07-10T01:16:02.734335+00:00
updated_at: 2026-07-12T12:44:48.449630+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Canonical Python packaged example — promote + gold-path demo-stack README

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

The Python peer of [[CLOACI-T-0884]]: a canonical PYTHON packaged example demonstrating the primary interface (pack → upload → compile → reconcile → execute) — the packaged-Python gold path that no user-facing example currently shows (only fixtures: demo-py-*). Establishes the Python half of the T-0886 standard.

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

### 2026-07-12 — canonical Python packaged example built; lane running
Grounded the packaged-Python shape from `cloacinactl package new --language python` (new.rs:163) + the `demo-py-workflow` fixture: minimal `package.toml` (`[package] name/version` + `[metadata] workflow_name/description` — language + entry_module INFERRED from the `workflow/<name>/` layout), `workflow/<module>/__init__.py` (empty) + `tasks.py` with bare `@cloaca.task(id=, dependencies=[])` decorators (NOT WorkflowBuilder — that's in-process only). No Cargo/build.rs: the compiler skips cargo for `language=python`, the reconciler imports via embedded Python.

Key confirmation (maintainer): **Python is a CORE server capability** — `cloacina-server` unconditionally deps `cloacina-python` + calls `cloacina_python::install()` at startup; the server synthesizes `cloaca` in-process via `ensure_cloaca_module` (I-0137). So a default host `cloacina-server` loads Python packages — no `--features` needed, and the `_run_gold_path` host lane works for Python (build is fast — no cargo).

Built `examples/features/workflows/python-packaged/` (peer of `simple-packaged`): `data_pipeline` = collect_data → process_data → generate_report, with what/why docstrings (T-0754 UI surfacing). Gold-path README. Bespoke `angreal demos features python-packaged` lane (excluded from auto-registration since `cargo run` is wrong for Python) reusing `_run_gold_path`; auto-joins the CI matrix (16 examples). Lane run in progress.
