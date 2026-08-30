---
id: cron-backstop-is-starved-by-the-1s
level: task
title: "Cron backstop is starved by the 1s trigger tick — a cron schedule created without a notify never fires"
short_code: "CLOACI-T-0928"
created_at: 2026-08-08T18:54:57.745724+00:00
updated_at: 2026-08-30T02:12:25.824714+00:00
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

# Cron backstop is starved by the 1s trigger tick — a cron schedule created without a notify never fires

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The unified scheduler's cron **backstop can never elapse**, so cron firing
depends entirely on the `cron_change` notification. Any cron `schedules` row
written without one sits there and never fires.

Found while building the [[CLOACI-T-0927]] live-stack lane: an instance created
over HTTP was correct in the DB (`enabled=t`, `paused=f`,
`cron_expression='*/2 * * * * *'`, `next_run_at` set ~2s out) and simply never
fired within 120s. The server log shows the cause plainly — over ~166s the
scheduler logged `Checking trigger schedules` **167 times** and the cron branch
**zero** times, after the single startup line
`Starting unified scheduler (cron interval: 30s, trigger base interval: 1s)`.

## Mechanism

`crates/cloacina/src/cron_trigger_scheduler.rs::run_polling_loop` (~line 280):

```rust
loop {
    let cron_delay = self.cron_sleep_delay(next_cron_due);
    let cron_sleep = tokio::time::sleep(cron_delay);   // NEW sleep every iteration
    tokio::pin!(cron_sleep);
    tokio::select! {
        _ = interval.tick() => { /* triggers, every 1s */ }
        _ = &mut cron_sleep => { /* cron */ }
        _ = self.cron_change.notified() => { ... }
        _ = self.shutdown.changed() => { ... }
    }
}
```

`interval` persists across iterations and fires every **1s**; `cron_sleep` is
**recreated from zero on every iteration** with a delay of up to 30s. The tick
therefore always wins, the loop restarts, and a fresh 30s sleep begins. The
cron branch is reachable only when `cron_delay` is shorter than the time to the
next tick — in practice only when it is **zero**, i.e. when `next_cron_due` is
already in the past.

And `next_cron_due` is refreshed in only two places: inside the cron branch
itself (unreachable, per above) and on a `cron_change` notification.

**CORRECTION (2026-08-09), after review pushback — the original framing here
overstated this.** The first version of this ticket said cron scheduling is
"effectively dead" in a process that starts with no cron schedules. That is
WRONG, and the reasoning was sloppy: a process with nothing scheduled *should*
idle, and every in-process path that creates a schedule DOES notify. Verified:

* embedded `register_cron_*` / `unregister_*` — `cron_api.rs` notifies;
* the reconciler — `services.rs:267` wires a `DalCronRegistrar` with the
  `cron_change` handle, so packaged `#[trigger(cron = ...)]` declarations
  notify at load time;
* the server's named-instance routes — notify as of T-0927.

Also note a known future due-time still fires roughly on time even with the
starvation: the delay shrinks on each 1s iteration until it reaches zero, at
which point the ready sleep is selected. The starvation only bites when the
cached value is `None` or later than reality.

So this is NOT a live production outage. The real defect is narrower and worth
fixing on its own terms: **the backstop — the documented safety net — cannot
fire, so a missed notification has no recovery path.**

This contradicts the design intent directly. T-0743 added the backstop as the
safety net, and `cron_api.rs`'s own comments say a notify makes the first fire
"on time **instead of waiting for the backstop**" — the backstop it defers to
does not work.

## Blast radius (revised down)

Every in-process writer notifies today — checked, listed above — so there is no
known path by which a schedule created on this process fails to fire. What
remains:

1. **A row appearing from outside the process.** Another replica writing to the
   shared DB does not notify this one. Any replica can claim and fire, and the
   replica that created it did re-arm, so this only bites if that replica dies
   before firing. Narrow, but it is exactly the case a working backstop would
   cover.
2. **The maintainability trap.** The next author who writes a cron row through
   the DAL will reasonably assume the documented 30s backstop catches them. It
   does not, and the failure is silent — nothing fires and nothing logs. This is
   the strongest argument for the fix: a safety net that silently does not catch
   is worse than no safety net.

Priority lowered from P1 to P3 on that basis: latent correctness / robustness,
not a live outage.

## Suggested fix

Keep T-0743's timer-driven intent but stop rebuilding the future: hold a
persistent deadline and create `tokio::time::sleep_until(deadline)` ONCE
outside the loop (or use an `Interval` reset on change), so accumulated
progress survives the 1s tick. Recompute the deadline only when it actually
elapses, on `cron_change`, or after a fire.

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
- [x] P3 - Low (when time permits)

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

- [x] The backstop can elapse while the 1s trigger tick is running: the cron
      sleep is created once and lives across loop iterations; only the cron
      branch and `cron_change` reset it.
- [x] A cron row written directly through the DAL — no notification — fires
      via the backstop. Proven by a regression test that FAILS on the pre-fix
      code with this ticket's exact symptom and passes with the fix, on both
      backends.
- [x] No busy loop: an elapsed `Sleep` is re-armed inside the cron branch
      (an un-reset elapsed sleep stays ready and would win every select).

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

- 2026-08-30 — FIXED, VERIFIED BOTH DIRECTIONS. Exactly the ticket's suggested
  shape: `cron_sleep` created once before the loop (`tokio::pin!` outside),
  reset via `Sleep::reset` only in the cron branch (after fire + re-query) and
  the `cron_change` branch. The 1s tick no longer wipes accumulated progress.
  One subtlety the suggested fix didn't mention: an elapsed `Sleep` stays
  ready forever, so the cron branch MUST re-arm it or it wins every
  subsequent select (busy loop) — commented in code.

  Regression test `test_cron_backstop_fires_unnotified_schedule`
  (integration/scheduler/cron_basic.rs): runner with a 2s backstop and the
  default 1s tick, then a due cron row written through the DAL directly —
  deliberately NOT `register_cron_workflow`, which notifies. The claim path
  advances `next_run_at` before any workflow lookup, so that advance IS the
  proof the cron branch ran. Counterfactual run with the fix stashed: FAILS
  with "backstop never fired ... after 10s" — the ticket's exact symptom.
  With the fix: sqlite green, postgres green (fires right at the 2s backstop).

  Postgres test gotcha worth remembering: the test fixture is schema-isolated
  (UUID schema), so the runner must be built via
  `DefaultRunner::with_database(fixture.get_database(), ...)` — a
  URL-constructed runner looks at `public` and never sees the fixture's row
  (observed as a false FAIL on postgres only).