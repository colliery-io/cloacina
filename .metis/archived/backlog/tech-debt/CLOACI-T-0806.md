---
id: ci-robustness-harden-the-remaining
level: task
title: "CI robustness — harden the remaining docker-compose test lanes against the postgres readiness race (exit 56)"
short_code: "CLOACI-T-0806"
created_at: 2026-06-25T12:03:18.070555+00:00
updated_at: 2026-07-06T00:04:37.972075+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# CI robustness — harden the remaining docker-compose test lanes against the postgres readiness race (exit 56)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Multiple docker-compose-based test lanes intermittently fail with **exit code 56** (transient connection failure) because they run `psql`/DB commands against Postgres before it is *stably* accepting connections. `pg_isready` passes during Postgres's **init-restart window**, then the server bounces, so the next command races a not-yet-stable server. This flaked the v0.9.0 release nightly (the UI Acceptance E2E `_fresh_database` DROP/CREATE, and a sqlite integration lane). The UI-e2e instance was hardened in **PR #145** (retry the idempotent DROP/CREATE 10×/2s). This task applies the same readiness-retry treatment to the *other* lanes so exit-56 stops costing release cycles.

### Type
- [x] Tech Debt — CI robustness

### Priority
- [x] P2 — recurring flake that has blocked a release; not user-facing

### Technical Debt Impact
- **Current Problems**: exit-56 transients intermittently fail any lane that does "compose up → immediate psql"; has blocked the 0.9.0 release nightly.
- **Benefits of Fixing**: reliable release/nightly gates; fewer spurious re-runs/re-cuts.
- **Risk Assessment**: low-risk change (retry/backoff around idempotent setup SQL).

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

- [ ] Audit the harnesses that bring Postgres up via `.angreal/docker-compose.yaml` and issue early DB/psql commands, and give each a consistent readiness-retry/backoff (mirror PR #145's `_fresh_database`):
  - [ ] `.angreal/test/e2e/cli.py` `_start_postgres` — strengthen the shared `pg_isready` wait (require **N consecutive** successes or a short settle) so callers don't race the init-restart bounce. Highest leverage — it's the shared entrypoint.
  - [ ] `.angreal/test/integration.py` — the "Waiting for PostgreSQL…" path (a sqlite integration lane hit exit 56 in run 28125071912).
  - [ ] `.angreal/test/e2e/compiler.py` — verify its `pg_isready` loop survives the bounce.
  - [ ] `.angreal/test/e2e/sdk_contract.py` — brings up the stack + psql; same exposure.
- [ ] Prefer a single shared retry helper over per-file copies.
- [ ] No remaining "compose up → immediate psql with check=True" call sites.

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

**2026-06-25 — filed** during the v0.9.0 release recovery. Reference fix already landed: **PR #145** added a 10×/2s retry to `_fresh_database` in `.angreal/test/e2e/ui_e2e.py`. Exit-56 occurrences this session: UI Acceptance E2E `_fresh_database` psql (fixed), Integration Tests (sqlite, ubuntu) in run 28125071912 (RuntimeError/exit 56; the tests themselves passed). Strengthening `cli.py:_start_postgres` is the highest-leverage single change since it's the shared entrypoint. (Note: a *separate* nightly-harness gap — the UI e2e server not seeding the `acme` tenant-admin key — was fixed in PR #146; that's not this task.)

### 2026-07-05 — DONE + LIVE-PROVEN (branch fix/t0806-pg-readiness-race)
Two shared helpers in `.angreal/test/_utils.py`: `wait_for_postgres_stable` (requires N **consecutive** `pg_isready` successes, 1s apart — a single pass can land inside the init-restart bounce) and `psql_retry` (idempotent-DDL retry, the PR #145 pattern lifted out of its one copy). Rewired **eight** call sites — the AC's five (`e2e/cli.py _start_postgres` [shared entrypoint], `integration.py` [blind `sleep(30)` → stable wait, also faster], `e2e/compiler.py`, `e2e/sdk_contract.py _fresh_database` [was raw `check=True`], `e2e/ui_e2e.py` [deduped onto the helper]) **plus three more single-success loops the audit list missed**: `auth.py`, `metrics_format.py`, `e2e/ws.py`. No remaining "compose up → immediate psql check=True" sites (swept).

**LIVE PROOF**: 3 complete fresh-volume init cycles (`down -v` → `up` → stable-wait → IMMEDIATE DROP/CREATE — the exact window where the bounce lives): stable at ~3.8s each (vs the 30s blind sleep), DROP/CREATE ok every time, zero exit-56. All 9 touched files parse; imports match each file's conventions. COMPLETE.