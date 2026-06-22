---
id: graph-node-code-on-click-source
level: task
title: "Graph node code-on-click — source_package on graph health + modal-ify the CG node viewer"
short_code: "CLOACI-T-0773"
created_at: 2026-06-22T14:36:39.923583+00:00
updated_at: 2026-06-22T14:59:22.080312+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Graph node code-on-click — source_package on graph health + modal-ify the CG node viewer

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Two UI gaps on the Graph detail view: (1) clicking accumulator/reactor/CG nodes
never shows code (the slide-out says "source isn't shipped" — wrong: graph source
IS retained, just under the *package* name, not the graph name); (2) the node
inspector is a slide-out Drawer, inconsistent with the workflow task code Modal.

## Plan

**Backend** — graph health has no graph→package mapping; the scheduler's GraphStatus
and even ComputationGraphDeclaration carry no package name. Resolve at request time
from the workflow registry (already scans packages by declared surface):
- registry: `find_package_for_surface(kind, name) -> Option<package_name>`.
- api-types `GraphStatus`: add `source_package: Option<String>`.
- `get_graph` handler resolves via the graph's reactor surface (g.reactor) →
  package; `list_graphs` leaves it None (detail-only). All demo graph packages
  declare typed boundary surfaces, so they resolve.

**SDK** — regen TS types + Python `_generated` (field addition; coverage gate
unaffected — endpoint-based).

**UI** — replace the Drawer with a Modal consistent with TaskCodeModal; fetch the
graph's `source_package` source; per-kind extraction (CG compute node = `fn <id>`
w/o the #[task] requirement; reactor = `#[reactor(...)] struct`; accumulator =
best-effort decl / full file at the mention). Keep the role/routing rows.

## Status Updates **[REQUIRED]**

- 2026-06-22: Scoped. User chose the proper backend-field approach over a UI
  heuristic. Registry-scan resolution avoids deep scheduler plumbing.
- 2026-06-22: Implemented + compiles. Backend: `find_package_for_surface` on the
  registry; `source_package` on api-types GraphStatus; `get_graph` resolves it via
  reactor/accumulator surface (`list_graphs` leaves None). cloacina + server cargo
  check green. SDK: re-emitted openapi.json (source_package present), regen TS
  types + Python `_generated`; coverage gate 38/38, all 3 SDKs build, imports
  resolve. UI: new `GraphNodeModal` (metadata rows + per-kind source extraction:
  compute fn / `#[reactor]` block / full-file fallback) replaces the Drawer;
  typecheck + build green. Rebuilding server+ui demo containers to verify live.
- 2026-06-22: DONE (35d41780). Verified live on the demo: `combine` node opens the
  modal showing `pub async fn combine` + Inputs/Upstream/Routes; the reactor node
  shows its `#[reactor(name=…, accumulators=…, criteria=…)] pub struct` block +
  criteria/strategy/role. `source_package` resolves to `demo-pipeline-rust` on the
  graph API. Accumulator nodes have no standalone decl → fall to full-file at the
  mention (still reachable, the original complaint). Both asks met: code shows,
  consistent modal. Committed + pushed.

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