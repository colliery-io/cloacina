---
id: ws-6-non-cron-triggers-in-the
level: task
title: "WS-6 — Non-cron triggers in the Triggers view (event/poll/reactor)"
short_code: "CLOACI-T-0708"
created_at: 2026-06-16T01:50:18.600468+00:00
updated_at: 2026-06-16T04:13:43.092656+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-6 — Non-cron triggers in the Triggers view (event/poll/reactor)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P1) Surface **non-cron triggers** in the Triggers view — today only `CRON` appears,
so event/poll/reactor triggers (e.g. the `mixed-rust` package's reactor+trigger) are
invisible.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Triggers list shows all trigger types with a meaningful **Type** (cron / poll / event / reactor).
- [ ] A trigger **detail** view (rows reachable) showing what it fires + schedule/criteria.
- [ ] **enable/disable** and **run-now** controls where the API supports them.
- [ ] Re-passes the Playwright walk with a non-cron trigger present in the seed.

## Dependencies

May need a richer seed trigger ([[CLOACI-T-0710]]); confirm trigger-type + control
endpoints via [[CLOACI-T-0702]] if unclear.

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

### 2026-06-16 — DONE (poll triggers now visible + firing)

**Root cause found:** packaged custom-poll triggers were both invisible *and*
never fired. The reconciler's `step_load_custom_triggers` registered the
in-memory `Trigger` impl, but the trigger scheduler drives polling off
`get_enabled_triggers()` (the `schedules` rows) and the Triggers read API lists
those same rows — and nothing ever persisted a `trigger`-type row for poll
triggers. Cron triggers persisted via `register_cron_workflow`; the poll path
had no equivalent.

**Server/runtime fix (the substance of the ticket):**
- New reconciler step `step_persist_poll_schedules` + a `CronWorkflowRegistrar
  ::register_poll_trigger` method; `DalCronRegistrar` upserts a
  `NewSchedule::trigger` (poll interval + allow_concurrent). Wired into all
  three load paths (rust cdylib / python / CG); schedule ids tracked for unload
  next to the cron rows (deleted by id, same path).
- Added `poll_interval_ms` to `TriggerScheduleInfo` (detail response) +
  populated it; regenerated `openapi.json` and the `@cloacina/client` types.

**UI (WS-6 proper):**
- Triggers list and detail now show a meaningful **kind** — `cron` / `poll` —
  with a plain-language tooltip instead of the raw `schedule_type`. Schedule
  column shows the cron expression or `every 30s`. (Honest mapping: the
  `schedules` table only holds cron + poll; event/reactor triggers aren't
  schedules — they live on computation graphs and surface in the Graphs view.)
- Detail gains **Run now** (fires the bound workflow via the execute endpoint)
  and a read-only enable/disable badge with a tooltip explaining there's no
  toggle endpoint (a new server capability = an I-0124 non-goal).
- New `demo-poll-rust` fixture (poll trigger firing `demo_poll_workflow`),
  added to the demo seed.

**Verified live** (`ui/e2e/ws6.spec.ts`, screenshots in
`/tmp/cloacina-ui-uat/ws6/`): `demo_poll_workflow` lists as **POLL / every 30s**
beside the cron rows; the detail shows fires-workflow + poll interval + Run now;
`recent_executions` confirm the poll trigger now actually fires (~30s apart) —
proving the fix restored function, not just visibility.

**Acceptance:** list shows meaningful types ✓; detail shows what it fires +
schedule/interval ✓; run-now wired, enable/disable degraded with reason ✓;
Playwright walk re-passed with a non-cron trigger in the seed ✓.

**Backlog noted:** cron schedules use `create` (not upsert) at load, so
repeated `ui up --keep-db` reboots accumulate duplicate cron rows (cosmetic;
a fresh seed shows one). The poll path uses `upsert_trigger` and stays
singular. Worth aligning the cron path to upsert in a follow-up.

Committed `22061743` on `feat/ui-0124-server-read-endpoints`.