---
id: t-01-cloacina-compiler-metrics
level: task
title: "T-01: cloacina-compiler /metrics endpoint with build-relevant metrics"
short_code: "CLOACI-T-0591"
created_at: 2026-05-14T15:10:50.747661+00:00
updated_at: 2026-05-14T15:16:31.835983+00:00
parent: CLOACI-I-0109
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0109
---

# T-01: cloacina-compiler /metrics endpoint with build-relevant metrics

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0109]]

## Objective **[REQUIRED]**

Install a Prometheus recorder in `cloacina-compiler` and expose a `/metrics` endpoint with build-relevant metrics. Mirrors the recorder setup that lives in `cloacina-server` today (see `crates/cloacina-server/src/lib.rs` line ~271-275).

Proposed metric set (all `cloacina_compiler_*`, bounded labels):
- `cloacina_compiler_builds_total{status}` — counter. `status` ∈ `ok`, `failed`, `timed_out`, `cancelled`.
- `cloacina_compiler_build_duration_seconds` — histogram. Wall-clock around the cargo build subprocess.
- `cloacina_compiler_queue_depth{state}` — gauge, **SQL-derived** per the REC-06 / I-0108 pattern. State ∈ `queued`, `building`, `failed`.
- `cloacina_compiler_sweep_resets_total` — counter. Stale-build sweeper resets (paired with T-0522).
- `cloacina_compiler_heartbeat_failures_total` — counter. Failed heartbeat writes from the builder.

Endpoint binds on `--metrics-bind 127.0.0.1:9000` (default). Public, no auth — matches the convention from `cloacina-server`.

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

- [ ] PrometheusBuilder recorder installed in `cloacina-compiler` startup.
- [ ] All five metrics registered via `describe_*!` with HELP text + emitted from the build-loop / sweeper / heartbeat-writer code paths.
- [ ] `--metrics-bind` flag wired (default `127.0.0.1:9000`); served alongside the existing compiler health endpoint.
- [ ] `cloacina_compiler_queue_depth{state}` is SQL-derived — re-seeded each sweep tick from `compiled_data` row counts, not via inc/dec at enqueue/dequeue sites (REC-06 pattern).
- [ ] `angreal test metrics-format` extended to also scrape the compiler's `/metrics` through `promtool check metrics`.
- [ ] `docs/operations/metrics.md` updated with a new "Compiler metrics" section listing all five.

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

- Added `metrics = 0.24` + `metrics-exporter-prometheus = 0.18` to `cloacina-compiler/Cargo.toml`.
- `PrometheusBuilder::new().install_recorder()` runs at the top of `cloacina_compiler::run()`, before any emit fires. New `register_compiler_metrics()` declares HELP/TYPE for all five metrics.
- Reused the existing `--bind` listener instead of adding `--metrics-bind` — kept the compiler footprint to a single bound port. `/metrics` is served alongside `/health` and `/v1/status` via `health::serve` (signature now also takes a `PrometheusHandle`).
- Emit sites in `loopp.rs`:
  - `cloacina_compiler_builds_total{status="ok"|"failed"|"timed_out"}` at the three `BuildOutcome` arms. Dropped the spec's `cancelled` value — no cancellation code path currently exists in the build loop; an explicit shutdown today just exits the loop after the current build finishes via its own outcome.
  - `cloacina_compiler_build_duration_seconds` recorded around `execute_build()`.
  - `cloacina_compiler_heartbeat_failures_total` incremented in the heartbeat loop on each `heartbeat_build` error.
  - `cloacina_compiler_sweep_resets_total` incremented by `n` after each successful `sweep_stale_builds(n)` return.
  - `cloacina_compiler_queue_depth{state="queued"|"building"}` re-seeded every sweep tick from `registry.build_queue_stats()`. Spec listed `failed` as a third state but `build_queue_stats` only returns pending/building (failed rows are terminal, not "in the queue") — described accordingly.
- `angreal test metrics-format` extended: builds both binaries, boots both, scrapes each `/metrics`, runs `promtool check metrics` against each. Helper functions `wait_for_health()` and `scrape_and_validate()` factor out the common shape.
- `docs/operations/metrics.md`: new "Compiler metrics" section with all 5 metrics + 2 PromQL examples (build rate by outcome, build p95 duration). Header rewritten to call out the two `/metrics` endpoints and the explicit daemon-is-quiet decision per CLOACI-A-0005.
