---
id: computation-graph-observability
level: initiative
title: "Computation Graph Observability — Reactor, Accumulator, Scheduler, and WebSocket Metrics"
short_code: "CLOACI-I-0099"
created_at: 2026-04-22T12:21:00.287617+00:00
updated_at: 2026-04-22T12:21:00.287617+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: computation-graph-observability
---

# Computation Graph Observability — Reactor, Accumulator, Scheduler, and WebSocket Metrics Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The T-0498 metrics audit (see that task's Status Updates for the full inventory) found that the current metrics surface covers only classic workflow/task execution and HTTP requests. The entire reactive-computation-graph stack — reactors, accumulators, the scheduler loop's health signals, and the WebSocket layer — emits **zero** Prometheus metrics.

Since CGs are now a first-class part of Cloacina (I-0069 through I-0088, tutorials 07-11, packaged CG support in I-0080), operators have no way to observe graph fires, accumulator throughput, reactor latency, supervisor restarts, or WebSocket connection pressure. During a CG soak test or production incident, the only data available today is log lines.

This initiative closes that gap.

## Goals & Non-Goals

**Goals:**
- Emit counters, histograms, and gauges from the reactor, accumulator runtime, scheduler loop, and WebSocket layer.
- Keep label cardinality bounded (no per-graph-instance explosion — graph name is OK, event keys are not).
- Make failure modes observable: supervisor restart counter with bounded `reason`, reactor cache staleness gauge, accumulator lag.
- Align names with the `cloacina_*` namespace convention established in I-0088.
- Integrate new metrics into the CG soak test to prove they behave under load.

**Non-Goals:**
- Full alerting / runbook content — T-0537 owns operator docs, and alert rules live in operator-owned Grafana/Prometheus configs.
- OTLP / distributed tracing additions — T-0455 covers tracing separately.
- Backfilling non-CG areas (registry reconciler, DAL) — can be follow-up initiatives if justified.
- Dashboards — out of scope here; this ships metrics only.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design

Proposed metric set (all names `cloacina_*` to match existing convention; all labels bounded):

### Reactor
- `cloacina_reactor_fires_total{graph,reactor,strategy}` — counter. Strategy ∈ {when_any, when_all, sequential}.
- `cloacina_reactor_fire_duration_seconds{graph,reactor}` — histogram. Time spent inside the user's reactor body.
- `cloacina_reactor_cache_age_seconds{graph,reactor,source}` — gauge. Age of the most recent emission held in the input cache for each source. Lets operators spot stalled sources.
- `cloacina_reactor_deduped_events_total{graph,reactor,source}` — counter. Uses the emission-sequence dedup path added in T-0413.

### Accumulator
- `cloacina_accumulator_events_total{graph,accumulator,kind}` — counter. Kind ∈ {passthrough, stream, polling, batch}.
- `cloacina_accumulator_emit_duration_seconds{graph,accumulator}` — histogram. End-to-end emit latency.
- `cloacina_accumulator_buffer_depth{graph,accumulator}` — gauge. Current queue depth for batch/stateful accumulators.
- `cloacina_accumulator_checkpoint_writes_total{graph,accumulator}` — counter. Checkpoint DAL writes (paired with T-0407/T-0408).

### Supervisor / Resilience
- `cloacina_supervisor_restarts_total{graph,component,reason}` — counter. Component ∈ {accumulator, reactor}. Reason bounded: {panic, error, shutdown_timeout}.
- `cloacina_component_health{graph,component,state}` — gauge (0/1 per state). State ∈ {healthy, degraded, starting, stopped, crashed} — mirrors the health state machine from T-0410/T-0419.

### Scheduler loop (adjacent but adjacent; core engine not CG)
- `cloacina_scheduler_claim_attempts_total{outcome}` — counter. Outcome ∈ {claimed, contended, empty}.
- `cloacina_scheduler_heartbeat_writes_total` — counter.
- `cloacina_scheduler_stale_claims_swept_total` — counter (the sweeper that subsumed RecoveryManager per T-0502).

### WebSocket
- `cloacina_ws_connections_active{endpoint}` — gauge. Endpoint ∈ {accumulator, reactor}.
- `cloacina_ws_messages_total{endpoint,direction}` — counter. Direction ∈ {in, out}.
- `cloacina_ws_auth_failures_total{reason}` — counter. Reason bounded: {ticket_expired, invalid_signature, tenant_mismatch, not_authorized}.

### Cardinality guards
All labels above are either enum-bounded or derived from package metadata (graph name, accumulator name, reactor name) — never event keys, tenant IDs, or tickets. A test in the `ws-integration` or `cg-soak` suite should enumerate emitted label values after a soak and assert the cardinality ceiling.

### Decomposition strategy
Likely 5 tasks, vertical by area: (1) scheduler loop metrics, (2) supervisor + health, (3) accumulator metrics, (4) reactor metrics, (5) WebSocket metrics + soak assertion. The audit-follow-up tasks (T-0533/0534/0535/0536/0537) are prerequisites for docs and CI but are independent of this initiative's implementation tasks.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered

- **Distributed tracing only (OTLP)**: already in place via T-0455, but spans don't give you rate/throughput/gauge views. Metrics and traces are complementary; we still need counters and histograms for dashboards and alerts.
- **Log-based metrics (derive from structured logs via Loki/Promtail)**: works at small scale but loses fidelity under load, couples metrics to log infra, and forces operators to run a log aggregator to see rates.
- **Expose raw `ReactiveScheduler::list_graphs()` over `/admin`**: gives point-in-time state but isn't scrapable, can't feed alerts, and doesn't give historical trends.
- **Per-event labels (event key, tenant id)**: rejected. Unbounded cardinality — would blow up Prometheus in any realistic deployment.

## Implementation Plan

After the audit follow-ups (T-0533-0537) land, decompose into ~5 tasks matching the subsystems listed in Detailed Design. Each task: register metrics, wire emit sites, add unit tests that a metric is emitted, update soak test assertions. One PR per initiative per project convention.

Sequencing:
1. Scheduler loop metrics (smallest, establishes pattern).
2. Supervisor + component health (wires into existing health state machine, low risk).
3. Accumulator metrics.
4. Reactor metrics.
5. WebSocket metrics + soak cardinality assertion (last because it depends on all the above being in place).

Exit: all metrics emit during the CG soak, promtool check (T-0536) passes, metrics doc (T-0537) is updated with the new entries.
