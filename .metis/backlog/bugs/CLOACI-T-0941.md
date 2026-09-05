---
id: ops-metrics-ws-ping-pong-dueling
level: task
title: "Ops-metrics WS ping-pong — dueling subscribers keep every session's health tiles at connecting"
short_code: "CLOACI-T-0941"
created_at: 2026-09-05T15:12:56.231439+00:00
updated_at: 2026-09-05T17:53:50.016162+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Ops-metrics WS ping-pong — dueling subscribers keep every session's health tiles at connecting

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The Overview health tiles (and anything else reading `use_ops_metrics()`)
show "connecting…" forever in EVERY session since the Leptos port — the warm
ops WS never delivers a snapshot. Found by T-0940's screenshot pass.

DIAGNOSIS (verified live on the demo stack, 2026-09-05):
1. `ops.rs provide_ops_metrics()`'s Effect tracks `auth.connection()`, which
   reads the WHOLE `connections` Vec. At startup the T-0803 whoami role
   resolve calls `auth.patch(...)` → the Vec changes → the Effect refires and
   spawns a SECOND subscriber loop.
2. The superseded loop cannot die: its generation check runs only when a
   frame arrives, and `SubscribeOptions::default().reconnect = true` means
   the client-internal loop silently reconnects on close without surfacing a
   frame.
3. The server allows one consumer per delivery recipient — each loop's
   reconnect closes the other's socket. Result: eternal ~2s ping-pong
   (observed: 1551 `ops_metrics:global` upgrades in one afternoon; Playwright
   `page.on(websocket)` shows open/welcome/open/close cycles; NO push frame
   ever received because the emitter cadence is 5s > the ~2s socket life).
4. The wire itself is healthy: a raw node probe (with or without the hello
   frame) receives welcome + pushes within 48ms and stays open indefinitely.

FIX (drafted during T-0940 then deliberately deferred; reconstruct from this):
- Key the effect on connection IDENTITY via a Memo over
  `(label, server_url, api_key, tenant)` so metadata patches (role/is_admin)
  don't refire it.
- Disable the stream-internal reconnect (`SubscribeOptions { reconnect:
  false, .. }`) and reconnect in the app-level loop with a 2s sleep, checking
  the generation before AND after — a closed socket then ends the stream and
  the zombie exits at the next iteration.
- Audit the same pattern in execution_detail.rs's live-event follow (it uses
  the same generation-check-between-frames idiom; it usually escapes because
  exec streams are chatty, but the zombie-reconnect flaw is identical).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: every web-UI session (Overview health row, Operations live tiles/pill degrade to polling-only or stay "connecting…")
- **Reproduction Steps**:
  1. `angreal ui up`; connect at localhost:8080 with the demo bootstrap key
  2. Watch the Overview service-health row — stays "connecting…" indefinitely
  3. `docker logs cloacina-demo-server-1 | grep -c "upgrade accepted.*ops_metrics"` climbs ~every 2s while a session is open
- **Expected vs Actual**: one warm ops WS delivering a snapshot ≤5s and staying connected vs dueling subscribers closing each other's sockets forever

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

- 2026-09-05: Fix applied on fix/t0941-ops-ws-pingpong (ops.rs): Memo keys the
  effect on (label, server_url, api_key, tenant) with the inner connection()
  read UNTRACKED (a tracked read would re-subscribe the whole Vec and defeat
  the memo); client-internal reconnect disabled, app-level retry loop with 2s
  sleep and generation checks before/after. wasm compiles clean. Demo stack
  rebuilding for live verification (expect: single WS, push ≤5s, tiles green).
- Audit finding (follow-up, NOT in this PR): execution_detail.rs's
  follow_execution_events has the same generation-check-between-frames idiom
  with internal reconnect. No ping-pong (per-exec recipients don't collide)
  but a superseded stream on a TERMINAL exec never gets a frame → zombie
  socket persists per viewed execution (leak). Fold into a small follow-up.