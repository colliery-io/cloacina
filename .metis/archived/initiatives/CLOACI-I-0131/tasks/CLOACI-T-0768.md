---
id: demo-ui-declared-injectors-end-to
level: task
title: "Demo + UI: declared injectors end-to-end (workflow params, CG boundary schemas, graph inject UI)"
short_code: "CLOACI-T-0768"
created_at: 2026-06-21T21:02:19.982916+00:00
updated_at: 2026-06-21T22:34:20.213599+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Demo + UI: declared injectors end-to-end (workflow params, CG boundary schemas, graph inject UI)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

The demo doesn't exercise injectors: every demo workflow has `declared_params=0`
and the CG accumulator/reactor input slots have empty schemas (`{}`). Make
declared injectors visible end-to-end (user picked all three surfaces):

1. **Workflow execute params** — add `#[workflow(params( … ))]` typed params to
   the demo workflow fixtures (Rust + Python). Macro auto-derives `InputSlot`s
   (schemars `schema_for::<T>()`). Surfaces in `InputsCard` + `RunWorkflowModal`.
2. **CG inject/fire boundaries** — give demo CG accumulators/reactors real typed
   boundary schemas (codegen `build_input_interface_fn` → one InputSlot per
   accumulator from the boundary type). Investigate why schemas come back `{}`
   (boundary type likely not `JsonSchema`/schemars-derived).
3. **Graph inject UI** — add per-accumulator `inject ▸` (useInjectAccumulator) +
   a `Fire ▾` force-fire / fire-with typed dialog to the new GraphDetail, driven
   by the `/interface` DeclaredSurface slots.

## Acceptance criteria
- [ ] ≥1 demo workflow declares typed params; InputsCard + Run modal show them.
- [ ] ≥1 demo CG source has non-empty inject/fire slot schemas.
- [ ] GraphDetail has per-accumulator inject + typed fire-with, validated by the
      declared interface.
- [ ] Verified live on the rebuilt demo (screenshots).

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

### 2026-06-21 — implemented (31abe892), demo rebuild in progress
Finding: demo-py-workflow ALREADY declared params (source_id/batch_size) and the
InputsCard surfaces them — so workflow-injector UI already worked; the gaps were
Rust workflows, empty CG slot schemas, and no graph inject UI.

Done:
- Part 1: demo-slow-rust now declares `#[workflow(params(source_id: String,
  batch_size: u32 = 100, dry_run: bool = false))]` (Rust parity).
- Part 2: demo-pipeline-rust OrderBookUpdate/PricingUpdate derive
  `schemars::JsonSchema` (+ dep) → typed inject/fire slot schemas. Root cause:
  boundary-type JsonSchema is opt-in (codegen.rs:222-224 SchemaProbe).
- Part 3: GraphInjectModal (typed field per declared slot from /interface);
  per-accumulator `inject ▸`; `Fire ▾` menu (force-fire / fire-with).
  controls.ts: useReactorInterface/useAccumulatorInterface + fire_with.

### 2026-06-21 — VERIFIED DONE (b4590bfd + earlier 31abe892/7373da64)
All demo workflows declare params (7373da64). Bug caught by the rebuild: declared
params ARE validated at the execute API (not pass-through) — the seed harness runs
demo_slow/demo_fail with empty context, so required params fataled seeding. Fixed
b4590bfd: auto-run workflows use defaulted params; demo-py-workflow keeps a
required source_id (not auto-run) to still show the `*` UI.

Live verification on the rebuilt demo:
- demo-slow-rust declared_params: source_id="demo-source", batch_size=100,
  dry_run=false — all typed + defaulted. Every demo workflow has params.
- orderbook /interface slot schema now typed: `OrderBookUpdate {best_bid, best_ask:
  number}` (was `{}`).
- GraphInjectModal: clicking `inject ▸` renders best_ask/best_bid NumberInputs from
  the schema; `Fire ▾` menu offers force-fire / fire-with. Screenshotted.
- Harness seeds cleanly (no crash-loop). All 4 acceptance criteria met.