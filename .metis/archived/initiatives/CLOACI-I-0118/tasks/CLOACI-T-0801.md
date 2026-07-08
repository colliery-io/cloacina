---
id: oidc-login-flow-state-postgres-nfr
level: task
title: "OIDC login-flow state → Postgres (NFR-003 multi-replica)"
short_code: "CLOACI-T-0801"
created_at: 2026-06-24T09:55:04.857631+00:00
updated_at: 2026-06-24T10:37:07.393693+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC login-flow state → Postgres (NFR-003 multi-replica)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Move the OIDC in-flight login state (state → nonce + PKCE verifier) from the in-memory `LoginFlowStore` (`crates/cloacina-server/src/oidc.rs`) to Postgres so the login flow is multi-replica safe (NFR-003) — no sticky sessions. **Deferred from T-0800** (demo readiness): in-memory is correct for single-replica; this is robustness only, no demo impact.

**Plan:** (1) migration `035_create_oidc_login_flows` (postgres-only: `state` PK, `nonce`, `pkce_verifier`, `expires_at TIMESTAMPTZ`) + schema `table!` block, mirroring `033_create_oidc_sessions`; (2) DAL `crates/cloacina/src/dal/unified/oidc_login_flows` (put / take-single-use `DELETE … WHERE state=$1 AND expires_at>now() RETURNING …` / sweep); (3) make `LoginFlowStore` hold `Option<Database>` — `Some` → DAL, `None` → keep the in-memory fallback (so `test_state` stays DB-free); wire the real DB at the main AppState construction. `put`/`take` are already async, so handler signatures don't change. Consider encrypting the PKCE verifier at rest (reuse `crypto::encrypt_private_key` like oidc_sessions), though it's a 10-min single-use secret.

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

- [ ] OIDC login-flow state is persisted in Postgres and read back across replicas (no sticky sessions).
- [ ] State is single-use + TTL'd; expired/replayed/unknown `state` fails closed.
- [ ] Tests (`test_state`) still construct without a live DB (in-memory fallback retained).

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

**2026-06-24 — IMPLEMENTED, `angreal check` clean; live-verify pending stack rebuild.** Per plan: migration `035_create_oidc_login_flows` (postgres-only — `state` PK, `nonce`, `pkce_verifier`, `created_at`, `expires_at`) + schema `table!` block; DAL `crates/cloacina/src/dal/unified/oidc_login_flows` (`put` / single-use `take` = `DELETE … WHERE state AND expires_at>now RETURNING (nonce, pkce_verifier)` / `sweep_expired`) registered in `dal/unified/mod.rs`. `LoginFlowStore` now holds `Option<Database>`: `with_db()` → DAL (wired at the main AppState build via `runner.database().clone()`), `new()` → in-memory fallback (kept for `test_state`); `put`/`take` signatures unchanged so handlers are untouched. **Decision:** PKCE verifier + nonce stored as-is (minutes-short, single-use, deleted on consume) rather than encrypted — `expires_at` bounds them and `take` fail-closes on expiry. **No sweeper loop wired** (none exists; oidc_sessions isn't swept either — `sweep_expired` is there for a future hygiene task; correctness doesn't depend on it). Compiles clean.

**2026-06-24 — COMPLETE, verified live in the compose stack.** Rebuilt + brought up the stack. Migration created `public.oidc_login_flows` (state PK, nonce, pkce_verifier, created_at, expires_at + `idx_…_expires_at`). Drove the OIDC flow watching the table: rows **1 → 2 after `/auth/oidc/login` (state persisted to Postgres) → 1 after the callback** — the single-use `take` (`DELETE … RETURNING`) consumed exactly the row it used. State now survives across replicas (NFR-003); no process-memory dependency. All 3 ACs met (persisted + read back; single-use/TTL'd/fail-closed; `test_state` keeps the in-memory fallback).