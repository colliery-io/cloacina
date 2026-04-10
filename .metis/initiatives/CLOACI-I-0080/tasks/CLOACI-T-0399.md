---
id: computation-graph-macro-packaged
level: task
title: "Computation graph macro packaged codegen — FFI plugin impl under cfg(feature = packaged)"
short_code: "CLOACI-T-0399"
created_at: 2026-04-05T17:13:25.701199+00:00
updated_at: 2026-04-05T17:37:50.390155+00:00
parent: CLOACI-I-0080
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0080
---

# Computation graph macro packaged codegen — FFI plugin impl under cfg(feature = "packaged")

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0080]]

## Objective **[REQUIRED]**

Update the `#[computation_graph]` macro codegen to emit an `_ffi` module under `#[cfg(feature = "packaged")]`, parallel to what `#[workflow]` does. The FFI module implements `CloacinaPlugin` with `get_graph_metadata()` returning accumulator declarations + topology, and `execute_graph()` running the compiled graph in the plugin's own tokio runtime. Also update accumulator macros to emit metadata that `get_graph_metadata()` can collect.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[computation_graph]` generates `_ffi` module under `#[cfg(feature = "packaged")]`
- [ ] FFI module implements `CloacinaPlugin` via `#[plugin_impl]`
- [ ] `get_graph_metadata()` returns `GraphPackageMetadata` with graph name, accumulators, reaction mode
- [ ] `execute_graph(request)` deserializes cache JSON, calls compiled graph fn in `OnceLock<Runtime>`, returns JSON result
- [ ] `fidius_plugin_registry!()` emitted
- [ ] Accumulator macros (`#[passthrough_accumulator]`, `#[stream_accumulator]`, `#[polling_accumulator]`, `#[batch_accumulator]`) emit static metadata for collection by `get_graph_metadata()`
- [ ] Compilation with `--features packaged` produces working cdylib
- [ ] Compilation without `packaged` feature still works (embedded mode unchanged)

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

- 2026-04-05: Complete. _ffi module generated under cfg(feature="packaged") with full CloacinaPlugin impl. get_graph_metadata() returns graph name + accumulator names + reaction mode. execute_graph() builds InputCache from JSON, runs compiled fn in OnceLock<Runtime>, serializes terminal outputs. Embedded #[ctor] path gated on cfg(not(feature="packaged")). Verified: test cdylib builds (56MB .dylib), 9 integration tests pass, packaged workflow examples still compile. Accumulator type metadata comes from macro's react declaration (names only) — full type/config from package.toml.
