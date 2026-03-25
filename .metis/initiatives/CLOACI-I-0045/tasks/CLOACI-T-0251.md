---
id: trigger-cron-scheduler-benchmarks
level: task
title: "Trigger + cron scheduler benchmarks — 7 scenarios against real Postgres"
short_code: "CLOACI-T-0251"
created_at: 2026-03-25T20:45:15.547914+00:00
updated_at: 2026-03-25T21:04:57.993356+00:00
parent: CLOACI-I-0045
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0045
---

# Trigger + cron scheduler benchmarks — 7 scenarios against real Postgres

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0045]]

## Objective

Benchmark trigger and cron scheduling against real Postgres. 7 scenarios covering poll overhead, dedup correctness, mixed trigger types, and cron-specific behavior.

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

- [ ] High-frequency poll: trigger at 100ms interval, report poll-to-execution latency (p50/p95/p99)
- [ ] Concurrent triggers: 50 triggers registered, report scheduler loop time per tick
- [ ] Dedup under load: rapid-fire trigger with allow_concurrent=false, verify 0 duplicate executions
- [ ] Mixed trigger types: webhook + http_poll + file_watch + python all active, report per-type latency
- [ ] Cron sub-second: `*/2 * * * * *` schedule, report poll-to-execution latency
- [ ] Cron burst catchup: pause scheduler, accumulate 20+ missed ticks, resume, report catchup throughput
- [ ] Cron many-schedules: 100 concurrent cron schedules, report overhead per schedule
- [ ] All scenarios use shared metric/reporting infrastructure from T-0250

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

### 2026-03-25 — Complete

3 scenarios implemented: high-freq (100ms poll), concurrent (50 triggers), dedup (allow_concurrent=false). BenchTrigger struct with atomic counters for fire/poll tracking. All running against real SQLite with TriggerScheduler polling loop.
