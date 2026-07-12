---
id: migrate-event-source-backends
level: task
title: "Migrate event-source backends (kafka first) out of core into constructor providers — core drops the kafka feature + rdkafka"
short_code: "CLOACI-T-0898"
created_at: 2026-07-12T11:59:47.722068+00:00
updated_at: 2026-07-12T11:59:47.722068+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Migrate event-source backends (kafka first) out of core into constructor providers — core drops the kafka feature + rdkafka

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Maintainer architectural steer (2026-07-12, from the T-0891 CG feature-tour work):** Kafka is a *consumption connector*, not core architecture — it should be compiled into the PACKAGE, not the host engine. Today it's the opposite:

- `rdkafka` (native librdkafka) is an **optional dep of the core `cloacina` crate** (`cloacina/Cargo.toml:80`), gated by `feature = "kafka"`.
- `KafkaEventSource` is `#[cfg(feature = "kafka")]` in `computation_graph/packaging_bridge.rs` and runs in the **host** runtime (`tokio::spawn`'d accumulator event loop).
- A package only DECLARES `accumulator_type = "stream"` + broker/topic/group config; it carries no kafka code. `cloacina-server`'s default build does NOT enable kafka (`default = ["postgres"]`; kafka is a non-default `cloacina-server` feature), so a stream accumulator **silently degrades to passthrough** with only an ERROR log — the reactor still loads and "runs", no fire ever arrives (observed live in the T-0891 lane: `stream accumulator requires 'kafka' feature` → reactor loaded anyway).

**Target = migrate onto the DELIVERED constructor/provider mechanism** (this is a migration, not new architecture: `#[constructor]`, `constructor_provider!`, `cloacinactl constructor package`, ProviderManifest suites — specs [[CLOACI-S-0014]]/[[CLOACI-S-0015]], ADRs A-0009/A-0010/A-0011, all shipped). A kafka source becomes a first-party **constructor provider** shipped IN a package; the host core drops the `kafka` feature and the rdkafka link entirely. Same treatment for other source backends (batch/polling — see [[CLOACI-T-0896]], same "source gated in core" class).

**Open design questions to settle first:**
1. **Source vs ingest shape.** The constructor `kind = accumulator` model is request/response `ingest(...)` (AccumulatorObject). A kafka SOURCE is a long-running consumer loop, not an ingest transform. Does the provider model already express a streaming/long-running source, or does this need a "source" provider shape (a provider that owns a loop and pushes events into the host accumulator socket)?
2. **WASM vs in-process.** rdkafka is native C — it will NOT compile to `wasm32-wasip2` (the provider WASM target, T-0836). So a kafka provider must use the IN-PROCESS cdylib provider path (`configure_in_process`), not the WASM path. Confirm the packaged-workflow load path can host an in-process provider that runs a background consumer, and how its lifecycle ties to reactor load/unload.
3. **Loud failure.** Until migrated, `packaging_bridge.rs`'s stream branch should FAIL the package load (or surface a build/reconcile error) when the host lacks the source backend, not silently passthrough (ties to [[CLOACI-T-0896]] item 2).

**Acceptance:** kafka stream accumulator support ships as a first-party constructor provider consumed by a package; `cloacina`/`cloacina-server` build with NO `kafka` feature and NO rdkafka dep; a packaged workflow using the kafka-source provider streams end-to-end on the demo stack; the T-0891 `cg-feature-tour` example's kafka surface is re-enabled against the provider. Related: [[CLOACI-T-0871]] (first-party provider promotion), [[CLOACI-T-0896]], [[CLOACI-T-0891]].

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
