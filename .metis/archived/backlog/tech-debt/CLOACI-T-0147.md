---
id: multi-cycle-reactive-loop
level: task
title: "Multi-cycle reactive loop integration test (detector → task → LedgerTrigger → detector → task)"
short_code: "CLOACI-T-0147"
created_at: 2026-03-15T14:39:35.107709+00:00
updated_at: 2026-03-15T14:51:44.338389+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Multi-cycle reactive loop integration test (detector → task → LedgerTrigger → detector → task)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

No test exercises the full reactive feedback loop across multiple cycles: source detector fires → task runs → LedgerTrigger fires → derived source detector fires → downstream task runs. The current tests only show one cycle. This is the core "make"-like behavior — the loop that keeps the graph alive.

Need an integration test with a two-level graph: `raw_events → aggregate_hourly → hourly_stats → build_dashboard`.

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

## Acceptance Criteria

## Acceptance Criteria

- [ ] Two-level graph: source → task_a, task_a output → derived source → task_b
- [ ] LedgerTrigger watches task_a, fires derived source detector
- [ ] Derived detector emits boundaries for task_b
- [ ] task_b fires after task_a completes (not simultaneously)
- [ ] Full cycle completes within a single scheduler run (not requiring restart)
- [ ] Verify via ledger events: both TaskCompleted and both AccumulatorDrained present

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

- `test_multi_cycle_reactive_loop`: Two-level graph (raw_events → aggregate, hourly_stats → dashboard)
  - Cycle 1: raw_events detector → aggregate fires
  - Cycle 2: simulated aggregate completion + derived detector → dashboard fires
  - Verifies both tasks in `fired` results and both `AccumulatorDrained` in ledger
  - Simulates LedgerTrigger's role by injecting derived detector completion (full Trigger integration is T-0143)
- `test_ledger_trigger_bridges_cycles`: Verifies LedgerTrigger fires on task completion, doesn't re-fire, then fires again on next completion
- 10 continuous + 11 macro = 21 integration tests passing
- Note: the multi-cycle test simulates the execution bridge (task completion → derived detector). Full end-to-end with real execution pipeline is T-0143.
