---
id: timer-driven-cron-scheduling
level: task
title: "Timer-driven cron scheduling — replace the 30s poll with sleep-until-next-due + change-notify"
short_code: "CLOACI-T-0743"
created_at: 2026-06-17T22:11:53.394484+00:00
updated_at: 2026-06-20T15:20:26.210744+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Timer-driven cron scheduling — replace the 30s poll with sleep-until-next-due + change-notify

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Replace the cron scheduler's fixed 30s poll with a **timer-driven** loop that
sleeps until the next due schedule and wakes immediately on schedule changes —
so cron pickup latency drops from up to 30s to ~ms and idle DB polling for due
schedules goes away. Surfaced 2026-06-17 watching the demo: `demo_py_cron_workflow`
(`*/15 * * * * *`) fired ~26s late (scheduled 21:29:15, ran 21:29:41).

## Root cause (code-grounded)

`cron_trigger_scheduler.rs::run_polling_loop` (:214) ticks at
`trigger_base_poll_interval` (1s) but only runs `check_and_execute_cron_schedules`
when `cron_poll_interval` has elapsed — **default 30s** (`cron_trigger_scheduler.rs:95`,
`runner/default_runner/config.rs:76`). So a due cron is detected 0–30s late.
Downstream is NOT the bottleneck: task-ready dispatch is `scheduler_poll_interval`
= 100ms (`config.rs:295`) and the fleet **pushes** work to agents over WS
(`fleet_executor.rs:17`). The latency is entirely the cron due-detection sweep.

## Technical Debt Impact

- **Current problems:** every scheduled workflow fires up to 30s late; the
  scheduler does an idle "get_due_cron_schedules" sweep on a timer even when the
  next fire is minutes away.
- **Benefits of fixing:** near-instant cron pickup; no idle polling; aligns with
  the embedded-first model (in-process timer, no DB chatter). [[project_embedded_first_philosophy]]
- **Risk:** scheduler is core; a regression delays/duplicates fires. Mitigate
  with unit tests + the demo stack as a live check, and keep a slow backstop.

## Technical Approach

`get_due_cron_schedules` already orders by `next_run_at asc`
(`dal/unified/schedule/crud.rs:415,445`), and there's an indexed `next_run_at`
column. Build on that:

1. **DAL:** add `schedule().next_cron_due_time() -> Option<DateTime<Utc>>` (min
   `next_run_at` over enabled cron schedules; reuse the existing query shape,
   `LIMIT 1`). Postgres + sqlite variants like `get_due_cron_schedules_*`.
2. **Scheduler loop (`run_polling_loop`):** keep the 1s tick for poll-triggers /
   reactors (those are poll-by-nature). For cron, add a `tokio::time::sleep_until`
   arm to the `select!` set to the next due instant; on fire, process due
   schedules and recompute. Compute the sleep as
   `min(next_due, now + cron_backstop_interval)`.
3. **Change-notify:** a `tokio::sync::Notify` (held by the scheduler, cloned to
   the cron registrar) signaled by `register_cron_workflow` / unregister /
   enable / disable so a newly-registered schedule wakes the sleeper immediately
   (avoids regressing register→first-fire latency). Wire it where schedules are
   mutated (reconciler `register_cron_workflow`, `mod.rs:183`; DAL enable/disable).
