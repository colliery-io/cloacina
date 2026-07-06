---
id: reactive-layer-ha-accumulator
level: task
title: "Reactive-layer HA — accumulator/reactor state is per-replica in-memory; no cross-replica coordination"
short_code: "CLOACI-T-0851"
created_at: 2026-07-06T11:39:27.698917+00:00
updated_at: 2026-07-06T11:39:27.698917+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Reactive-layer HA — accumulator/reactor state is per-replica in-memory; no cross-replica coordination

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Make the reactive layer (accumulators + reactors) safe and well-defined under multi-replica cloacina-server deployments. Filed from the 2026-07-06 HA review: everything else is HA-proven (T-0818 / ADR A-0008 — task/cron scheduling is active-active via atomic claiming; the fleet control loop is per-tick leader-elected with validated failover; login state is in Postgres), but the reactive layer is the gap:

- **Accumulator buffers and reactor dirty-flags/caches are per-replica in-memory state.** Every replica that loads a CG package spawns its OWN accumulators + reactor.
- **An event lands on ONE replica** (whichever receives the WS/REST inject), so with N replicas a stream's events can split across N independent buffers — a `state` accumulator's window or a `when_all` criteria set may never assemble on any single replica.
- Reactor snapshots persist to the DB (`persist_reactor_state`) but restore is per-instance, not coordinated; two replicas restoring the same reactor both proceed independently.
- T-0722 moved graph COMPUTE to the agent fleet, but the reactor state machine stays server-side per-instance.

## Design directions (discovery — pick with a human check-in)

1. **Reactor leadership (likely v1)**: per-reactor claim/lease (Postgres advisory lock or leased row, mirroring A-0008's per-tick election) — exactly one replica runs a given reactor+accumulators; others route incoming socket events to the owner (the delivery substrate/outbox already gives an at-least-once inter-replica channel). Failover = lease expiry → another replica restores from the persisted snapshot.
2. **Sticky routing only (stopgap)**: document + enforce that reactive socket traffic must be session-pinned to one replica (LB affinity); accept reactor loss on replica death until restore.
3. **Externalized accumulator state**: buffers in Postgres/streams rather than memory — biggest change, best semantics; probably post-v1.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (correctness gap only under multi-replica postgres deployments; single-replica and embedded modes are unaffected)

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

- [ ] A chosen coordination model (leadership / routing / externalized state) recorded as an ADR with maintainer sign-off.
- [ ] Under a 2-replica postgres deployment: events injected round-robin across replicas assemble correctly (a `when_all` reactor fires; a `state` window fills) with no split-brain buffers.
- [ ] Replica death while owning a reactor → another replica resumes it from the persisted snapshot (bounded takeover time; no lost checkpointed state).
- [ ] Multi-replica reactive validation added to the k8s-leader e2e lane (extends T-0818's harness).
- [ ] Single-replica + embedded behavior byte-for-byte unchanged.

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

*To be added during implementation*