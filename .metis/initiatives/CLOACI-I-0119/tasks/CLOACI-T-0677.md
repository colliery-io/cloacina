---
id: reconcile-the-packaged-workflow
level: task
title: "Reconcile the packaged-workflow format — one canonical layout, fix Python docs + broken example"
short_code: "CLOACI-T-0677"
created_at: 2026-06-14T15:31:35.240398+00:00
updated_at: 2026-06-14T15:54:16.503804+00:00
parent: CLOACI-I-0119
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0119
---

# Reconcile the packaged-workflow format — one canonical layout, fix Python docs + broken example

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0119]]

## Objective **[REQUIRED]**

T4 of CLOACI-I-0119. Establish **one canonical packaged-workflow format** and make
docs + examples conform to it (today docs ≠ examples ≠ server). Concretely: a
single "package format" reference (reconciler-accepted layout + `package.toml`
schema), a rewritten Python packaging how-to, and a fixed broken Python CG
example. Foundation T1 (`package new`) and T2 (`package pack`) build on.

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

- [x] A single canonical "package format" reference doc: `package-format.md`
      rewritten to cover the accepted directory layouts (Rust + Python) and the
      exact `package.toml` `[package]`/`[metadata]` schema (accepted keys;
      `package_type`/`[[metadata.triggers]]` rejected). `package-manifest.md`
      collapsed to a redirect.
- [x] The Python packaging how-to is rewritten to the server-accepted format
      (`package.toml` + module under `workflow/<mod>/`); the obsolete top-level
      module + `manifest.json` procedure is removed.
- [x] The broken `examples/.../python-packaged-graph` is fixed (module under
      `workflow/`) so it actually loads — verified on a live server.
- [x] CLOACI-T-0666 (compiler read `[package].language`, already fixed) is closed.
- [x] No remaining doc/example references to rejected keys or the old Python
      layout (re-grep clean; remaining hits are explanatory "obsolete" notes and
      auto-generated rustdoc for the still-existing `ManifestV2` Rust type).

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

**2026-06-14 — In progress (worst offenders fixed + verified).**
- Fixed the broken `examples/.../python-packaged-graph` — moved `market_maker/`
  under `workflow/market_maker/`. **Verified on the live demo server**: packed via
  the documented tar recipe, uploaded (201), `market_maker` registered in
  `/v1/health/graphs`.
- Rewrote `docs/.../python/workflows/how-to-guides/packaging-python-workflows.md`
  to the canonical format: `package.toml` (`[package]` + closed `[metadata]`
  schema) + module under `workflow/<mod>/`; removed the obsolete
  `pyproject.toml`/`[tool.cloaca]`/`manifest.json`/top-level-module procedure;
  documented accepted `[metadata]` keys + the `workflow/`/rejected-keys footguns.
- Canonical format stated in the how-to + the I-0119 initiative.

**Remaining (straggler sweep — old format still lingers):**
- `docs/.../python/workflows/tutorials/08-packaged-triggers.md` (tool.cloaca/manifest)
- `examples/features/workflows/python-workflow/pyproject.toml` (old python example)
- `examples/tutorials/python/workflows/08_packaged_triggers.py`
- `docs/content/platform/reference/package-manifest.md` +
  `docs/content/platform/explanation/package-format.md` (manifest.json-era refs —
  confirm vs current format)
- `docs/.../computation-graphs/tutorials/service/07-packaging.md` (`package_type`)
- Re-grep to confirm no stragglers; close T-0666 (compiler `[metadata].language`,
  already fixed — Python packages build/load fine, e.g. market_maker just did).

**2026-06-14 — Completed (straggler sweep done).** Commit `0b78705e` on
`i0119-authoring-dx`:
- `08-packaged-triggers` (`.md` + tutorial `.py`): removed the dead
  manifest.json/ManifestV2 trigger-array narrative; both now state the
  `@cloaca.trigger` decorator *is* the declaration (registered at import), and
  `package.toml` has no triggers section / rejects `package_type`.
- `package-format.md`: full rewrite into the **canonical** reference — bzip2 tar
  of source under `<name>-<version>/`, `[package]` + closed `[metadata]` schema
  for both Rust and Python, the build-on-load flow (cargo for Rust, import for
  Python), and the rejected keys. Verified against source via subagent +
  fixtures (`examples/fixtures/demo-cron-rust`, `demo-py-workflow`).
- `package-manifest.md`: collapsed the 380-line obsolete manifest.json schema to
  a thin redirect to `package-format.md`.
- T-0666 closed.
- Final re-grep clean: remaining `manifest.json`/`tool.cloaca`/`package_type`
  hits are all explanatory "obsolete" notes (mine) or auto-generated rustdoc for
  the `ManifestV2` Rust type that still exists in source (not the on-disk
  format). The CG `07-packaging.md:72` `package_type` mention is the correct
  deprecation note — left intact.

All acceptance criteria met. Follow-on CLI work (T1 `package new`, T2/T-0665
`package pack` for Python, T3 `package validate`) remains under I-0119.