---
id: cloacina-provider-kafka-the
level: task
title: "cloacina-provider-kafka — the flagship native stream-accumulator provider (ships rdkafka)"
short_code: "CLOACI-T-0906"
created_at: 2026-07-15T12:09:22.233803+00:00
updated_at: 2026-07-16T20:54:32.938346+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# cloacina-provider-kafka — the flagship native stream-accumulator provider (ships rdkafka)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Author the flagship first-party provider: `cloacina-provider-kafka` — a `kind = accumulator` (stream) NATIVE provider that ships `rdkafka` + the `KafkaStreamBackend` consumer logic (lifted from core's `stream_backend.rs`/`packaging_bridge.rs::KafkaEventSource`), configured by `#[config] broker/topic/group`, exposing the streaming member the T-0904 loader drains.

**Scope:** new published crate in the providers home (T-0871); `#[constructor(kind = accumulator)]` streaming member using fidius `Stream::from_iter`/streaming plugin_impl over an rdkafka `StreamConsumer`; `grants` advisory (native = trusted, I-0139 (e)); deps on published `cloacina-constructor-contract` + `cloacina-macros` version deps.

**Acceptance:**
- [ ] `cloacinactl constructor package --native` produces a signed `cloacina-provider-kafka` `.cloacina`; it loads and streams real Kafka messages into a `stream` accumulator on the demo stack.
- [ ] The provider crate has NO dependency on core `cloacina` (contract + macros only); rdkafka lives ONLY here.

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0902]]/[[CLOACI-T-0903]]/[[CLOACI-T-0904]] + the publish home ([[CLOACI-T-0871]]/[[CLOACI-T-0872]]).

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

### 2026-07-16 — activated; design settled after de-risking the BLOCKING question.

**KEY FINDING — blocking iterators are SAFE on the native stream path.** fidius's cdylib stream bridge (`pump_stream_handle`, fidius-host executor/cdylib.rs:990) pumps the plugin's iterator on a **dedicated OS thread** (`std::thread::spawn`) pushing into a bounded (cap=4) tokio channel via `blocking_send`; `ChunkStream` reads it async. So a Kafka `source()` whose `next()` blocks on `BaseConsumer::poll()` blocks the PUMP THREAD, never the tokio executor. Cancellation: consumer drop → `blocking_send` fails → pump exits → iterator dropped → rdkafka consumer closed.

**Idle-teardown wrinkle + KEEPALIVE convention:** if the topic is idle, the pump thread parks inside our poll loop and can't observe the drop (blocking_send only fails when there's an item). FIX: the provider's iterator yields an **empty string on poll timeout** (keepalive) and `ProviderStreamSource` (T-0904 shim) **skips empty items** — an idle stream still ticks every poll interval, so teardown resolves within one timeout. Documented as the stream-accumulator keepalive convention.

**Environment facts:**
- Publish home (T-0871/0872) still BACKLOG ⇒ crate lives at `examples/constructor-contract/cloacina-provider-kafka` (the `cloacina-provider-*` precedent), `publish = false`, path deps; crates.io publishing deferred to T-0871.
- Core rdkafka: `rdkafka = "0.39"` (tokio) behind `kafka`. Provider uses `rdkafka = "0.39"` PLAIN (sync `BaseConsumer`, no tokio feature).
- Dev stack Kafka at `localhost:9092` (.angreal/docker-compose.yaml, KRaft). NO Rust kafka test precedent (kafka lives in the Python demos/soak harness) ⇒ local proof = broker-gated Rust test that SELF-SKIPS when 9092 is unreachable.

**Build plan:**
1. Core (small): `ProviderStreamSource::run` skips empty items (keepalive filter).
2. New crate `cloacina-provider-kafka`: `#[constructor(kind = accumulator, mode = stream, name = "kafka_source")] struct KafkaSource { #[config] broker, topic, group }`; `source()` = `BaseConsumer` (bootstrap.servers, group.id, auto.offset.reset=earliest) + subscribe + poll-loop iterator (payload → UTF-8 String; timeout → "" keepalive; transient error → log + keepalive). `constructor_provider!(stream_accumulator = [KafkaSource])`, `native` feature default-on, fidius host deps. NO cloacina core dep (acceptance #2).
3. Broker-gated E2E (`constructor_provider_kafka_native.rs`, features `constructors-wasm,kafka` so the test can use rdkafka's producer): skip if 9092 unreachable; else `package_constructor_provider(new_native)` SIGNED → `unpack_provider_archive` (verify key) → `load_stream_accumulator_source` → produce 3 JSON messages to a fresh topic → 3 boundaries via `accumulator_runtime_with_source(Passthrough)` → shutdown joins (keepalive-bounded idle teardown).
4. Demo-stack lane + core-rdkafka removal remain T-0907 (parity swap first).

### 2026-07-16 — BUILT + PROVEN AGAINST REAL KAFKA → `5403812c`. T-0906 DONE.

- **Crate**: `examples/constructor-contract/cloacina-provider-kafka` — `kafka_source` (`kind=accumulator, mode=stream`), `#[config] broker/topic/group`, `source()` = sync `BaseConsumer` poll-loop iterator (payload → boundary-JSON String; timeout → `""` keepalive; transient error → log+keepalive; create/subscribe failure → panic → fidius `CallError::Panic` = clear load-time error). Deps: macros+contract+fidius+rdkafka 0.39 — **NO core cloacina** (acceptance #2 ✓).
- **Keepalive convention landed in core**: `ProviderStreamSource` skips empty items; documented on `StreamAccumulatorObject::source` (both contracts). Bounds idle-stream teardown to one poll window (2s) — without it, fidius's pump thread parks in the blocking poll forever on an idle topic.
- **PROOF** (`constructor_provider_kafka_native.rs`, self-skips without a broker): signed native package (exact `--native --sign-key` code path) → `unpack_provider_archive` w/ verifying key → `load_stream_accumulator_source` → produced `{"n":1..3}` to a fresh topic on the dev-stack broker (`angreal services up`, localhost:9092) → **all 3 payloads arrived as boundaries** → shutdown joined within the keepalive window. **1/1 green in 6.7s against real Kafka.** `constructor_provider_native` regression 5/5.

**ACCEPTANCE:**
- [x] Signed `.cloacina` via the `--native` path; loads + streams REAL Kafka messages into a stream accumulator. (The *demo-stack* wiring — server/daemon lane — is T-0907's consumption proof, same split as T-0904.)
- [x] No core-cloacina dep; rdkafka ships in the provider. (Core's own `kafka` feature/`KafkaEventSource` stays until T-0907 proves demo-stack parity, THEN gets removed — noted as T-0907 scope.)

**Deferred:** crates.io publish (blocked on T-0871 providers home); demo-stack lane + core-rdkafka removal + authoring docs (T-0907).