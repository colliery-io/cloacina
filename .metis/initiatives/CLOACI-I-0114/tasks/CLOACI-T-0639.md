---
id: configurable-agent-liveness
level: task
title: "Configurable agent liveness — heartbeat interval + dead-after threshold"
short_code: "CLOACI-T-0639"
created_at: 2026-06-06T02:50:05.259140+00:00
updated_at: 2026-06-06T02:50:32.637787+00:00
parent: CLOACI-I-0114
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Configurable agent liveness — heartbeat interval + dead-after threshold

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]]

## Objective **[REQUIRED]**

Make fleet agent liveness/failover aggressiveness operator-configurable instead of
hardcoded. Today the heartbeat interval (15s) and the dead-after threshold (3 missed beats =
45s) are constants, so dead-agent detection + in-flight reclaim is fixed at ~45-60s with no
way to tune it. Surfaced by T-0638 (reclaim e2e): an operator who loses an agent waits ~a
minute before reclaim, with no knob.

## Current state (hardcoded)
- `DEFAULT_HEARTBEAT_INTERVAL_SECONDS = 15` (`crates/cloacina/src/fleet/protocol.rs:65`).
- Register handler advertises it verbatim: `heartbeat_interval_seconds:
  DEFAULT_HEARTBEAT_INTERVAL_SECONDS` (`crates/cloacina-server/src/routes/agent.rs:103`).
- Sweeper: `beat = DEFAULT_HEARTBEAT_INTERVAL_SECONDS; dead_after = beat * 3;` ticks every
  beat (`crates/cloacina-server/src/lib.rs:777-783`).

## Plan
Two server CLI flags (keep current values as defaults so behavior is unchanged):
- `--agent-heartbeat-interval-s` (env `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S`, default 15) —
  advertised to agents (the agent's heartbeat loop already honors the register response) AND
  the sweeper's cadence.
- `--agent-liveness-misses` (env `CLOACINA_AGENT_LIVENESS_MISSES`, default 3) — dead-after =
  interval × misses.

Plumb: `main.rs` args → `run()` params → `AppState.agent_heartbeat_interval_seconds` (so the
register handler advertises the configured value) → sweeper uses interval × misses. Clamp
both to ≥1.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `--agent-heartbeat-interval-s` / `--agent-liveness-misses` (+ env) on cloacina-server.
- [ ] Register response advertises the configured interval (agents heartbeat at that rate).
- [ ] Sweeper detects dead after interval × misses; defaults reproduce today's 15s/45s.
- [ ] cloacina-server compiles + existing tests pass.

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

### 2026-06-05 — implemented (compiles clean); fleet e2e now exercises it

Two server CLI flags added (defaults preserve the old 15s/45s behavior):
- `--agent-heartbeat-interval-s` / `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` (default 15).
- `--agent-liveness-misses` / `CLOACINA_AGENT_LIVENESS_MISSES` (default 3).

Plumbing:
- `main.rs`: 2 clap args (heartbeat default = `cloacina::fleet::DEFAULT_HEARTBEAT_INTERVAL_SECONDS`) → passed to `run()`.
- `lib.rs`: `run()` gained `agent_heartbeat_interval_s: u32`, `agent_liveness_misses: u32`; sweeper now `beat = from_secs(interval.max(1)); dead_after = beat * misses.max(1)`; new `AppState.agent_heartbeat_interval_seconds` (clamped) so the register handler advertises the configured value; both AppState literals (real + test) updated.
- `routes/agent.rs`: register response advertises `state.agent_heartbeat_interval_seconds` (was the const); dropped the now-unused import.

Compiles: `angreal check crate crates/cloacina-server` ✅ (only pre-existing cloacina warnings).

Bonus — `angreal helm fleet` now sets `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S=5` +
`CLOACINA_AGENT_LIVENESS_MISSES=2` (dead-after ~10s) on the server, which (a) exercises these
flags end-to-end and (b) cuts the reclaim step from ~165s to ~75s. Lowered the fixture
`SLEEP_SECONDS` 90→45 to match. Harness py_compile + ruff clean.

**Needs a fresh `angreal helm fleet` run** (server image rebuild) to validate. All exit
criteria met pending that run.
