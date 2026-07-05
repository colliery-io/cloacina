---
id: python-computation-graphs-on-the
level: task
title: "Python computation graphs on the agent fleet — agent-side graph-fn assembly from source (T-0722 follow-on)"
short_code: "CLOACI-T-0841"
created_at: 2026-07-05T21:54:59.701905+00:00
updated_at: 2026-07-05T21:54:59.701905+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Python computation graphs on the agent fleet — agent-side graph-fn assembly from source (T-0722 follow-on)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Extend T-0722's whole-graph fleet dispatch to PYTHON-packaged computation graphs. Today `FleetGraphExecutor` detects `language == "python"` and falls back in-process (cleanly, logged): a py CG's compiled graph fn is a live PyObject assembled by the cloaca graph-builder machinery (node registrations + topology + dispatcher composition) inside the scheduler's load path — not a shippable artifact.

The model is the same source-shipping approach Python TASKS already use on the fleet (T-0716): PyObjects never cross a boundary — the agent fetches the source archive by digest, imports it in ITS interpreter, and materializes its own objects. What's missing is a standalone "import module → assemble JUST the graph fn (no accumulators/reactor wiring) → call with the FFI cache" path the agent can run per firing:

1. Factor the py graph-fn assembly out of the scheduler-coupled load path (cloacina-python graph builder) into a callable that takes (staged module, graph name) → graph fn.
2. Agent `process_graph_packet`: when `language == "python"`, fetch the SOURCE archive (the `/v1/agent/source/{digest}` path tasks use), stage + import (the T-0840 eviction applies), assemble the graph fn, convert the packet cache into the py-side input shape, execute, report.
3. Server: drop the python early-fallback and dispatch (packet gains `language`, mirroring `WorkPacket`).
4. Verify live: `demo_py_graph` fires on an agent.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (scaling parity; py CGs run correctly in-process today via T-0722's fallback)

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
