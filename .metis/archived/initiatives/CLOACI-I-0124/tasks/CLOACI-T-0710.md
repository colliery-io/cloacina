---
id: ws-8-expand-demo-fixtures-to
level: task
title: "WS-8 — Expand demo fixtures to richer example graphs"
short_code: "CLOACI-T-0710"
created_at: 2026-06-16T01:50:20.786945+00:00
updated_at: 2026-06-16T04:29:23.671785+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-8 — Expand demo fixtures to richer example graphs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P2, **demo-stack — not UI code**) Expand the demo fixtures so the UI has rich
structure to render. Today's seed packs thin graphs (2–3 nodes); the example library
has much richer ones.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Demo fixtures include richer example computation graphs: `09-full-pipeline` (multi-source `when_any`), `10-routing` (enum dispatch), `08-accumulators`, plus a multi-task workflow and the `mixed-rust` reactor+trigger combo.
- [ ] The demo harness packs/uploads/builds them cleanly (pending → success) on `docker compose -f docker/docker-compose.demo.yml up`.
- [ ] Graphs/workflows/triggers views show meaningful structure (branches, fan-in, non-cron trigger) for UAT.

## Dependencies

Enables good UAT for the UI workstreams; supports [[CLOACI-T-0708]]. Touches
`docker/`, `ui/harness/`, demo fixtures — not the SPA.

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

### 2026-06-16 — DONE (richer CG fixtures: fan-in + routing, built + verified)

Ported the two richest tutorial graphs into packable cdylib fixtures (the
tutorials ship as `src/main.rs` binaries with no `package.toml`, so each had to
be turned into a `package!()` cdylib modeled on demo-kafka-stream-rust, dropping
the standalone accumulator-runtime/`main()` wiring):

- **demo-pipeline-rust** (graph `market_pipeline`, from `09-full-pipeline`) —
  TWO accumulators (`orderbook`, `pricing`) fan into one reactor on `when_any`,
  then a three-node pipeline `combine → evaluate → signal`.
- **demo-routing-rust** (graph `market_maker`, from `10-routing`) — a `decision`
  node using enum `=>` dispatch to two labeled branches: `Trade → signal_handler`,
  `NoAction → audit_logger`.

Both added to `DEMO_FIXTURES`. `angreal ui build-fixtures` packs them; on seed
the compiler builds both to **`build_status: success`** and the reconciler loads
both — `/v1/health/graphs` now lists `market_pipeline` + `market_maker` beside
`demo_kafka_graph`.

Verified live (`ui/e2e/ws8.spec.ts`, screenshots in `/tmp/cloacina-ui-uat/ws8/`):
`market_pipeline` renders the two-source **fan-in**; `market_maker` renders the
**decision node branching** with `Trade` / `NoAction` edge labels — real
structure for UAT.

**Acceptance coverage:**
- Richer CGs (multi-source `when_any` fan-in + enum routing) ✓.
- Pack/upload/build clean (pending → success) ✓.
- Graphs view shows branches + fan-in ✓; non-cron trigger shown via WS-6's
  demo-poll-rust ✓; multi-task workflow already present (demo-slow-rust, 5
  chained tasks) ✓.
- **Deferred (with reason):** `08-accumulators` (its multi-accumulator structure
  is already demonstrated by `market_pipeline`'s fan-in) and `mixed-rust` — the
  latter's `Cargo.toml` uses relative `../../../crates` paths (not
  `__WORKSPACE__`) for its integration-test usage, so it can't be demo-packed
  without edits that would break those tests. The reactor+trigger combo it would
  add is already covered across demo-kafka (reactor+CG) + demo-poll (poll
  trigger).

Committed `8fd6d2ff` on `feat/ui-0124-server-read-endpoints`.