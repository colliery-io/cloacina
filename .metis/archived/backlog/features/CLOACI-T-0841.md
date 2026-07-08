---
id: python-computation-graphs-on-the
level: task
title: "Python computation graphs on the agent fleet — agent-side graph-fn assembly from source (T-0722 follow-on)"
short_code: "CLOACI-T-0841"
created_at: 2026-07-05T21:54:59.701905+00:00
updated_at: 2026-07-05T22:24:11.174335+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-07-05 — implementation (branch feat/t0841-py-cg-fleet); code-complete
The feared "scheduler-coupled assembly" mostly didn't exist: a py CG's graph fn is `get_graph_executor(name)` (a process-global registry populated as an import side effect) + `PythonGraphExecutor::execute(&InputCache)` — no factoring needed. The agent's existing Python-task load path (`fetch_source_archive` + `stage_agent_providers` + `python_runtime().load_workflow_package`, so T-0716/T-0838/T-0840 all apply) provides the import verbatim.
- **Protocol**: `GraphWorkPacket.language` (serde-default, mirrors `WorkPacket.language`).
- **Server**: python early-fallback removed; interpreted packages get `runnable_triples = None` (any arch, same as tasks) + primary/source digest; `language` stamped into the packet.
- **Agent**: `execute_python_graph` — one import per digest (`imported_py_graph_digests`; executors are global, upgrades arrive as new digests where the T-0840 eviction re-executes the module), then `get_graph_executor(packet.graph_name)` and execute with the packet cache REBUILT into the exact in-process wire shape (`bincode(Vec<u8>)` of the raw JSON per entry — byte-identical to server-side accumulator frames, so executor behavior matches a local firing exactly). Outputs reported empty (parity: in-process py graph outputs are discarded). Executor-not-found after import → Refused → server falls back in-process.
Remaining: tests, live verify demo_py_graph firing on an agent, PR.

### 2026-07-05 — 🎯 LIVE-PROVEN, two real fixes found by the live loop — CLOSING
The live verification loop caught two design gaps unit checks wouldn't have (each time the T-0722 fallback correctly ran the firing in-process — the safety policy proving itself):
1. **Package resolution**: py CG packages don't declare a findable REACTOR surface — added the accumulator-surface fallback (resolve via the firing's own snapshot sources), mirroring the health API's `source_package` logic.
2. **Reactor≠graph naming**: the packet carries the REACTOR name; `GRAPH_EXECUTORS` keys by CG name (reactors fan to subscriber graphs — Rust worked only because FFI `execute_graph` ignores the name). Added `get_graph_executors_for_reactor` (cloacina-python) + agent-side fan-out executing every subscriber graph (any error fails the firing, matching the in-process dispatcher's aggregate).

**LIVE PROOF**: inject `{"value": 7.5}` into `py_alpha` → `demo_py_graph_rx` fired → **"fleet: graph firing completed on agent agent_id=08a8642e… duration_ms=140"** (including first-time source import) → fires log `ok: true`, inputs decodable. CG suite 45/45 unchanged. CG fleet dispatch is now language-complete: Rust ships the cdylib, Python ships the source. COMPLETE.