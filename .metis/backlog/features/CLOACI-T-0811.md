---
id: back-pressure-autoscaler-scale
level: task
title: "Back-pressure autoscaler — scale tenant agent pool within limits (leader-elected)"
short_code: "CLOACI-T-0811"
created_at: 2026-06-27T14:43:38.320400+00:00
updated_at: 2026-06-27T18:04:04.083154+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Back-pressure autoscaler — scale tenant agent pool within limits (leader-elected)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The back-pressure autoscaler (slice 1 #4): a server control loop that reads per-tenant utilization (heartbeat `in_flight`/`available_capacity`) + demand (Ready backlog / NoCapacity rate) and adjusts `desired_count` within [floor, effective_limit], with hysteresis + cooldown. The DECISION lives in our control plane, not K8s HPA (HPA cannot see per-tenant queue depth). It writes `desired_count`; the actuator reconciles.

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

- [ ] Sustained high-utilization-with-backlog scales the tenant pool UP to its effective limit (never beyond); sustained idle scales DOWN to the floor; hysteresis + cooldown prevent thrash.
- [ ] Single-writer under multi-replica (NFR-003): only one replica runs the autoscaler (leader election / Postgres advisory lock) — no conflicting scale decisions.
- [ ] Decision is decoupled from actuation: the loop only writes `desired_count`; T-0810 / T-0814 actuate.

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

### 2026-06-27 — Implemented (branch `i0127-agent-control-plane`, not committed)

New module `crates/cloacina-server/src/autoscaler/`:
- `mod.rs` — pure decision logic, all unit-tested: `decide(util, desired, effective_limit, floor, cfg) -> ScaleAction` (Up/Down/Hold, strict thresholds + clamp to `[floor, effective_limit]`); `should_act(last_change, now, cooldown)` cooldown gate; `tenant_utilizations(&[AgentRecord]) -> HashMap<tenant, f64>` (Sum in_flight / Sum max_concurrency, sum-cap==0 → 0.0 guard, global None-tenant agents excluded); `ScaleConfig::from_env()` reading `CLOACINA_AUTOSCALE_{UP_THRESHOLD=0.8,DOWN_THRESHOLD=0.2,COOLDOWN_S=60,FLOOR=0,INTERVAL_S=30}`.
- `leader.rs` — `with_fleet_leadership(db, work) -> Option<T>` via Postgres `pg_try_advisory_lock`/`pg_advisory_unlock` on fixed key `FLEET_CONTROL_LOCK_KEY = 8_110_127`. Runs `work` ONLY on the lock holder; always unlocks after. Holds ONE pooled connection for the tick so lock+unlock share a session (session-scoped lock requires this); failover is automatic — a dead leader's session ends and Postgres frees the lock.

`lib.rs`:
- Replaced the standalone T-0810 reconcile loop (and removed its leader-election TODO) with ONE leader-gated control loop. Each tick, inside `with_fleet_leadership`: (a) autoscale over tenants from `agent_desired.list_all()` plus tenants-with-live-agents → util, `effective_limit`, `decide`, and if Up/Down + `should_act` → `set_desired(clamped +/-1)` + record change time in an in-memory per-leader cooldown map; (b) reconcile `fleet_actuator.reconcile(tenant, desired)` for every desired-count row. Skipped entirely when `fleet_actuator.kind() == "none"`. Kill-switch `CLOACINA_AUTOSCALE` (default on) disables only the autoscale step; reconcile still runs.
- Added `diesel` as a server dep (postgres backend tied to the `postgres` feature) so the leader module runs the advisory-lock SQL via `conn.interact` + `diesel::sql_query`, mirroring the DAL idiom.

Validation (all green, local, Docker postgres on :5432):
- `cargo check -p cloacina-server --tests` clean (only pre-existing cloacina-core warnings); no clippy findings in new code.
- 13 autoscaler unit tests pass (decide x6, utilization x3, should_act x3, config x1).
- Leader-lock integration test `tests::fleet_leadership_is_mutually_exclusive` (`#[serial]` server lib test over real Postgres) passes: while one caller holds leadership a concurrent caller gets `None`; after release leadership is re-acquirable.

Future refinement noted in code: per-tenant Ready-task backlog would be a better leading indicator than reactive utilization, but it lives in per-tenant schemas; utilization is the v1 signal. NOTE: `CLOACINA_RECONCILE_INTERVAL_S` no longer applies — the unified loop ticks at `CLOACINA_AUTOSCALE_INTERVAL_S` (default 30s). Not committed/pushed per instructions.