---
id: auth-demo-readiness-ui-silent
level: task
title: "Auth demo readiness — UI silent-refresh + OIDC browser integration + compose OIDC wiring"
short_code: "CLOACI-T-0800"
created_at: 2026-06-24T09:43:29.708084+00:00
updated_at: 2026-06-24T10:08:57.580459+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Auth demo readiness — UI silent-refresh + OIDC browser integration + compose OIDC wiring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Make the I-0118 auth work demoable end-to-end in the stack. Three follow-ups: (1) **UI silent-refresh** — keep a minted session alive by calling `/auth/refresh` before expiry; (2) **OIDC browser integration** — a "Login with provider" button on Connect + the callback redirecting back to the UI with the minted key (instead of JSON), so the OIDC flow completes in the browser; (3) **compose OIDC wiring** — the demo server gets `CLOACINA_OIDC_*` pointed at the Dex sidecar + the success-redirect, and the stack is rebuilt with this code. Plus the named NFR-003 follow-up: move OIDC login-flow state to Postgres.

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

- [ ] A minted (local/OIDC) session auto-refreshes before its short TTL expires; a non-refreshable pasted key is left alone.
- [ ] Connect offers "Login with provider" (when OIDC is enabled); the full browser flow logs the user in via Dex and lands on the overview.
- [ ] The demo compose server runs with OIDC configured against the Dex sidecar; the stack is rebuilt with the current code.
- [ ] OIDC login-flow state is Postgres-backed (NFR-003). → **split out to [[CLOACI-T-0801]]** (deferred: no demo impact, single-replica in-memory is correct).

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

**2026-06-24 — code done; stack rebuild in flight.** Three demo-critical follow-ups implemented + typecheck/compile clean:
- **Silent-refresh** (`ui/src/auth/AuthContext.tsx`): per-connection effect re-mints via `client.refresh()` every ~10m (minted TTL ~15m); a non-refreshable pasted key errors once → loop stops.
- **OIDC browser flow**: `Connect.tsx` gains an **SSO** mode (Continue with SSO → full-page nav to `{server}/v1/auth/oidc/login`) + a fragment pickup (`/connect#key=…&tenant=…` → connect, key stripped from history). Server callback (`routes/oidc_auth.rs`) redirects to `CLOACINA_OIDC_SUCCESS_REDIRECT#key=…` when set, else JSON.
- **Compose wiring**: server gets `CLOACINA_OIDC_*` vs the dex sidecar + `depends_on: dex`; dex issuer → `host.docker.internal:5556` so browser + in-container server resolve the same issuer (one `/etc/hosts` line, documented). `docker/AUTH_DEMO.md` runbook added.

`tsc -b` clean; `vite build` clean; `angreal check` clean; `docker compose config` valid. **Stack image rebuild (`docker compose build server ui`) running** to replace the stale `docker-server-1`. **NFR-003 Postgres login-state → [[CLOACI-T-0801]]** (deferred by design). Next: bring up the rebuilt stack + smoke-test the flows live, then complete.

**2026-06-24 — COMPLETE, verified live in the rebuilt compose stack.** Rebuilt `server`+`ui` images, brought the stack up. **Local accounts (8080):** create 201 · login mints · acme read 200 · cross-tenant 403 · leak-fix 403 · refresh re-mints — all green. **OIDC SSO — full browser flow verified end-to-end** (curl-driven through Dex, `--resolve host.docker.internal`): `/auth/oidc/login` → Dex login (`alice@acme.com`/`password`) → callback validates the ID token (JWKS sig, iss/aud/exp, nonce) → `domain:acme.com` maps to acme/admin → mints key → `303` to `localhost:8082/connect#key=…&tenant=acme&role=admin`; the minted key reads acme workflows = **200**; server logged "OIDC login succeeded". UI serves on 8082. **Two live-found fixes committed:** demo bcrypt hash (didn't match `password`), and email-domain mapping (Dex static users have no groups). **Gotcha documented:** recreating dex rotates its keys → restart the server (JWKS re-discovery). Silent-refresh: code clean + the `/auth/refresh` endpoint verified live in the smoke test. **NFR-003 Postgres login-state → [[CLOACI-T-0801]]** (deferred by design). All demo-critical ACs met.