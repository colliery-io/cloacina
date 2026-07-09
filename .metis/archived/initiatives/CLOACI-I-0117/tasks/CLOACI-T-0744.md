---
id: enrich-accumulator-card
level: task
title: "Enrich accumulator-card operational metrics — lift buffer-depth/fires gauges into the health API"
short_code: "CLOACI-T-0744"
created_at: 2026-06-17T22:44:24.570660+00:00
updated_at: 2026-07-05T16:09:39.179+00:00
parent: CLOACI-I-0117
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# Enrich accumulator-card operational metrics — lift buffer-depth/fires gauges into the health API

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Enrich the Accumulators card in the CG view from a single coarse status dot into
real operational metrics: buffer/fill (with **capacity for state accumulators**,
e.g. `demo-py-state`'s window at N/5), events received (total + derived rate),
and last-event-at. Carved out of [[CLOACI-T-0742]] — whose reactor-surfacing half
shipped in #132 — this is the accumulator-metrics half (T-0742 AC lines 3–4).

## Why / grounding (code-cited)

`AccumulatorStatus` (`crates/cloacina-api-types/src/health.rs`) carries only name
+ a coarse health enum (`computation_graph/accumulator.rs:42`: Starting/Connecting/
Live/Disconnected/SocketOnly), rendered as one dot in `ui/src/routes/Graphs.tsx`.
The richer signal already exists but only as **Prometheus** series —
`set_accumulator_buffer_depth` (`accumulator.rs:405,541`) and
`cloacina_reactor_fires_total` (`reactor.rs:702`) — which the UI's polled JSON
never sees.

## Scope

- Server: extend `AccumulatorStatus` with buffer depth / capacity / fill,
  events-received (total + rate), last-event-at; source from the
  endpoint/accumulator registry. Altitude: make the health API the single source
  for these counters rather than teaching the UI to scrape `/metrics`.
- UI (`routes/Graphs.tsx`): render those columns on the Accumulators card; show
  `N/capacity` for state accumulators.
- Regenerate `docs/static/openapi.json` (drift gate) + TS client types.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P3 - Low (when time permits)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Accumulator rows show buffer/fill (capacity for state accumulators),
      events rate, and last-event — not just a status dot.
- [ ] Buffer-depth + fires counters are served from the health API JSON.
- [ ] OpenAPI + TS client regenerated; UI tsc + build green.
- [ ] Live demo: `demo-py-state`'s `py_window` shows N/5 fill.

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

### 2026-07-05 — implementation (branch feat/t0744-accumulator-gauges)
**Design:** ride the T-0765 freshness-probe seam — `BoundarySender` + `FreshnessHandle` (shared Arc atomics, `accumulator.rs`) gained `buffer_depth` (−1 = kind doesn't buffer → API None) and `buffer_capacity` (≤0 = unbounded/N-A → None). `set_accumulator_buffer_depth` (the existing Prometheus funnel) now ALSO stores into the probe, so every existing gauge call site feeds the polled health API for free.
- **batch runtime**: reports `max_buffer_size` as capacity at start.
- **state runtime**: was invisible to BOTH the Prometheus gauge and the API — now reports depth (incl. DAL-restored buffer) + bounded `capacity` (the `demo-py-state` N/5 case), and updates depth per ingest/evict.
- **API**: `AccumulatorStatus` += `buffer_depth`/`buffer_capacity` (serde-default, utoipa); `list_accumulators` samples them from the probe.
- **UI** (`Graphs.tsx` accumulators card): buffer fill (`buf N` / `buf N/cap`), events total, last-event via the library's `formatAgo` — the card previously showed NONE of the freshness fields despite the API carrying them since T-0765.
Rust checks clean (cloacina + api-types + server); `tsc -b` clean. Remaining: openapi.json regen (in flight) + TS API types + live demo-stack check of py_window N/5 + commit/PR.
NOTE on "fires" in the title: reactor fire counts already ship on `ReactorStatus` (T-0766 fires log + timeseries); the accumulator-side gap was buffer/fill, which this closes.

### 2026-07-05 — CLOSING (commit 4501781f); the live py_window AC exposed a SEPARATE latent bug → [[CLOACI-T-0839]]
Also derived ~events/min on the card by feeding `events_total` through the library's fires-delta throughput hook, and confirmed the new fields LIVE on the rebuilt demo server (`buffer_depth`/`buffer_capacity` present in `/v1/health/accumulators`; freshness + inject counters all correct under real injects). openapi.json + TS client regenerated, drift gates clean; unit probe test `test_state_accumulator_probe_reports_buffer_fill` proves the state runtime reports depth 3 / capacity 5 through the shared probe.

**The one AC not demonstrable live** — `py_window` at N/5 — is blocked by a pre-existing, unrelated defect this verification UNCOVERED: packaged Python state accumulators silently degrade to passthrough server-side (the names-only reactor dispatch drops `accumulator_type`), so live `py_window` has no buffer for ANY gauge to report — Prometheus reads 0 too. Filed with full root cause + repro as [[CLOACI-T-0839]] (P1). When T-0839 lands, the N/5 fill renders with zero further work here — the T-0744 machinery is unit-proven against the real state runtime. COMPLETE.