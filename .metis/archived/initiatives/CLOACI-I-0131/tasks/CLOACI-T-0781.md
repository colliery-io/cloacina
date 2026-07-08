---
id: execution-isolation-per-tenant
level: task
title: "Execution isolation — per-tenant runner namespaces tasks + tenant-scoped artifact fetch"
short_code: "CLOACI-T-0781"
created_at: 2026-06-23T03:33:43.441695+00:00
updated_at: 2026-06-23T12:40:39.868771+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Execution isolation — per-tenant runner namespaces tasks + tenant-scoped artifact fetch

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Multi-tenancy demo (T-0779) revealed runtime execution isn't tenant-isolated: a
tenant's tasks were namespaced `public::pkg::wf::task` regardless of tenant
(reconciler default_tenant_id hardcoded "public" in services.rs:200), so they
resolved to task_tenant=None, routed to the public/admin agents, and couldn't
fetch their cdylib (which lives in the tenant's schema) — tenant runs hung in
Running. Make execution tenant-isolated: a tenant's tasks namespace under it,
route to its agents, fetch from its schema.

## Plan

- DefaultRunnerConfig gains `tenant_id` (default "public") + accessor/setter;
  services.rs ReconcilerConfig.default_tenant_id = self.config.tenant_id();
  tenant_runner_cache stamps it (config.set_tenant_id(tenant)) per tenant runner.
- Agent artifact endpoint (fetch_artifact) resolves the requesting agent's tenant
  schema (auth.tenant_id → tenant_databases.resolve), so a tenant agent fetches
  its cdylib from its own schema (was state.database / admin only).
- Fleet executor is already per-tenant (registrar builds tenant-scoped DAL), so
  dispatch resolution + agent filtering (a.tenant_id == task_tenant) already work.

## Status Updates **[REQUIRED]**

- 2026-06-23: Found via T-0779 (acme runs stuck Running; task ns = public::…).
- 2026-06-23: DONE + verified end-to-end. The peel was FIVE layers: (1) task
  namespacing — DefaultRunnerConfig.tenant_id → reconciler default_tenant_id
  (per-tenant runner stamps it) → acme tasks namespace acme:: and route to acme
  agents; (2) dispatch artifact lookup — tenant-schema packages carry tenant_id=NULL
  (schema is the isolation), so the schema-scoped DAL looks up by None not the
  namespace tenant; (3) agent artifact fetch — resolve the requesting agent's tenant
  schema; (4) WorkPacket enqueue → ADMIN delivery outbox (delivery is server-global:
  one relay drains the admin schema, woken by the admin NOTIFY) via a separate
  outbox_dal on the fleet executor; rows stay tenant-tagged for WS routing.
  VERIFIED: all 3 acme workflows (billing/payroll/fulfillment) Complete on acme
  agents; public still Completes (no regression). First fully tenant-isolated
  execution. Commits: namespace+fetch b22pgyjcd, dispatch lookup bna56d1e4, outbox
  b2e18hoaq.

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

*To be added during implementation*