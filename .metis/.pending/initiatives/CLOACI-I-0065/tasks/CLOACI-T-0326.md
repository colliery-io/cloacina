---
id: update-integration-tests-soak-test
level: task
title: "Update integration tests, soak test, and demos for fidius source packages"
short_code: "CLOACI-T-0326"
created_at: 2026-04-01T12:34:39.527654+00:00
updated_at: 2026-04-01T22:54:30.864770+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Update integration tests, soak test, and demos for fidius source packages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0065]]

## Objective

Update all integration tests, the daemon soak test, and demo programs to work with fidius source packages. The soak test should use real compilable Rust source packages instead of dummy archives that always fail to load.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `dal/workflow_registry.rs` — `create_package_from_prebuilt_so()` replaced with source package creation
- [ ] `dal/workflow_registry_reconciler_integration.rs` — same
- [ ] `registry_workflow_registry_tests.rs` — same
- [ ] `packaging.rs` — tests use `pack_package()` not gzip tar
- [ ] `packaging_inspection.rs` — tests validate `package.toml` not `manifest.json`
- [ ] Soak test (`soak.py`) creates real compilable `.cloacina` source packages (Rust source + `package.toml`)
- [ ] Soak test verifies packages actually compile and load (not just "didn't crash")
- [ ] `angreal cloacina integration` builds source packages instead of pre-compiled dylibs
- [ ] `registry-execution` demo uses source packages
- [ ] All demos pass (`./run_demos.sh`)
- [ ] All CI checks pass
- [ ] Depends on T-0320 through T-0325 (all format changes complete)

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
