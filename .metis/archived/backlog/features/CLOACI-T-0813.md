---
id: ui-tenant-agent-management
level: task
title: "UI — tenant agent management (provision/deprovision + fleet/limit state)"
short_code: "CLOACI-T-0813"
created_at: 2026-06-27T14:43:40.667620+00:00
updated_at: 2026-06-27T18:04:05.937261+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# UI — tenant agent management (provision/deprovision + fleet/limit state)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The UI tenant-agent-management surface (slice 1 #6): provision/deprovision agents + view the tenant pool (agents, health, capacity), the effective limit, and autoscaler state. Role-gated via whoami/useCan — tenant-admin writes; read users see state but not controls.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] A tenant-admin provisions/deprovisions an agent from the UI and sees the pool update (agent joins/drains); the effective limit + autoscaler state are shown.
- [ ] Read-scope users see the fleet state but the provision/deprovision controls are hidden/disabled (matches server authZ; no client-only trust).

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

### 2026-06-27 — Fleet management UI implemented (ui/ only)

Built the tenant agent-fleet page against the shipped T-0808/0809 endpoints. Branch `i0127-agent-control-plane`. No `crates/` touched.

**Files**
- `ui/src/api/fleet.ts` (new) — `useFleet()`, `useTenantLimit()`, `useProvision()`, `useDeprovision()`. Hand-written fetch (endpoints not in the generated client yet) reusing the SAME auth mechanism as the generated client: the active `connection`'s bearer key (`Authorization: Bearer <apiKey>`) against `connection.serverUrl`, via `useAuth()`. Throws `CloacinaApiError` on non-2xx so `classifyError` + the 409 check work identically to the generated helpers. Mutations invalidate `["fleet", tenant]`.
- `ui/src/routes/Fleet.tsx` (new) — stat row (Provisioned / Running / Effective limit), admin-only Provision +1 / Deprovision −1 controls, read-only limit-source line (override vs default), loading/error Alerts. Mirrors `Accounts.tsx` styling (var(--fg)/--muted/--sidebar/--border, TOKEN.ice).
- `ui/src/App.tsx` (mod) — `/fleet` route under `<RequireAuth><Shell>`.
- `ui/src/components/Shell.tsx` (mod) — "Agent fleet" nav item (IconServer) in the System group.
- `ui/e2e/fleet.spec.ts` (new) — stack-gated smoke (acme tenant-admin scales up then down); NOT run here (needs live stack).

**Role-gating**: `useCan().canAdmin` gates the controls — read users see the stats but get an "admin access" Alert instead of buttons. Server enforces it regardless (no client-only trust).

**409 / capacity**: `atCapacity = desired_count >= effective_limit` disables Provision and shows an "At capacity (N)" hint; a returned 409 is surfaced as a neutral "Fleet is at capacity…" Alert (detected via `classifyError(err).status === 409`). Deprovision disabled at desired_count ≤ 0 (floor).

**Validation (npm; repo has no pnpm)** — all green:
- `npm run typecheck` (`tsc -b --noEmit`) → clean.
- `npm run build` (`tsc -b && vite build`) → built OK.
- `npm run lint` (`eslint .`) → my 5 files clean (`npx eslint` on them exits 0). Repo-wide lint shows 31 PRE-EXISTING errors in untouched files (ws6/7/8 specs empty catches; `react-hooks/exhaustive-deps` rule-not-found in AuthContext/Connect/ExecutionDetail) — not introduced here.

Not committed/pushed.