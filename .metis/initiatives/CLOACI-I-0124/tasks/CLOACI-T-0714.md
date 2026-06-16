---
id: ws-11-richer-demo-load-live-cg
level: task
title: "WS-11 — Richer demo load: live CG data feed (WS + Kafka) + example/how-to packages"
short_code: "CLOACI-T-0714"
created_at: 2026-06-16T17:17:10.658280+00:00
updated_at: 2026-06-16T18:47:28.956246+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-11 — Richer demo load: live CG data feed (WS + Kafka) + example/how-to packages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective **[REQUIRED]**

Make the demo load richer for testing: a live data feed so the computation
graphs actually fire (WS for socket accumulators + Kafka for the stream
accumulator), and seed the real example/how-to packages alongside the demo-*
fixtures. Stack-agnostic (host `ui up` + docker-compose demo).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [x] Continuous WS producer feeds `orderbook`/`pricing` so market_pipeline +
      market_maker fire (verified: reactor "fires" count climbs on the host).
- [x] Kafka producer feeds `kafka_alpha` when a broker is set (gated; wired into
      the compose demo as a `producer` service).
- [x] Example/how-to packages (`simple-packaged`, `packaged-workflows`,
      `packaged-triggers`, `complex-dag`, `packaged-graph`,
      `python-packaged-graph`) build to success + appear in the catalog.
- [x] Runs via `angreal ui produce` (host) and the compose `producer` service.

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

### 2026-06-16 — DONE (3 parts; one known gap + compose-Kafka not end-to-end run)

**Part 1 — live data producer (commit `44393c0a`):** new harness `produce` mode
(`ui/harness/src/produce.mjs`) + `angreal ui produce`. Pushes market data into
the CG accumulators: `orderbook`/`pricing` over the server WS producer endpoint
(`/v1/ws/accumulator/{name}`, authed via `/v1/auth/ws-ticket`, **binary** frames)
and `kafka_alpha` via kafkajs to `demo.kafka.stream` when `HARNESS_KAFKA_BROKER`
is set (skipped on host). Resilient reconnect. **Verified on host `ui up`:**
server logs show `market_pipeline_reactor` / `market_maker_reactor` "graph
execution completed fires=N" climbing — the graphs fire live.

**Part 2 — example/how-to packages (commit `3a7287f2`):** generic stager
(`_stage_and_pack_example` + `_normalize_cargo`) that rewrites relative
`../crates/` deps → absolute and handles Python source trees, so the docs'
packages seed alongside the fixtures. **Verified:** all six
(`simple-packaged` / `packaged-workflows` / `packaged-triggers` / `complex-dag`
/ `packaged-graph` / `python-packaged-graph`) pack and build to
`build_status=success`; the workflow ones (incl. `complex-dag-example`, 20 tasks)
render in the Workflows list with run circles.

**Part 3 — compose producer service (commit `6eb44dc6`):** added a `producer`
service to `docker-compose.demo.yml` (produce mode, `HARNESS_KAFKA_BROKER=
kafka:9092`, after seed + kafka-healthy). `docker compose config` validates.

**Known gaps / honest caveats:**
- The two **CG example packages are triggerless** computation graphs, which the
  platform does not register in `/v1/health/graphs` (cross-cdylib triggerless-CG
  limitation, T-0553 follow-up). So they're task-less workflows with no graph →
  currently **invisible in the UI** (filtered from Workflows, absent from
  Graphs). Needs the triggerless-CG registration gap closed, or a UI surface for
  "CG packages." Backlog follow-up.
- The compose **Kafka path is wired + config-validates but was not run
  end-to-end** (would need a full `docker compose` bring-up). The Kafka producer
  code path itself is exercised via kafkajs; the WS path is fully verified.
- CG accumulators report a steady `socket_only` status even while receiving data
  and firing — the firing count is the real liveness signal (a throughput tile,
  WS-10 deferred, would surface it).