---
id: python-computation-graph-packages
level: task
title: "Python computation-graph packages build but their graph/reactor/accumulators never register in the server health endpoints"
short_code: "CLOACI-T-0668"
created_at: 2026-06-12T21:22:09.827728+00:00
updated_at: 2026-06-13T23:48:13.146868+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Python computation-graph packages build but their graph/reactor/accumulators never register in the server health endpoints

## Objective **[REQUIRED]**

Filed 2026-06-12: Python computation-graph packages built successfully but their
graph / reactor / accumulators reportedly never showed up in the server health
endpoints (`/v1/health/graphs`, `/v1/health/accumulators`). Verify against current
code and fix if still reproducing.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Resolution: NOT REPRODUCIBLE on current code (2026-06-13)
Verified against the live demo stack, which loads the Python CG package
`demo-py-graph`:
- `GET /v1/health/graphs` → `demo_py_graph` is present, with
  `reactor: "demo_py_graph_rx"`, `accumulators: ["py_alpha"]`, and full node/edge
  topology (CLOACI-T-0673).
- `GET /v1/health/accumulators` → `py_alpha` is present.
- The 30-min server soak (CLOACI-T-0675/0676) drove ~84k WS events into
  `py_alpha` with 0 errors — the Python CG is live and consuming.
So the symptom (Python CG graph/reactor/accumulators "never register") does **not
reproduce**. Reactors are surfaced via each graph's `reactor` field; there is no
standalone `/v1/health/reactors` route (404) — if a dedicated reactor-health
endpoint is wanted, that's a separate enhancement, not a Python-CG defect.

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

## Acceptance Criteria **[REQUIRED]**

- [x] A built Python CG package registers its graph in `/v1/health/graphs`
      (verified: `demo_py_graph`).
- [x] Its accumulators register in `/v1/health/accumulators` (verified: `py_alpha`).
- [x] Its reactor is surfaced (verified: `reactor: demo_py_graph_rx` on the graph
      status).

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

**2026-06-13 — Closed: not reproducible.** Verified Python CGs register across the
health endpoints on current code (evidence above). Likely fixed by the Python
CG-loading work that landed since this was filed (2026-06-12). No code change
needed. (Stub ticket fleshed out + closed.)
