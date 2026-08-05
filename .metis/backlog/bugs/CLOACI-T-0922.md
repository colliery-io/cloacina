---
id: cel-predicate-errors-silently
level: task
title: "CEL predicate errors silently black-hole subscriptions — hold the watermark, dead-letter, and lint unbound variables"
short_code: "CLOACI-T-0922"
created_at: 2026-08-05T22:33:06.974265+00:00
updated_at: 2026-08-05T22:33:06.974265+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# CEL predicate errors silently black-hole subscriptions — hold the watermark, dead-letter, and lint unbound variables

## Objective

Stop a broken reactor-subscription predicate from silently and permanently discarding firings. Deep-dive risk register #5 (HIGH), split out of CLOACI-T-0915 which fixed the adjacent `tenant`-stub bug but deliberately left this behavior alone.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: any tenant using CEL predicates on reactor->workflow subscriptions.
- **Expected vs Actual**: expected — a predicate that errors is visible and recoverable. Actual — the firing is skipped AND the watermark advances past it, so the firing is unrecoverable and the failure is invisible: no metric, no dead-letter, no health signal. A predicate broken by payload-shape drift black-holes the subscription forever while everything reports healthy.

## Findings

1. In `process_reactor_subscription` (crates/cloacina/src/cron_trigger_scheduler.rs, the per-firing loop ~:1160-1200), `Ok(false)` and `Err(_)` take the same path: log, `advance_watermark`, `continue`. For `Ok(false)` that is correct (the filter did its job). For `Err(_)` it converts a transient/structural fault into permanent silent data loss.
2. The fail-closed doctrine ("a broken filter should not fire workflows") is right about NOT dispatching — but it does not require advancing the watermark. The two decisions were conflated.
3. No observability: no `cloacina_reactor_predicate_errors_total` counter, no dead-letter row, nothing in the graph/reactor health surface.
4. Related gap (finding 1 of T-0915, deliberately deferred there): predicates referencing variables that are not bound (the `tenant` stub incident) compile fine and evaluate false/error forever. A load-time or subscribe-time lint over the known variable set (`payload`, `reactor`, `tenant`) would have caught that class at the door.

## Proposed shape (adjust on implementation)

- Distinguish the two outcomes: `Ok(false)` -> skip + advance watermark (unchanged). `Err(_)` -> do NOT advance; retry the firing with a bounded attempt count.
- After N consecutive errors on the same firing: write a dead-letter record (or mark the subscription degraded) and THEN advance so one poison firing cannot wedge the subscription forever. Surface the degraded state on the reactor/subscription health view.
- Add `cloacina_reactor_predicate_errors_total{subscription,reactor}` and log at warn with the expression text truncated.
- Add the subscribe-time lint for unbound variable references; reject with a clear error naming the allowed variables.

## Acceptance Criteria

- [ ] A predicate that errors does not advance the watermark on the first failure; the firing is retried
- [ ] A persistently-failing firing is dead-lettered/degraded (bounded), never silently dropped, and the state is visible on a health surface
- [ ] `Ok(false)` behavior unchanged (skip + advance) with a test pinning the distinction
- [ ] Predicate errors are counted in a metric and logged
- [ ] Subscribe-time lint rejects predicates referencing unbound variables (test: `tenant` ok, `tennant` rejected)

## Status Updates

- 2026-08-05: Filed from the architecture deep dive (DEEPDIVE.md consolidated risk register #5; cg-runtime report S1.6). Split out of CLOACI-T-0915 (completed) which fixed the tenant stub but explicitly left the watermark/dead-letter behavior and the lint for a separate change.
