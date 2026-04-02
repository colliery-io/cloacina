---
id: update-filesystemworkflowregistry
level: task
title: "Update FilesystemWorkflowRegistry to detect fidius source packages"
short_code: "CLOACI-T-0325"
created_at: 2026-04-01T12:34:33.765541+00:00
updated_at: 2026-04-01T12:34:33.765541+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Update FilesystemWorkflowRegistry to detect fidius source packages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0065]]

## Objective

Update `FilesystemWorkflowRegistry` to scan for fidius source packages (`.cloacina` bzip2 tar files) and extract metadata from `package.toml` via `load_manifest::<CloacinaMetadata>()`. Also update the reconciler's `package.rs` helpers.

## Acceptance Criteria

- [ ] `scan_packages()` detects `.cloacina` files and unpacks to read `package.toml`
- [ ] Metadata extracted from `CloacinaMetadata` (workflow_name, version, language) instead of `manifest.json`
- [ ] `is_cloacina_package()` / `extract_so_from_cloacina()` in `package.rs` deleted
- [ ] Package fingerprint from `fidius_core::package::package_digest()` instead of manifest field
- [ ] Daemon watcher still triggers reconciliation on `.cloacina` file changes
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
