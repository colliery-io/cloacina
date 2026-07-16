---
id: stream-accumulator-supplied-by-a
level: task
title: "Stream accumulator supplied by a provider — accumulator constructor produces a stream accumulator via fidius call_streaming"
short_code: "CLOACI-T-0904"
created_at: 2026-07-15T12:09:19.200275+00:00
updated_at: 2026-07-15T12:09:19.200275+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Stream accumulator supplied by a provider — accumulator constructor produces a stream accumulator via fidius call_streaming

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Let an accumulator constructor (`kind = accumulator`) produce a STREAM accumulator (loop-owning), not only the current per-event `ingest` transform. On native, the provider's stream backend is driven via fidius `PluginHandle::call_streaming` → `ChunkStream`; a loader shim drains the stream (`.next().await`, host-pull) and pushes each item into the host accumulator boundary channel — replacing today's host-side `KafkaEventSource`/`StreamBackendAccumulatorFactory`. Load↔unload scoped to the consuming accumulator (drop the stream → producer tears down, matching `EventSource` shutdown).

**Scope:** contract `AccumulatorObject` gains a streaming variant (or a streaming member interface); `registry/loader/constructor_loader.rs` accumulator loader recognizes a stream accumulator and wires the `ChunkStream`→boundary shim; integrate with `accumulator_factory_for` (T-0896) so the `"stream"` accumulator type resolves to a provider-supplied backend instead of the host kafka branch.

**Acceptance:**
- [ ] A native provider member exposing a `call_streaming` source drives a `stream` accumulator: items flow into the CG boundary channel and fire the reactor on the demo stack.
- [ ] Shutdown/unload tears the producer down cleanly (no leaked task); backpressure is bounded (host stops pulling → producer blocks).

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0902]]/[[CLOACI-T-0903]]; builds on [[CLOACI-T-0896]]'s accumulator-type dispatch.

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

*To be added during implementation*