---
id: delete-old-archive-code-gzip-tar
level: task
title: "Delete old archive code — gzip tar, manifest.json, ManifestV2, dylib extraction, flate2"
short_code: "CLOACI-T-0324"
created_at: 2026-04-01T12:34:25.740130+00:00
updated_at: 2026-04-01T12:34:25.740130+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Delete old archive code — gzip tar, manifest.json, ManifestV2, dylib extraction, flate2

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0065]]

## Objective

Delete all code related to the old gzip tar + manifest.json + compiled dylib archive format. This is the cleanup task after T-0320 through T-0323 have replaced all callers.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Delete `archive.rs` (gzip tar creation via flate2)
- [ ] Delete `compile.rs` or gut it (pre-compilation moved to reconciler)
- [ ] Delete `manifest_schema.rs` types: `Manifest`, `ManifestV2`, `PackageInfo`, `RustRuntime`, `PythonRuntime`, `PackageLanguage`
- [ ] Delete `manifest_v2.rs` if it exists
- [ ] Remove `flate2` from Cargo.toml dependencies
- [ ] Remove `tar` from cloacina Cargo.toml if no longer used (fidius-core handles tar)
- [ ] `grep -r "manifest.json" crates/` returns zero results (except docs/comments)
- [ ] `grep -r "GzDecoder\|flate2" crates/` returns zero results
- [ ] All tests pass
- [ ] Depends on T-0320, T-0321, T-0322, T-0323 (all callers migrated first)

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
