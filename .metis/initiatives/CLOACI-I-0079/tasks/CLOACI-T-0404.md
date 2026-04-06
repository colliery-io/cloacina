---
id: computation-graph-soak-test
level: task
title: "Computation graph soak test — sustained 60s event injection with memory and stability checks"
short_code: "CLOACI-T-0404"
created_at: 2026-04-05T19:22:26.325184+00:00
updated_at: 2026-04-06T11:02:01.167342+00:00
parent: CLOACI-I-0079
blocked_by: [CLOACI-I-0083]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0079
---

# Computation graph soak test — sustained 60s event injection with memory and stability checks

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0079]]

## Objective **[REQUIRED]**

Build a standalone soak test binary (or angreal task) that runs a market maker computation graph under sustained event injection for 60+ seconds. Two accumulators push events at different rates (10ms fast, 200ms slow). Verify: no panics, no memory growth, no channel backup, graph continues firing correctly throughout.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Soak test binary or Rust integration test at `examples/performance/computation-graph/` or similar
- [ ] 2 passthrough accumulators → reactor (when_any, latest) → routing graph (market maker from Tutorial 10)
- [ ] Fast source: 10ms interval event injection (100 events/sec)
- [ ] Slow source: 200ms interval event injection (5 events/sec)
- [ ] Runs for 60+ seconds continuously
- [ ] Tracks: total events pushed, total graph fires, fire rate per second
- [ ] Memory check: RSS before and after soak, assert < 10% growth
- [ ] Channel backup check: if accumulator socket channel is full, log warning
- [ ] No panics — test passes cleanly
- [ ] Runnable via `angreal performance computation-graph` or similar
- [ ] Print summary: events pushed, fires, fire rate, memory delta

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

**2026-04-06 — Implementation complete**
- Created `examples/performance/computation-graph/` with Cargo.toml, build.rs, src/main.rs
- Soak test uses Tutorial 10 market maker graph: 2 passthrough accumulators → reactor (when_any, latest) → routing graph
- Fast source (orderbook) at 10ms, slow source (pricing) at 200ms — both configurable via CLI
- Tracking allocator measures heap usage without external deps (alloc/dealloc/realloc counters)
- Channel backup detection via try_send with fallback to blocking send
- Progress reports every N seconds showing fires, events pushed, backups, memory
- Pass/fail: fails if memory growth >10% or zero graph fires
- CLI args: --duration, --fast-interval-ms, --slow-interval-ms, --mem-threshold-pct, --report-interval
- Added `angreal performance computation-graph` task with duration/interval args
- Integrated into `angreal performance all` and `angreal performance quick` (10s duration)
- Verified: 5s test run shows 85 fires/s, 0 backups, +3.1% memory growth — PASS
- NOTE: Embedded performance binary created at examples/performance/computation-graph/ — useful for T-0405 benchmarks
- This binary is NOT the soak test — soak tests must run against the server (packaged mode)
- The actual soak test should be a step in `angreal cloacina server-soak` that uploads a CG package and injects events via WebSocket

**2026-04-06 — BLOCKED**
- Server-mode soak test requires WebSocket event injection into packaged CG accumulators
- AccumulatorAuthPolicy is deny-by-default and scheduler never sets policies → all WS connections get 403
- Broader investigation revealed auth model is fundamentally incomplete (see auth initiative)
- Blocked until auth initiative delivers key-to-tenant binding and CG policy wiring
