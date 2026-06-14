---
id: package-validate-check-a-package
level: task
title: "package validate — check a package (dir or archive) against the canonical format"
short_code: "CLOACI-T-0679"
created_at: 2026-06-14T16:29:25.209502+00:00
updated_at: 2026-06-14T16:33:27.382410+00:00
parent: CLOACI-I-0119
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0119
---

# package validate — check a package (dir or archive) against the canonical format

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0119]]

## Objective **[REQUIRED]**

T3 of CLOACI-I-0119. Add `cloacinactl package validate <path>` to check a
package against the canonical format **without uploading** — accepting either a
source directory or a packed `.cloacina` archive (unpacked to a temp dir first).
Surfaces the same problems the server rejects: the closed `[metadata]` schema
(unknown keys, `package_type`, `[[metadata.triggers]]`, missing `language`) plus
the language-specific layout (`workflow/` + `entry_module` for Python;
`Cargo.toml` + `src/lib.rs` for Rust). Reuses the `manifest.rs` helpers added in
T-0665.

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

- [x] `cloacinactl package validate <path>` accepts a source dir OR a `.cloacina`
      archive (archive unpacked to a temp dir, then validated).
- [x] Validates the closed `[metadata]` schema (rejects `package_type`,
      `[[metadata.triggers]]`, unknown keys; requires `language`).
- [x] Python: requires `workflow/` + `entry_module` resolving under it. Rust:
      requires `Cargo.toml` + `src/lib.rs` (warns on missing `build.rs`).
- [x] Prints an `ok: <name> <version> (<lang>)` summary on success; a specific
      error otherwise. Exits non-zero on failure.
- [x] Unit tests: good Python dir, missing-`workflow/`, `package_type` rejection,
      missing path.

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

**2026-06-14 — Implemented; verification pending.** Added
`crates/cloacinactl/src/nouns/package/validate.rs` + the `Validate` verb.
Refactored `manifest.rs` to expose `read_manifest` (full `[package]`+`[metadata]`)
and added `validate_rust_layout`; `validate` dispatches on `[metadata].language`
and reuses `validate_python_layout`. Archive inputs are unpacked via
`fidius_core::package::unpack_package` into a temp dir before validation. Unit
tests added. **Pending:** `angreal check crate crates/cloacinactl` + a smoke
(`package new` → `package validate` on the dir and on the packed archive).