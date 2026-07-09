---
id: accumulator-freshness
level: task
title: "Accumulator freshness instrumentation — last_event_at + events/min + error on AccumulatorStatus"
short_code: "CLOACI-T-0765"
created_at: 2026-06-21T19:21:02.078499+00:00
updated_at: 2026-06-21T19:52:14.619385+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Accumulator freshness instrumentation — last_event_at + events/min + error on AccumulatorStatus

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Make per-accumulator freshness observable so the graph operational view can show
the degraded banner, SOURCES card, accumulators table, and reactor readiness.
Today `AccumulatorStatus` is `{name, reactor, tenant_id, status:<free-form>}`;
`AccumulatorHealth` (Starting/Connecting/Live/Disconnected/SocketOnly) flows via a
`watch` channel per source but carries no timing. Add typed freshness.

### Proposed contract (api-types `AccumulatorStatus`)
- `state: String` — `AccumulatorHealth` label (already derivable).
- `last_event_at: Option<DateTime<Utc>>` — wall-clock of the last boundary emit.
- `events_per_min: Option<f64>` — windowed emit rate.
- `error: Option<String>` — degradation detail (e.g. "WS open, no boundary frames
  since …") when Disconnected/stale.
(Keep `status` free-form for back-compat / forward fields.)

### Instrumentation points (grounded)
- `accumulator.rs`: each successful `ctx.output.send(&boundary)` stamps a
  `last_event` Instant + bumps a windowed counter; degradation sets `error`.
  Carry it next to the `AccumulatorHealth` watch channel (a parallel freshness
  watch or an enriched report struct), registered with the registry like
  `register_accumulator_health`.
- `registry.rs`: `list_accumulators_with_health[_for_key]` also returns freshness.
- `health_graphs.rs` `list_accumulators`: populate the new typed fields.
- Regenerate the TS SDK; the UI reads the typed fields (no free-form parsing).

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

### 2026-06-21 — DONE (d86c9a42, 59855fb3, 69b262a1)
Shipped end-to-end:
- `BoundarySender` (single emit chokepoint) gains `last_event_ms`; `events_total`
  is the existing monotonic `sequence`. Shared via a `FreshnessHandle`.
- `AccumulatorSpawnConfig` carries the handle; every factory (passthrough /
  stream-backend / state / test) builds the sender with `with_freshness`; the
  scheduler registers it on all 3 spawn paths (initial, restart, re-spawn).
- `EndpointRegistry` stores + returns freshness; `list_accumulators` promotes
  typed `state` / `last_event_at` / `events_total` / `error` onto
  `AccumulatorStatus` (error left None v1 — UI derives staleness from
  last_event_at; rate is UI-derived from events_total deltas like fires/min).
- OpenAPI spec + TS SDK regenerated (drift gate green); SDK builds.
`cargo check` (cloacina + api-types + server) + SDK build green.

Note: shipped `events_total` (monotonic) rather than server-computed
`events_per_min` — the UI derives the per-minute rate from successive polls,
matching the existing fires/min pattern (`useGraphThroughput`).