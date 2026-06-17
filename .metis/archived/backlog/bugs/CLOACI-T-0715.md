---
id: kafka-stream-accumulator-receives
level: task
title: "Kafka stream accumulator receives messages but never fires its reactor (E2E delivery gap)"
short_code: "CLOACI-T-0715"
created_at: 2026-06-16T19:17:53.917358+00:00
updated_at: 2026-06-16T19:45:33.594868+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Kafka stream accumulator receives messages but never fires its reactor (E2E delivery gap)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

A Kafka **stream accumulator** consumes messages from its topic but never emits
a boundary that fires its bound reactor — so a reactor-bound computation graph
fed by Kafka never runs. Found while verifying Kafka end-to-end in the
docker-compose demo (CLOACI-I-0124 / WS-11). Kafka stream accumulators appear to
have never been exercised end-to-end (the soak explicitly skips Kafka — see
`.angreal/test/soak/server.py` header), so this is a latent gap, not a
regression.

Note: the *prerequisite* broker-resolution bug (the accumulator failed to even
init because `{{ KAFKA_BROKER }}` wasn't resolved) was fixed in WS-11 (commit
`7a42a890`). This ticket is the **next** layer: it now connects and consumes,
but doesn't drive the reactor.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected**: any reactor-bound computation graph whose entry accumulator is a
  Kafka `stream` source (e.g. the `demo-kafka-stream-rust` fixture's
  `demo_kafka_graph`). Socket/passthrough-fed graphs are unaffected (verified
  firing on the host stack).
- **Reproduction**:
  1. `docker compose -f docker/docker-compose.demo.yml up -d --build` (now seeds
     + runs the `producer` service which produces to `demo.kafka.stream`).
  2. Server logs: `Kafka stream backend connected` + `Kafka event source started`
     (kafka_alpha), and with `RUST_LOG=info,cloacina::computation_graph=debug`,
     **494 "Kafka message received"** in ~35s.
  3. Yet `grep -c "graph execution completed.*demo_kafka_rx"` = **0**, and the
     `kafka_alpha` component health metric stays `state="starting"`.
- **Expected**: each consumed Kafka message → accumulator emits a boundary →
  `demo_kafka_rx` reactor fires `demo_kafka_graph`.

### Evidence / where it breaks
- `recv()` works: `accumulator_runtime_inner`'s event-source task (packaging_bridge
  `KafkaEventSource::run`) logs "Kafka message received" 494×, forwarding payloads
  to the merge channel.
- The processor loop (`accumulator.rs` ~462) reads `event_rx`, calls
  `GenericPassthroughAccumulator::process` (returns `Some(raw_bytes)`), and emits
  via `ctx.output.send(&Vec<u8>)`. **No "boundary send failed" errors** — so emit
  reportedly succeeds, yet the reactor never fires.
- Two suspects to investigate:
  1. **Boundary encoding mismatch:** the passthrough emits `Vec<u8>` and
     `BoundarySender::send` re-serializes it, so the reactor may receive a
     double-encoded payload it can't deserialize into the node input
     (`EventData`). (But the socket/passthrough path fires on the host, so
     confirm whether socket accumulators use the same `GenericPassthroughAccumulator`
     or a typed one — if typed, the generic stream passthrough is the divergence.)
  2. **Boundary→reactor wiring:** the stream accumulator's `boundary_tx` may not
     be the channel the reactor's aggregator listens on (spawn/bind ordering),
     so emitted boundaries go nowhere.
- Secondary defect: the stream-accumulator runtime sets health `Connecting`
  (`accumulator.rs` ~420) and **never advances to `Live`** — so even a working
  Kafka accumulator shows `starting`/`socket_only`-ish health forever. Add a
  `set_health(Live)` once the source is delivering.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A Kafka stream accumulator that receives messages emits boundaries that
      fire its bound reactor — verified: `demo_kafka_rx` fires continuously
      (1243 fires) on the `demo.kafka.stream` feed in the compose demo.
- [x] Stream-accumulator health advances to `Live` — `kafka_alpha` now reports
      `component_health …state="healthy" 1`.
- [x] Regression test `test_stream_accumulator_reaches_live` (asserts a
      stream/event-source accumulator advances Connecting → Live).

## Root cause (corrected)

Not the boundary encoding. The reactor's **startup health-gate** (`reactor.rs`:
"Wait until all accumulators are healthy") only proceeds once every bound
accumulator reports `Live`/`SocketOnly`. The stream/event-source accumulator
path (`accumulator_runtime_inner`) set health to `Connecting` and **never
advanced** — so a Kafka-fed reactor gated in startup forever: the consumer
pulled messages and the accumulator emitted boundaries (494 received, verified
via debug logs) but the reactor never reached its boundary-processing loop → 0
fires. Socket-fed graphs worked because the socket path reports `SocketOnly`
immediately. Fix: advance stream accumulators `Connecting → Live` once the
source task is spawned (commit `bc547136`); regression test `eb8fe522`.

## Status Updates **[REQUIRED]**

**2026-06-16 — Filed (CLOACI-I-0124 / WS-11).** Found verifying Kafka in the
compose demo. Broker-resolution prerequisite fixed (`7a42a890`); this is the
remaining consume→fire gap. Full evidence above. Deferred from WS-11 as a
focused engine fix (needs iterative server rebuilds in compose to test).

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