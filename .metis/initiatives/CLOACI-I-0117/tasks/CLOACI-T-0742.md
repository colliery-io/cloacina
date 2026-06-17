---
id: surface-reactors-as-first-class
level: task
title: "Surface reactors as first-class entities in the CG view + enrich accumulator/reactor operational metrics"
short_code: "CLOACI-T-0742"
created_at: 2026-06-17T21:41:13.884119+00:00
updated_at: 2026-06-17T22:43:48.880264+00:00
parent: CLOACI-I-0117
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# Surface reactors as first-class entities in the CG view + enrich accumulator/reactor operational metrics

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Make **reactors** first-class in the Computation-graphs view (today they're only
surfaced graph-first), and enrich both accumulators and reactors with the
operational metrics the runtime already computes. Surfaced 2026-06-17 watching
the demo stack: you can't see, at a glance, which accumulators a reactor consumes
or its firing logic (any/all), and a reactor not bound to a graph is invisible.

## Why this matters (code-grounded)

- **Reactors are standalone, not 1:1 with a graph.** The scheduler's core map is
  `reactors: HashMap<String, RunningGraph>` (`computation_graph/scheduler.rs:333`),
  keyed by reactor name. A reactor is `load_reactor()`'d first
  (`scheduler.rs:384`) and a graph `bind_graph_to_reactor()`'s to it afterward
  (`scheduler.rs:581`) — confirmed live in the demo logs ("reactor loaded and
  running reactor=demo_py_state_rx" then "graph bound to reactor"). So a reactor
  with **no graph bound** (or a shared one) exists in the runtime but is **not
  shown anywhere** today, because the UI and `/v1/health/graphs` are graph-first.
- **The data already exists.** Each running reactor carries `accumulators:
  Vec<String>` and `reaction_mode: String` (any/all, `scheduler.rs:115,131`),
  plus live counters `(fires, last_fire_unix_ms)` (`reactor.rs:249-251`) and a
  `paused` flag. There's just no reactor-centric endpoint or UI card.
- **Accumulator metrics are lopsided.** `AccumulatorStatus` exposes only name + a
  coarse health enum (`accumulator.rs:42` Starting/Connecting/Live/Disconnected/
  SocketOnly); the UI renders it as a single status dot (`routes/Graphs.tsx`
  Accumulators card). Meanwhile **buffer depth** is already a Prometheus gauge
  (`set_accumulator_buffer_depth`, `accumulator.rs:405,541`) and
  `cloacina_reactor_fires_total` a Prometheus counter (`reactor.rs:702`) — both go
  to `/metrics`, NOT the JSON the UI polls every 5s (`api/health.ts`).

## Scope

### Server (`crates/cloacina-server/src/routes/health_graphs.rs`)
- Add `GET /v1/health/reactors` returning `ReactorStatus { name, criteria
  (any/all from reaction_mode), accumulators: Vec<String>, bound_graph:
  Option<String>, fires, last_fired_at, paused, health }`, sourced from the
  scheduler's reactor-keyed map so **unbound reactors are included**. Reuse the
  key-visibility filtering used by `list_graphs`/`list_accumulators`.
- Enrich `AccumulatorStatus`: lift the buffer-depth gauge into the JSON + add
  capacity/fill for state accumulators, events-received (total + derived rate),
  last-event-at. (Altitude: make the health API the single source for these
  counters instead of teaching the UI to scrape `/metrics`.)

### UI (`ui/src/routes/Graphs.tsx`, `ui/src/api/health.ts`)
- Add a **Reactors** card: name, a **criteria chip (any/all)**, its accumulators
  (reuse the `accStatus` dots so each shows source health), throughput +
  last-fired (reuse `useGraphThroughput`), paused badge. Row links to graph
  detail when a graph is bound.
- Enrich the **Accumulators** card from a status dot to real metrics: buffer/fill
  (incl. **state-accumulator capacity** — visible on the new `demo-py-state`,
  capacity=5 → window N/5), events rate, last-event-at.
- Add a `useReactors()` hook alongside `useGraphs()`/`useAccumulators()`.

## Notes
- The graph-detail page (`routes/GraphDetail.tsx`) already renders the reactor as
  a first-class node (sources → reactor → graph) with reaction-mode/criteria;
  this task brings that to the **list** view and adds operational metrics.
- The graph row's existing `fires`/`throughput`/`last-fired` ARE the reactor's
  counters; once reactors have their own card, clarify that attribution on the
  graph row (tooltip/label) to avoid double-reading.
- Validate against demo fixtures: `demo-py-state` (state acc, capacity=5, reactor
  `demo_py_state_rx`, when_any), `mixed-rust`, `demo-py-graph`.

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

- [ ] `GET /v1/health/reactors` exists, returns reactors from the scheduler's
      reactor-keyed map (incl. ones with no graph bound), key-visibility filtered.
- [ ] Each reactor row shows: name, criteria chip (any/all), the accumulators it
      consumes (with per-source health dots), throughput, last-fired, paused.
- [ ] Accumulator rows show real operational metrics, not just a status dot —
      buffer/fill (with capacity for state accumulators), events rate, last-event.
- [ ] Buffer-depth + fires counters are served from the health API (not only
      `/metrics`); the UI reads them from the polled JSON.
- [ ] Verified live against the demo stack: `demo-py-state` shows reactor
      `demo_py_state_rx` (when_any) consuming `py_window` at N/5 fill.

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

- 2026-06-17: **Reactors-first surfacing built** (branch `py-parity-t0688` /
  PR #132 per user). Server: `scheduler::ReactorStatus` +
  `ComputationGraphScheduler::list_reactors()` (iterates the `reactors` map so
  unbound reactors are included; `bound_graphs` is the reverse `graph_to_reactor`
  lookup), `cloacina_api_types::ReactorStatus`, `GET /v1/health/reactors` (reuses
  the `graph_visible` tenant gate). Registered route + OpenAPI path/schema;
  regenerated `docs/static/openapi.json` (drift gate) + TS client types; added
  `client.listReactors()`. UI: `useReactors()` + `queryKeys.reactors`, and a
  **Reactors card** in `routes/Graphs.tsx` — name, **criteria chip (any/all via
  explainToken)**, accumulators consumed (reusing source-health dots), bound graph
  (or "unbound"), throughput (reused `useGraphThroughput`), last-fired, paused.
  Compiles green: `angreal check crate cloacina-server`, TS client typecheck+build,
  UI tsc + vite build. **Still TODO (this task):** accumulator-card enrichment
  (buffer/fill incl. state-accumulator capacity, events rate, last-event — needs
  lifting the Prometheus buffer-depth gauge into the health API); live-verify on
  the demo stack after a server+ui rebuild.
