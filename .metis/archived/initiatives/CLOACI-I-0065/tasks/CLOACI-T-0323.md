---
id: update-python-loader-for-fidius
level: task
title: "Update Python loader for fidius source package format"
short_code: "CLOACI-T-0323"
created_at: 2026-04-01T12:34:19.315626+00:00
updated_at: 2026-04-01T12:34:19.315626+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Update Python loader for fidius source package format

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0065]]

## Objective

Update `python_loader.rs` to read `package.toml` (fidius format) instead of `manifest.json` (gzip tar). Python packages are source-only (no compilation needed), so the change is simpler: unpack bzip2 tar, read `package.toml` with `CloacinaMetadata`, extract `workflow/` and `vendor/` dirs.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `extract_python_package()` uses `fidius_core::package::unpack_package()` instead of gzip tar extraction
- [ ] Reads `package.toml` via `load_manifest::<CloacinaMetadata>()` instead of `manifest.json`
- [ ] `requires_python` and `entry_module` read from `CloacinaMetadata` fields
- [ ] `PackageKind::Python` detection based on `metadata.language == "python"`
- [ ] Python tutorial examples updated with `package.toml`
- [ ] `ExtractedPythonPackage` still works with rest of Python execution pipeline
- [ ] Depends on T-0319 (schema), T-0321 (loading)

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
