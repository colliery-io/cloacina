---
id: t-02-cg-persist-failure-counters-5
level: task
title: "T-02: CG persist-failure counters + 5-strike watchdog → Degraded"
short_code: "CLOACI-T-0590"
created_at: 2026-05-14T13:57:40.252884+00:00
updated_at: 2026-05-14T14:05:59.235077+00:00
parent: CLOACI-I-0108
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0108
---

# T-02: CG persist-failure counters + 5-strike watchdog → Degraded

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0108]]

## Objective **[REQUIRED]**

Surface silent persist failures in the CG runtime as Prometheus counters, then degrade reactor health after a sustained failure streak so operators see the problem on the health endpoint without tailing logs.

Two metrics + one health watchdog:
- `cloacina_reactor_persist_failures_total{graph,reactor,kind}` — counter. `kind` ∈ `cache`, `dirty`, `seq_queue` (corresponding to the three branches of `persist_reactor_state`).
- `cloacina_accumulator_persist_failures_total{graph,accumulator,kind}` — counter. `kind` ∈ `checkpoint`, `boundary`, `batch_buffer` (corresponding to `CheckpointHandle::save`, `persist_boundary`, `persist_batch_buffer`).
- Watchdog: 5 consecutive persist failures on a reactor downgrade `ReactorHealth::Live` (or `Warming`) to `ReactorHealth::Degraded`. Surfaces via `/v1/health/graphs/{name}` (no readiness-probe propagation — that would be too aggressive for a transient DB blip).

Replaces the `let _ = persist_*` patterns flagged as OPS-15 in the May 2026 review.

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

- [ ] `cloacina_reactor_persist_failures_total` increments on every failed `persist_reactor_state` branch (cache / dirty / seq_queue).
- [ ] `cloacina_accumulator_persist_failures_total` increments on every failed `CheckpointHandle::save`, `persist_boundary`, `persist_batch_buffer`.
- [ ] A reactor whose persist fails 5 times in a row transitions to `ReactorHealth::Degraded` and reports that state via `/v1/health/graphs/{name}`.
- [ ] On the next successful persist, the failure streak resets to 0 and the reactor can transition back to `Live` via the existing health state machine.
- [ ] Both metrics registered with `describe_counter!` in `crates/cloacina-server/src/lib.rs`.
- [ ] `docs/operations/metrics.md` updated with both rows and a PromQL example for "graphs in degraded state".
- [ ] Promtool format check passes; new unit test asserts each `kind` label round-trips through /metrics.

## Implementation Notes

- Failure streak is reactor-level (one counter per reactor), not per-kind — operators want to know "is this reactor stuck?" not "is its cache branch stuck specifically?".
- The streak counter lives on `RunningGraph` alongside the existing `failure_counts: HashMap<String, u32>`. Reuse that map with a new key like `format!("{}::persist", graph_name)`.
- `ReactorHealth::Degraded { disconnected: Vec<String> }` already carries a payload — reuse it with `disconnected = vec!["persist".to_string()]` so the health surface explains *why* degraded.
- Reset on success: every `persist_*` success path resets the streak counter to 0.

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

- `cloacina_reactor_persist_failures_total{graph,reactor,kind}` emitted at every failure branch in `persist_reactor_state`. **Note on kind values**: the spec listed `cache | dirty | seq_queue` but `save_reactor_state` is a single atomic DAL call bundling all three — so the bounded `kind` actually shipped is `cache_serialize | dirty_serialize | seq_serialize | save`. Each kind attributes the failure to which step blew up; documented in metrics.md.
- `cloacina_accumulator_persist_failures_total{graph,accumulator,kind}` emitted at the three accumulator persist paths: `checkpoint` (polling-runtime `handle.save`), `boundary` (`persist_boundary` — serialize + DAL save), `batch_buffer` (`persist_batch_buffer`).
- **Watchdog**: `persist_streak: Arc<AtomicU32>` captured by the reactor executor. On any persist failure the streak increments; reaching `PERSIST_FAILURE_DEGRADE_THRESHOLD=5` sends `ReactorHealth::Degraded { disconnected: vec!["persist"] }` via the existing health channel — surfaces on `/v1/health/graphs/{name}` because the response already projects ReactorHealth. Next successful persist resets the streak and sends `Live`. (Diverged from the AC note about `RunningGraph::failure_counts` — that map is owned by the supervisor on a separate task, so an atomic captured inside the reactor task itself is the simpler lock-free path.)
- Both metrics registered with `describe_counter!` in `crates/cloacina-server/src/lib.rs`.
- Unit test `test_persist_failure_metrics_emit` covers all four reactor kinds + all three accumulator kinds via direct emit + /metrics scrape.
- Cardinality guard (`test_i0099_cardinality_within_ceiling`) extended to emit + verify both new metrics — they're held to the same ≤ 64 ceiling as the rest of the I-0099 surface.
- `docs/operations/metrics.md`: rows added for both metrics + a PromQL example "Graphs currently in Degraded state" that uses the existing `cloacina_component_health` gauge to surface stuck reactors.
