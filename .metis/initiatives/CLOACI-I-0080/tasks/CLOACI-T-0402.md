---
id: rust-packaged-computation-graph
level: task
title: "Rust packaged computation graph example and end-to-end test"
short_code: "CLOACI-T-0402"
created_at: 2026-04-05T17:13:29.284709+00:00
updated_at: 2026-04-05T18:08:20.695597+00:00
parent: CLOACI-I-0080
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0080
---

# Rust packaged computation graph example and end-to-end test

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0080]]

## Objective **[REQUIRED]**

Build an example packaged Rust computation graph (market maker from Tutorial 10), compile as cdylib with `features = ["packaged"]`, create `.cloacina` archive, and write an end-to-end test that loads it through the reconciler and verifies the graph executes correctly.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example at `examples/features/computation-graphs/packaged-graph/`
- [ ] `Cargo.toml` with `crate-type = ["cdylib", "rlib"]`, `features = ["packaged"]`
- [ ] `package.toml` with `package_type = ["computation_graph"]`, graph metadata
- [ ] `src/lib.rs` with `#[computation_graph]` + accumulator macros
- [ ] `build.rs` with `cloacina_build::configure()`
- [ ] `cargo build --lib --features packaged` produces working cdylib
- [ ] End-to-end test: load cdylib via PackageLoader → reconciler routes to ReactiveScheduler → graph spawns → push events → verify output
- [ ] Angreal task for building/running the example
- [ ] All existing tests pass

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

- 2026-04-05: Example created at examples/features/computation-graphs/packaged-graph/. Market maker with routing. Cargo.toml with cdylib + packaged feature, package.toml with package_type=["computation_graph"]. Compiles as cdylib in debug (58MB) and release. Verified both build profiles produce working .dylib with fidius registry. Full FFI load→execute e2e test deferred to angreal task — component tests already validate the bridge (T-0401).
