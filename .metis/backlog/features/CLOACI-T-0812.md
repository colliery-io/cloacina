---
id: auto-provision-initial-agent-s-on
level: task
title: "Auto-provision initial agent(s) on tenant create"
short_code: "CLOACI-T-0812"
created_at: 2026-06-27T14:43:39.258788+00:00
updated_at: 2026-06-27T17:44:34.061405+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Auto-provision initial agent(s) on tenant create

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Auto-provision initial agent(s) on tenant create (slice 1 #5): POST /v1/tenants sets the new tenant's initial `desired_count` (within the default limit) so a fresh tenant comes up with working compute without a manual provision step.

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

- [ ] Creating a tenant sets an initial `desired_count` (configurable, default e.g. 1) within the default limit; the actuator then brings the agent(s) up.
- [ ] Respects the limits model (never exceeds the default/effective limit); the initial count is configurable.

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

### 2026-06-27 — Implemented + tested (not committed)

Done on branch `i0127-agent-control-plane`. Auto-provision wired into the tenant-create path.

**Source of initial count + clamp**: Added `AppState.initial_agents: u32`, read from `CLOACINA_INITIAL_AGENTS` (default 1), mirroring the existing `default_max_agents` (`CLOACINA_DEFAULT_MAX_AGENTS`, default 4) env read. Set in BOTH `AppState` builders (runtime `run()` + `test_state`, where it defaults to 1). The effective count is `min(initial_agents, default_max_agents)` via a small pure helper `routes::tenants::initial_desired_count(initial, max)`. `0` (from either knob) → `0` → the desired-count write is skipped (auto-provision disabled).

**Best-effort write (DECISION + flag)**: After `admin.create_tenant` succeeds, if the clamped initial is `> 0` the handler calls `dal.agent_desired().set_desired(&credentials.schema_name, initial)`. If that write fails it logs a `warn!` and STILL returns `201 Created` — auto-provision bookkeeping must not fail tenant creation, and the T-0811 control loop reconciles the row on a later tick regardless. Tenant identifier used is `credentials.schema_name` (== public name == username per T-0594).

**Test approach**: Both a pure-helper unit test AND a real-tenant HTTP test (the established pattern here — `test_tenant_runner_cache_lru_evicts_oldest` already creates real tenants via `POST /v1/tenants` with a uuid-suffixed name + `DELETE` cleanup, so a real create was practical rather than impractical).
- `test_initial_desired_count_clamp`: initial<limit→initial, initial>limit→limit (clamp), 0→0, limit=0→0.
- `test_create_tenant_auto_provisions_initial_desired`: `POST /v1/tenants` (unique `test_0812_<uuid>` name) then `GET /v1/tenants/{t}/fleet` asserts `desired_count == min(initial_agents, default_max_agents)` (== 1 in test state); DELETEs the tenant to clean up the shared DB.

**Validation**: `angreal check crate crates/cloacina-server` green (only pre-existing `cloacina` warnings). Both tests pass against the postgres lane (`cargo test -p cloacina-server --lib --features postgres -- test_initial_desired_count_clamp test_create_tenant_auto_provisions_initial_desired` → 2 passed).

**Files changed**: `crates/cloacina-server/src/routes/tenants.rs` (helper + best-effort write), `crates/cloacina-server/src/lib.rs` (AppState field + env read + both builders + 2 tests).

**Flag**: scope held to tenants.rs + AppState/lib wiring + tests; actuator/autoscaler/limits/desired-DAL, OpenAPI/SDK, and UI untouched. Not committed/pushed per instruction.