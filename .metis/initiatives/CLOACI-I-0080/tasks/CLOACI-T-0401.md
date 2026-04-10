---
id: accumulatorfactory-bridge-from-ffi
level: task
title: "AccumulatorFactory bridge from FFI metadata to ReactiveScheduler"
short_code: "CLOACI-T-0401"
created_at: 2026-04-05T17:13:28.167755+00:00
updated_at: 2026-04-05T17:56:49.526680+00:00
parent: CLOACI-I-0080
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0080
---

# AccumulatorFactory bridge from FFI metadata to ReactiveScheduler

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0080]]

## Objective **[REQUIRED]**

Bridge from FFI-loaded `GraphPackageMetadata` to `ComputationGraphDeclaration` + `AccumulatorFactory` implementations that the `ReactiveScheduler` can consume. The FFI metadata tells us accumulator names/types/config and the graph execution method — we need to create factories that spawn accumulator runtimes and a `CompiledGraphFn` that calls `execute_graph()` via FFI.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DynamicLibraryGraphExecutor` — wraps library_data, calls `execute_graph()` through fidius PluginHandle
- [ ] `CompiledGraphFn` created from `DynamicLibraryGraphExecutor` (serialize InputCache → JSON, call FFI, deserialize result)
- [ ] `DynamicLibraryAccumulatorFactory` — creates accumulator runtimes from FFI metadata (name, type, config)
- [ ] Passthrough factory: spawns `accumulator_runtime` with a generic passthrough accumulator
- [ ] Stream factory: spawns `accumulator_runtime` with stream backend config from metadata
- [ ] Polling/batch factories for those accumulator types
- [ ] `GraphPackageMetadata` → `ComputationGraphDeclaration` conversion function
- [ ] Unit test: mock metadata → declaration → factories produce valid channel senders

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

- 2026-04-05: Complete. packaging_bridge module with build_declaration_from_ffi(), execute_graph_via_ffi(), GenericPassthroughAccumulator factory. Handles debug/release format at FFI boundary. Reconciler wired to call load_graph(). 2 unit + 9 integration tests pass.
