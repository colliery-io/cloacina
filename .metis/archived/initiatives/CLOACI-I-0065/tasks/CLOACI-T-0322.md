---
id: add-build-on-host-compilation-to
level: task
title: "Add build-on-host compilation to reconciler for Rust source packages"
short_code: "CLOACI-T-0322"
created_at: 2026-04-01T12:34:13.219305+00:00
updated_at: 2026-04-01T12:34:13.219305+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Add build-on-host compilation to reconciler for Rust source packages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0065]]

## Objective

Add `cargo build --lib` compilation step to the reconciler's package loading flow. When a Rust source package is loaded, the reconciler unpacks source, compiles to cdylib, then loads via fidius-host. This is the core of the build-on-host model.

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

- [ ] Reconciler `load_package()` flow: unpack source -> `cargo build --lib` -> find cdylib -> `fidius_host::load_library()`
- [ ] Compilation blocks per-package (returns success/failure) but runs concurrently across packages via `spawn_blocking` or similar
- [ ] Concurrency limit on simultaneous compilations (configurable, default 2-4) — prevent cargo lock contention and CPU saturation
- [ ] Compilation happens in a managed build dir (not temp — needs to persist for the loaded dylib lifetime)
- [ ] Build artifact cleanup: compiled artifacts removed when package is unloaded or reconciler shuts down
- [ ] Compilation errors produce clear `LoaderError` with full cargo stderr output
- [ ] Compiled cdylib is used for metadata extraction + task execution (same as current post-load flow)
- [ ] Build profile configurable (debug for dev, release for production)
- [ ] `DynamicLibraryTask` still works — loads from compiled output path
- [ ] End-to-end: drop `.cloacina` source package in daemon watch dir -> compiles -> tasks register -> workflow executes
- [ ] Failed compilations don't block or crash the reconciler — logged and retried on next cycle
- [ ] Depends on T-0319, T-0320, T-0321

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
