---
id: add-kafka-to-a-soak-compose
level: task
title: "Add Kafka to a soak compose-profile + restore stream/batch accumulator load"
short_code: "CLOACI-T-0676"
created_at: 2026-06-13T17:23:08.508997+00:00
updated_at: 2026-06-13T17:40:35.766793+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add Kafka to a soak compose-profile + restore stream/batch accumulator load

## Objective **[REQUIRED]**

When the server soak was unified onto the demo compose (CLOACI-T-0675), the
Kafka-sourced load was dropped because `docker/docker-compose.demo.yml` has no
Kafka broker. Restore Kafka coverage in the unified soak: add a Kafka service
(behind a compose profile so the plain demo stays lean) + Kafka stream/batch
computation-graph fixtures, and re-add the soak's stream/batch producer load.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: the unified soak (T-0675) exercises HTTP/WS-fed CGs but
  not Kafka stream/batch accumulators — a coverage gap vs. the old standalone
  soak (which had hand-rolled kafka CG packages, now removed).
- **Benefits of Fixing**: restores Kafka-accumulator soak coverage on the shared
  infra; Kafka CG fixtures become part of the canonical `examples/fixtures/*`.
- **Risk**: low; additive. Keep Kafka behind a compose profile so the default
  demo/CI stays lean.

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

Kafka is a **required** part of the soak (per direction 2026-06-13) — not an
optional profile. It lives in the shared stack and the soak always exercises it.

- [x] Kafka broker is a service in `docker/docker-compose.demo.yml` (KRaft,
      advertised `kafka:9092`, healthcheck); server gets `CLOACINA_VAR_KAFKA_BROKER`
      + `depends_on: kafka healthy`. No host port — producer execs in-container.
- [x] Kafka-stream CG fixture `examples/fixtures/demo-kafka-stream-rust` (reactor
      `demo_kafka_rx` + CG `demo_kafka_graph`; `package.toml` `[[metadata.accumulators]]`
      `accumulator_type="stream"` + `{{ KAFKA_BROKER }}`/topic/group; no `package_type`).
      Packed via `pack-demo-fixtures.sh`; harness uploads it.
- [x] The soak always creates the topic, runs a stream producer, asserts the CG
      loaded + has topology, reports `kafka_produced` in the summary.
- [x] `angreal test soak server --minutes N` exercises Kafka end-to-end — verified
      `--minutes 1`: `demo_kafka_graph` registered with topology
      (process(kafka_alpha)→output), stream accumulator connected (`health: waiting
      [kafka_alpha]`), producer ran, 0 api/conn errors, EXIT=0.

## Reference
Prior art for the Kafka CG packages + producer load lives in git history at
`.angreal/test/soak/server.py` before commit `36c55362` (the
`create_kafka_cg_source_package` / `create_python_kafka_cg_source_package`
generators and the `kafka_stream_worker` / `kafka_batch_worker` threads).

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

### Technical Approach (turn-key recipe — investigated 2026-06-13)
1. **Kafka service** in `docker/docker-compose.demo.yml` — reuse the KRaft def
   from `.angreal/docker-compose.yaml` BUT set
   `KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:9092` (the server is in-network,
   not on host). Add a healthcheck (`kafka-broker-api-versions.sh`). No host port
   needed — the soak producer execs inside the container.
2. **Server env**: add `CLOACINA_VAR_KAFKA_BROKER: "kafka:9092"` to the `server`
   service + `depends_on: kafka: { condition: service_healthy }` so the stream
   accumulator can connect at CG-load time. Broker resolves via `var.rs`
   templating: package config uses `broker = "{{ KAFKA_BROKER }}"`.
3. **Fixture** `examples/fixtures/demo-kafka-stream-rust/` — model Cargo.toml on
   `demo-slow-rust` (`__WORKSPACE__` deps), build.rs = `cloacina_build::configure()`.
   lib.rs needs a standalone `#[reactor]` (declares accumulator `kafka_alpha`) +
   a `#[computation_graph(trigger = reactor("<rx>"), graph = { process(kafka_alpha)
   -> output })]` — the bundled `react = when_any(...)` form was REMOVED
   (CLOACI-I-0101), so it MUST be the split reactor form (model on
   `examples/fixtures/reactor-subscriber-rust` + a reactor decl). package.toml:
   NO `package_type`; `[metadata]` graph_name + reaction_mode + input_strategy +
   a `[[metadata.accumulators]]` entry (`accumulator_type="stream"`, config
   broker `{{ KAFKA_BROKER }}` / topic `demo.kafka.stream` / group) — the
   reconciler merges this over the macro's default passthrough accumulator.
4. **Pack**: add `pack_rust_ws demo-kafka-stream-rust` to
   `docker/pack-demo-fixtures.sh`. Harness auto-uploads it.
5. **Soak** (`.angreal/test/soak/server.py`): restore (from git `36c55362^`)
   `kafka_create_topic` + `KafkaProducer` but retarget to
   `docker compose -f docker/docker-compose.demo.yml exec -T kafka …` (no
   hardcoded `cloacina-kafka` container name). Always: create `demo.kafka.stream`,
   run a stream producer worker for the soak duration, assert the
   `demo_kafka_graph` CG loaded + has topology, report produced count.
6. **Verify**: rebuild demo stack; confirm the fixture builds (compiler), the CG
   appears in `/v1/health/graphs` with topology, and produced Kafka events drive
   it. Each cycle is a full `up --build`.

### Dependencies
- Built on CLOACI-T-0675 (unified soak/demo stack). Touches the demo compose,
  `examples/fixtures/*`, `pack-demo-fixtures.sh`, and `soak/server.py`.

### Risk Considerations
- **Multiple must-be-correct pieces** (reactor macro + CG macro + package.toml
  accumulator merge + kafka listener/readiness); each wrong guess costs a full
  stack rebuild. Budget several verify iterations.
- Advertised-listener must be `kafka:9092` (in-network) — `localhost:9092` (the
  old host-soak value) will not work for the in-container server.
- Stream accumulator connects at CG load → kafka must be healthy first
  (depends_on healthy). If it still races, add load retry.

## Status Updates **[REQUIRED]**

**2026-06-13 — Scoped + reframed (required, not optional). Implementation
pending.** Per direction, Kafka is a required part of the soak. Investigated the
full path (recipe above). Key findings: the old soak's CG package format
(`package_type=[...]`) is rejected by the current server; the bundled
`react = when_any` macro form was removed (must use split `trigger = reactor`);
broker resolves via `{{ KAFKA_BROKER }}` + `CLOACINA_VAR_KAFKA_BROKER`; producer
runs via `docker compose exec kafka` (no host port). This is a genuine multi-part
integration (reactor+CG fixture that must compile, stream-accumulator config
merge, kafka readiness) with several full-stack-rebuild verify iterations — to be
built as a focused effort, not rushed.

**2026-06-13 — DONE.** Built per the recipe and verified first try. Added the
Kafka service (KRaft, advertised `kafka:9092`) + server `CLOACINA_VAR_KAFKA_BROKER`
+ `depends_on healthy`; new fixture `demo-kafka-stream-rust` (reactor + CG, stream
accumulator via `package.toml`); packed it; restored the producer + topic helpers
in the soak via `docker compose exec kafka` (no host port). `--minutes 1`:
`demo_kafka_graph` registered with topology, stream accumulator connected to Kafka
(`health: waiting [kafka_alpha]`), producer streamed events, 0 errors, EXIT=0.
Demo + soak now share one stack that includes Kafka. (Follow-up if wanted: a
batch-accumulator fixture too — only the stream path is exercised here.)