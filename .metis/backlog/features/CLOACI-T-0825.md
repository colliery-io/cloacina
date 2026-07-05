---
id: seed-built-in-constructors-one-per
level: task
title: "Seed built-in constructors (one per primitive: task/trigger/accumulator/reactor)"
short_code: "CLOACI-T-0825"
created_at: 2026-06-28T23:57:44.661370+00:00
updated_at: 2026-07-05T01:26:06.416792+00:00
parent: CLOACI-I-0132
blocked_by: [CLOACI-T-0834]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Seed built-in constructors (one per primitive: task/trigger/accumulator/reactor)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Ship a **seed built-in library**: >=1 constructor per primitive — e.g. an http-poll **trigger**, a windowed **accumulator**, a **reactor** with firing criteria, and a shell/http **task** — authored via the macros, compiled to WASM components.

**AC:** each ships as a loadable constructor, instantiable with config, runnable end-to-end in a sample workflow; covered by tests. Blocked by CLOACI-T-0824.

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

### 2026-07-04 — DONE (branch feat/i0132-completion, commit 3ede7654)
Seed library shipped under `examples/constructor-contract/` — one provider per primitive, each a single-member A-0011 suite (multi-kind-per-component stays the noted fidius unknown):
- **task**: `cloacina-provider-fs` (`read_file`/`write_file`) — pre-existing from T-0834, counts as the task seed; e2e-proven by `constructor_provider_package_wasm` + fs-grant-demo + the live stack.
- **trigger**: `cloacina-provider-sensor` / `file_present` — the Airflow-FileSensor analog; fires when a configured path exists INSIDE the sandbox, so it's grant-gated (no `fs` grant → the path is invisible → fails closed by never firing).
- **accumulator**: `cloacina-provider-extract` / `extract` — projects a configured field from each event into the boundary; buffers events without it.
- **reactor**: `cloacina-provider-quorum` / `quorum` — fires when ≥ `required` boundaries are held (N-of-M criteria).

**Design constraint discovered**: the authoring model REBUILDS the instance from bound config per call (config_binds in the macro), so seed accumulators must be STATELESS transforms — cross-event windowing (count windows etc.) needs runtime-held state and is a follow-on. The originally-sketched "http-poll trigger / shell task" need wasi-http/exec surfaces — the fs/file-based seeds exercise the same grant machinery without new host surface.

**Tests**: `constructor_seed_library_wasm.rs` packages all three via the real `package_constructor_provider` path and drives poll/ingest/evaluate through wasmtime — 3/3 green, including the sensor's default-closed no-grant case. AC met: loadable, config-instantiable, end-to-end runnable, tested (the task member additionally runs in real workflows via fs-grant-demo + the demo stack).
