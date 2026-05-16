---
id: t-04-reactor-metrics-fires-fire
level: task
title: "T-04: Reactor metrics — fires, fire duration, cache age, deduped events"
short_code: "CLOACI-T-0587"
created_at: 2026-05-14T13:03:12.972273+00:00
updated_at: 2026-05-14T13:33:28.100420+00:00
parent: CLOACI-I-0099
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0099
---

# T-04: Reactor metrics — fires, fire duration, cache age, deduped events

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0099]]

## Objective **[REQUIRED]**

Instrument the reactor with fire rate, latency, input-cache staleness, and dedup counters so operators can spot stalled sources and quantify reactor cost.

Metrics to add:
- `cloacina_reactor_fires_total{graph,reactor,strategy}` — counter. Strategy ∈ {when_any, when_all, sequential}.
- `cloacina_reactor_fire_duration_seconds{graph,reactor}` — histogram. Time inside the user's reactor body.
- `cloacina_reactor_cache_age_seconds{graph,reactor,source}` — gauge. Age of the most recent emission per source.
- `cloacina_reactor_deduped_events_total{graph,reactor,source}` — counter. Uses the emission-sequence dedup path from T-0413.

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

- [ ] All four metrics emit for each of the three strategies (when_any, when_all, sequential).
- [ ] `source` label values come exclusively from declared input names in the package metadata.
- [ ] Cache-age gauge updates on every reactor poll, not just on fire.
- [ ] Unit tests assert each strategy emits the right `strategy` label.
- [ ] Promtool format check passes; metrics doc updated.

## Implementation Notes

Depends on T-0584. Touches reactor runtime and the input-cache from I-0077. Be careful that cache-age gauge does not allocate per poll — pre-register the per-source gauge handle at reactor start.

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

### 2026-05-14 — implemented

- Strategy label collapsed (criteria × input_strategy) onto bounded `{when_any, when_all, sequential}` at top of `Reactor::run()`.
- `cloacina_reactor_fires_total` + `cloacina_reactor_fire_duration_seconds` emitted around both fire sites in `reactor.rs` (Latest branch line ~606, Sequential drain branch line ~640).
- `cloacina_reactor_cache_age_seconds` driven by a new `last_received_at: Arc<RwLock<HashMap<SourceName, Instant>>>` populated in the receiver task. Every boundary arrival refreshes ages for every known source, so silent sources show increasing staleness without a separate ticker. Sources with no emissions yet are absent until their first boundary — documented limitation.
- `cloacina_reactor_deduped_events_total` registered + documented as **reserved**: the reactor-side dedup check is a follow-up to T-0413's persistence work. Metric is exposed today so alert rules can be authored against the eventual name; unit test exercises the emit path directly.
- All four metrics registered in `crates/cloacina-server/src/lib.rs`. Unit test `test_reactor_metrics_emit` covers each strategy + histogram/gauge/dedup signatures.
- `docs/operations/metrics.md`: rows added per-section, deduped marked Reserved, "Current gaps" narrowed to WebSocket-only.
