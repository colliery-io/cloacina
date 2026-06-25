---
id: ui-authz-gating-hide-write-admin
level: task
title: "UI authz gating — hide write/admin controls for read users (whoami + role)"
short_code: "CLOACI-T-0803"
created_at: 2026-06-24T12:34:17.869704+00:00
updated_at: 2026-06-24T13:10:57.418215+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI authz gating — hide write/admin controls for read users (whoami + role)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

The server correctly 403s write/admin ops for a `read` key, but the **UI still shows** Upload Package / Run / Fire / Inject / Poll buttons (and admin key/account controls) to read users — "don't display what isn't functional." Add a `whoami` endpoint so the UI learns the active key's role, carry role on the connection, and gate write controls (`canWrite`) + admin controls (`canAdmin`) so non-permitted actions aren't offered.

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

**2026-06-24 — foundation DONE + committed; component gating in flight (3 agents).** Server: `GET /v1/auth/whoami` → `{tenant_id, role, is_admin, name}` from the request `AuthenticatedKey` (any auth key, read level); authz table 51→52 + no-drift test; openapi registered + regenerated; `angreal check` + authz test clean. SDK: `whoami()` + regen. UI AuthContext: `Connection` carries `role`/`isAdmin` (resolved via whoami on connect/enterMemberships + a fallback for restored sessions); derives `canWrite` (write|admin|god) / `canAdmin` (admin|god) via a new `useCan()` hook; unknown role **fails closed**. UI tsc clean. **Gating (3 parallel agents, disjoint clusters):** (A) Workflows/WorkflowDetail/Upload/Triggers/TriggerDetail/ExecutionDetail/RunWorkflowModal — write; (B) GraphDetail/Graphs/GraphMiniCard/GraphInjectModal/TriggerFireModal — write; (C) Keys/Accounts — admin. Each typechecks before reporting; I review + commit + rebuild. **Live-verified earlier:** a read key gets 403 on POST workflows (upload), execute, accounts — server enforcement already correct; this task is the UI surface.

**2026-06-24 — gating COMPLETE + committed (with the design-sync refactor).** All 3 agent clusters landed; I fixed 2 gates the background linter had reverted (TriggerDetail Run, GraphInjectModal submit). **Entanglement resolved:** the gating was intertwined with a large in-progress **design-sync refactor** (migrate UI to `@colliery-io/aurora-dark`) which the user confirmed is **part of 0.9.0**. Reconciled: applied the refactor's 13 orphaned-file deletions (verified a closed import cluster — nothing outside imported them; typecheck still clean after removal), then committed design-sync + gating together (`feat(ui): aurora-dark … + role gating`). `tsc -b` + `vite build` clean; 57 files (42 mod, 13 del). Stale `stash@{0}` (Agent C's backup) now redundant — left for the user. **Rebuilding server (whoami) + ui (gating+design) images → redeploy → live-verify a read user sees no write/admin controls.**