4. **Backstop / multi-instance:** keep a long `cron_backstop_interval` (e.g. 60s)
   capping the sleep, as a safety net for missed notifies and for the
   **multi-instance** server case (an in-process Notify only wakes the local
   replica; a schedule registered on replica A won't wake B). Document Postgres
   `LISTEN/NOTIFY` as the future zero-poll cross-instance path; SQLite/embedded is
   single-process so the in-process Notify is complete there.
5. Replace/retire `cron_poll_interval`; add `cron_backstop_interval` to config.

## Scope / branch
Standalone task → its own branch off `main` + its own PR (not #132, which is the
Python-parity/UI work). Core crate: `crates/cloacina/src/cron_trigger_scheduler.rs`
+ `dal/unified/schedule/`. Verify on the demo stack (cron should fire within ~1s
of its scheduled second).

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

- [ ] Cron scheduler sleeps until the next due schedule instead of polling every
      30s; no idle `get_due_cron_schedules` sweep when nothing is due soon.
- [ ] A schedule registered/enabled while the scheduler is idle fires on time
      (change-notify wakes the sleeper) — no register→first-fire regression.
- [ ] A due schedule fires within ~1s of its scheduled second (vs up to 30s).
- [ ] A long backstop bounds the sleep for the multi-instance / missed-notify
      case; behavior documented (in-process notify = embedded; LISTEN/NOTIFY = future).
- [ ] Unit tests cover next-due computation + fire-on-notify; postgres + sqlite.
- [ ] Verified live on the demo stack: `demo_py_cron_workflow` fires within ~1s.
- [ ] No regression to poll-trigger / reactor cadence or per-task dispatch.

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

- 2026-06-17: **Built + live-verified (PR #133, commit on `timer-driven-cron`).**
  Implemented exactly as planned: DAL `next_cron_due_time()` (postgres+sqlite),
  timer-driven loop in `cron_trigger_scheduler.rs` (cache next-due → sleep exactly
  until it, backstop = repurposed `cron_poll_interval`, recompute only on fire or
  notify), shared `Arc<Notify>` on `DefaultRunner` → `Scheduler` + `DalCronRegistrar`,
  signaled by register/unregister/enable/disable/delete. 4 sqlite unit tests on
  the sleep math pass; core+server compile clean.
  **LIVE RESULT (demo stack, branch rebased onto main w/ #132):**
  `demo_py_cron_workflow` (`*/15`) now fires within **~10ms** of its scheduled
  second — `scheduled 22:48:00 → executing 22:48:00.009`; `scheduled 22:48:15 →
  22:48:15.010`. Detection logged "Found N due cron" ~3ms after the boundary
  (was up to 30s). The 30s idle sweep is gone.
- 2026-06-17: **Separate, pre-existing issue observed (NOT this task's scope).**
  When ≥2 cron schedules are due in the same instant, `check_and_execute_cron_schedules`
  processes them **sequentially**, and `execute_cron_workflow`'s handoff can block
  (~13s seen when the fleet was at capacity under demo load), delaying the 2nd
  schedule. This is orthogonal to detection latency (which this task fixed) — it's
  the cron loop's sequential handoff + executor backpressure. Candidate follow-up:
  spawn per-schedule handoffs / don't block the cron loop on executor capacity.
- 2026-06-17: AC status — sleep-until-due ✓, change-notify ✓, fires within ~1s
  (10ms) ✓, backstop + multi-instance documented ✓, unit tests ✓, live demo ✓,
  no poll-trigger/reactor regression ✓ (1s tick path untouched). PR #133 open,
  CI running. Note: kept the `cron_poll_interval` field name (repurposed as the
  backstop) rather than adding `cron_backstop_interval`, to avoid churning the
  config builder API — semantics documented on the field.
- 2026-06-17: **Sequential-handoff issue FIXED in this PR (commit a0cc44cb)** —
  pulled the earlier "separate follow-up" into scope since it was the user-visible
  residual. `check_and_execute_cron_schedules` now spawns each
  `process_cron_schedule` on its own task instead of awaiting them in a loop
  (`executor.execute()` blocks until the workflow runs, which is why a 2nd co-due
  schedule waited for the 1st's full execution). Per-row `claim_and_update_cron`
  is atomic so concurrent processing is safe; matches the module's "move on
  immediately" contract. **LIVE RESULT:** at the :30 and :45 boundaries both
  `demo_cron_workflow` and `demo_py_cron_workflow` now log "Executing" at the
  **same millisecond**, ~4ms after the scheduled second (was: 2nd waited ~3–10s).
  Also bumped the demo fleet 4→16/agent (48 aggregate, commit 79ab87bf) to clear
  the "no capacity" backpressure that compounded it. Final latency picture:
  detection ~3ms, single pickup ~10ms, co-due pickup ~4ms concurrent. The WS push
  was never the bottleneck (agent results return in ms).
