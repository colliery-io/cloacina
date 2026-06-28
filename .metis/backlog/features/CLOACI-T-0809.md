---
id: per-tenant-desired-agent-count
level: task
title: "Per-tenant desired agent count + provision/deprovision REST API"
short_code: "CLOACI-T-0809"
created_at: 2026-06-27T14:43:35.484772+00:00
updated_at: 2026-06-27T18:04:02.348061+00:00
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

# Per-tenant desired agent count + provision/deprovision REST API

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Per-tenant desired-agent-count state + the provision/deprovision REST API (slice 1 #2). A tenant-admin provisions/deprovisions agents for THEIR OWN tenant, bounded by the effective limit from T-0808. The UI 'provision an agent' action calls this.

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

- [ ] POST/DELETE (or PATCH) adjusts the tenant's `desired_count`; provisioning past the effective limit is rejected (4xx); `desired_count` is persisted.
- [ ] Tenant-scoped: a caller cannot provision/deprovision for another tenant — denied server-side via the I-0118 fail-closed ABAC route table (NFR-004), regardless of substrate.
- [ ] `desired_count` is the single input the actuator (T-0810) and autoscaler (T-0811) reconcile against.

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

### 2026-06-27 — Implemented + green

Mirrored the T-0808 vertical slice exactly. All layers implemented and tests pass locally against Postgres.

**Files created**
- `crates/cloacina/src/database/migrations/postgres/037_create_agent_desired_counts/{up,down}.sql` — `agent_desired_counts(tenant_id PK, desired_count INT DEFAULT 0, updated_at TIMESTAMPTZ DEFAULT now())`.
- `crates/cloacina/src/dal/unified/agent_desired/mod.rs` — `AgentDesiredDAL`: `get_desired` (absent → 0), `set_desired` (upsert; `updated_at` left to DB default; `NaiveDateTime` read-only row field). Postgres-only.
- `crates/cloacina-server/src/routes/fleet.rs` — `GET /fleet`, `POST /fleet/provision` (409 at-capacity), `POST /fleet/deprovision` (floor 0). DTO `FleetScaleInfo` inline. Not added to central OpenApi `paths()` (same deferral as T-0808).
- `crates/cloacina/tests/integration/agent_desired.rs` — 3 DAL tests.

**Files modified**
- `crates/cloacina/src/database/schema.rs` — `agent_desired_counts` `table!` (postgres module).
- `crates/cloacina/src/dal/unified/mod.rs` — `pub mod agent_desired`, re-export, `DAL::agent_desired()`.
- `crates/cloacina/tests/integration/main.rs` — `pub mod agent_desired`.
- `crates/cloacina-server/src/routes/mod.rs` — `pub mod fleet`.
- `crates/cloacina-server/src/lib.rs` — 3 routes after limits; 7 HTTP functional tests.
- `crates/cloacina-server/src/routes/authz.rs` — 3 entries (provision/deprovision tenant-admin, GET tenant-read); table size 55 → 58 + assertions.

**actual_count** uses `state.agent_registry.snapshot()` filtered by `AgentRecord.tenant_id == Some(tenant_id)`. The `AgentRegistry` (T-0631) is an in-memory per-replica roster, so this is the local-replica view (documented in the DTO + route doc).

**Tests (all green)**
- DAL integration: 3 passed (`agent_desired`).
- Server HTTP functional: 7 fleet tests passed (provision increments, 409 at-capacity, deprovision floor, GET fields, cross-tenant 403, god any-tenant 200, no-auth 401).
- `authz_table_classifies_known_routes`: passed at 58.
- `angreal check crate` clean for both crates.

Not committed (per instructions — awaiting review).