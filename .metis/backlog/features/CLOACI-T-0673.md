---
id: expose-visualize-computation-graph
level: task
title: "Expose + visualize computation-graph node/edge topology (CG DAG) in the UI"
short_code: "CLOACI-T-0673"
created_at: 2026-06-13T14:21:49.702805+00:00
updated_at: 2026-06-13T14:55:08.494805+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Expose + visualize computation-graph node/edge topology (CG DAG) in the UI

## Objective **[REQUIRED]**

Let users **see the node/edge DAG of a defined computation graph** in the UI, like
the workflow task DAG (T-0663). Today CGs are listed (`GET /v1/health/graphs` →
name + accumulators + health; e.g. `mixed_graph`, `demo_py_graph`) and
GraphDetail shows accumulators, but the **compute-node topology is not exposed
anywhere** — it's baked into the compiled graph closure and discarded after load.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: see what a computation graph actually computes (its compute
  nodes + edges), not just its accumulators — parity with the workflow DAG view.
- **Effort Estimate**: **L** — multi-crate + FFI. The CG topology is not available
  host-side today (verified: `RunningGraph`/`ComputationGraphDeclaration` hold only
  accumulators + mode; `GraphPackageMetadata` has no `graph_data_json`; the DAG is
  compiled into `CompiledGraphFn`). Requires new extraction plumbing.

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

## Acceptance Criteria **[REQUIRED]**

- [x] CG node/edge topology is extracted from the package (Rust via macro→FFI,
      Python via the graph executor) and exposed via `GET /v1/health/graphs/{name}`
      (`topology: {nodes, edges}`).
- [x] The UI GraphDetail renders the CG as an interactive DAG (shared `Dag`
      React Flow component), alongside accumulators/health.
- [x] Verified on `mixed_graph` (Rust: compute→output) and `demo_py_graph`
      (Python: decision routing to signal_handler[Trade] / audit_logger[NoAction])
      in Chromium — DAGs render with labeled routing edges, zero console errors.

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
Mirror the workflow `graph_data` pattern for CGs. Topology source-of-truth:
- **Rust**: the macro IR (`cloacina-macros/src/computation_graph/graph_ir.rs`
  `GraphIR` — nodes + edges, Linear/Routing) is known at compile time. Emit it as
  JSON and expose via a new fidius method (e.g. `get_graph_topology()`), or add a
  `graph_data_json`/`nodes`/`edges` field to `GraphPackageMetadata`
  (`cloacina-workflow-plugin/src/types.rs:93`).
- **Python**: `PythonGraphExecutor` (`cloacina-python/src/computation_graph.rs:
  ~564-603`) holds `node_map` + `execution_order` host-side — serialize to the
  same JSON shape when building the declaration (`packaging_bridge.rs`).

Then thread it through:
1. Capture topology at CG load (reconciler `step_load_reactor_bound_cgs` /
   triggerless graph load) and retain it on the scheduler's `RunningGraph`
   (`computation_graph/scheduler.rs:224`) or a side registry the API can read.
2. api-types: extend graph detail (`health.rs` `GraphStatus`) with a `topology`
   (nodes + edges), or add `GET /v1/health/graphs/{name}/topology`.
3. routes `health_graphs.rs` populate it; OpenAPI + SDK regen.
4. UI: GraphDetail renders the DAG via the shared React Flow component (generalize
   `WorkflowGraph` → a `Dag` component taking nodes+edges).

### Dependencies
- Reuses the React Flow DAG component from T-0663.
- Independent of T-0672 (Python tasks) but shares the "expose topology" theme.

### Risk Considerations
- FFI surface change (new method / new metadata field) — ABI version bump +
  rebuild of all packages; keep `#[serde(default)]` for backward compat.
- Routing edges (conditional) need a representation the UI can draw (label edges).
- Decide endpoint shape (extend detail vs separate topology route) before SDK regen.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented (Rust + Python), rebuild in progress.**
Topology JSON shape: `{nodes:[{id,inputs}], edges:[{from,to,label}]}` (label =
routing variant, null for linear). Threaded end-to-end:
- **Producer (Rust):** `cloacina-macros` codegen `graph_topology_json(ir)` serializes
  the GraphIR; emitted as a literal in every `ComputationGraphEntry.graph_data_json`.
  `ComputationGraphEntry` + `GraphPackageMetadata` gained `graph_data_json`; the FFI
  `get_graph_metadata` shell passes it through.
- **Producer (Python):** `build_python_graph_declaration` serializes the
  `PythonGraphExecutor` node_map via `python_graph_topology_json` → `decl.topology`.
- **Carry:** `ComputationGraphDeclaration` gained `topology`; `build_declaration_from_ffi`
  sets it from `graph_data_json`. Scheduler keeps a `graph_topologies: graph→JSON`
  map (set in `load_graph`, removed in `unbind_graph_from_reactor`); scheduler
  `GraphStatus` gained `topology`, populated in `list_graphs`.
- **Expose:** api-types `GraphStatus.topology: Option<GraphTopology>` (+ new
  `GraphTopology`/`GraphTopologyNode`/`GraphTopologyEdge`); route `health_graphs.rs`
  parses the JSON into it; registered in openapi.rs.
- **SDK/UI:** openapi.json + TS types updated (SDK rebuilt, UI build clean).
  Extracted a shared `Dag` component (React Flow + dagre); `WorkflowGraph` delegates
  to it; `GraphDetail` renders the CG DAG (labeled edges for routing variants).
Remaining: verify on the demo stack (mixed_graph Rust + demo_py_graph Python) in
Chromium. Rust topology requires the fixtures recompiled by the cloacina-compiler
with the new macro → fresh DB rebuild (in progress).
