---
id: warm-up-live-ops-metrics-ws-app
level: task
title: "Warm up live ops-metrics WS app-wide — no cold-start flash on live pages"
short_code: "CLOACI-T-0774"
created_at: 2026-06-22T17:54:57.435893+00:00
updated_at: 2026-06-22T18:04:25.577437+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Warm up live ops-metrics WS app-wide — no cold-start flash on live pages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Live pages flashed "connecting…" / "down"-looking empty states on every visit:
`useLiveOpsMetrics` (the T-0718 ops-metrics WS) was per-page (Overview + Operations)
and reset to `null` + reconnected the WS on every mount — so the first packet's
latency was paid on every navigation, reading as "the server is down."

Fix: an app-level `OpsMetricsProvider` mounted in the Shell (always-on, wraps the
Outlet). The single WS connects at login and stays warm for the session, retaining
the last snapshot across navigation (stale-while-revalidate — never blanks to
null on reconnect). Pages read it via `useOpsMetrics()` from context. Only the
one-time first-login connect can show a brief null. (`operations.ts`→`.tsx` for JSX.)

## Status Updates **[REQUIRED]**

- 2026-06-22: Implemented. Provider + `useOpsMetrics()` in api/operations.tsx;
  Shell wraps the Outlet; Overview + Operations switched off the per-page hook.
  typecheck + build green. Verifying live.
- 2026-06-22: Diagnostic screenshot exposed the real cold-start was server-side:
  the ops-metrics publisher (ops_metrics.rs) was a single global 5s ticker gated
  on subscribers, so a fresh connection waited up to 5s for the next tick. Added a
  1s subscriber-poll that pushes an immediate snapshot on first connect, then keeps
  the 5s cadence (e130cbce). Measured first-frame: ~1.6s (was ≤5s).
- 2026-06-22: DONE (f10ddea9 client + e130cbce server). Verified live via SPA
  navigation: Overview (warm) → Workflows → Operations shows full live metrics
  instantly (150ms; "live", all tiles + agents populated, no "connecting…"). NB:
  earlier failed checks used page.goto() which forces a full reload — not how the
  app navigates; with real link clicks the Shell+provider persist and stay warm.
  Known limitation: the immediate push fires on the FIRST subscriber; a 2nd
  concurrent UI joining mid-interval waits for the next 5s tick (fine for single-
  operator; upgrade to a per-subscribe notify if multi-client instant is needed).

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