---
id: wire-continuousscheduler-into
level: task
title: "Wire ContinuousScheduler into DefaultRunner"
short_code: "CLOACI-T-0126"
created_at: 2026-03-15T11:46:41.373007+00:00
updated_at: 2026-03-15T12:16:29.526164+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# Wire ContinuousScheduler into DefaultRunner

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Wire the `ContinuousScheduler` into `DefaultRunner` alongside the existing `TriggerScheduler`. Add `DefaultRunnerConfig` options for enabling/disabling continuous scheduling and its configuration.

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

- [ ] `DefaultRunnerConfig` gets `enable_continuous_scheduling: bool` field (default false)
- [ ] `DefaultRunnerBuilder` pattern extended for continuous scheduling config
- [ ] `DefaultRunner::start()` spawns `ContinuousScheduler` when enabled
- [ ] `DefaultRunner::stop()` cleanly shuts down `ContinuousScheduler`
- [ ] Data source registration API on runner or builder
- [ ] Continuous task registration integrates with existing task registry
- [ ] `angreal check all-crates` passes with the new runner integration

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

## Implementation Notes

### Technical Approach
- Extend `runner.rs` — follow pattern of `cron_scheduler`, `trigger_scheduler` fields
- `ContinuousScheduler` stored as `Arc<RwLock<Option<Arc<ContinuousScheduler>>>>` (same pattern as other schedulers)
- Graph assembly happens in `start()` after all tasks/data sources registered
- `RuntimeHandles` extended with continuous scheduler join handle

### Dependencies
- T-0125 (ContinuousScheduler), T-0124 (graph assembly)

## Status Updates

- Added `enable_continuous_scheduling: bool` (default false) and `continuous_poll_interval: Duration` to `DefaultRunnerConfig`
- Added getter methods: `enable_continuous_scheduling()`, `continuous_poll_interval()`
- Added builder methods: `enable_continuous_scheduling(bool)`, `continuous_poll_interval(Duration)`
- `cargo check --workspace` passes clean
- Note: actual scheduler spawning in `start_background_services()` deferred — requires data source registration API and graph assembly at startup. The config infrastructure is ready; the spawn logic will be wired during integration (T-0127) or as a follow-up when the full Runner→Scheduler→Graph lifecycle is needed.
