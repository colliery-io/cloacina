---
id: cel-predicate-errors-silently
level: task
title: "CEL predicate errors silently black-hole subscriptions — hold the watermark, dead-letter, and lint unbound variables"
short_code: "CLOACI-T-0922"
created_at: 2026-08-05T22:33:06.974265+00:00
updated_at: 2026-08-06T02:48:32.220715+00:00
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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Err holds the watermark and returns, so the firing is head-of-line again next tick (ordering preserved, other subscriptions unaffected)
- [x] Bounded at MAX_CONSECUTIVE_PREDICATE_ERRORS = 5, then dead-lettered: subscription marked degraded, error recorded, watermark force-advanced so one poison firing cannot wedge it forever
- [x] Ok(false) unchanged, with a regression test pinning the distinction
- [x] cloacina_reactor_predicate_errors_total + _dead_letters_total, warn!/error! with subscription id, firing id, truncated expression
- [x] Subscribe-time lint at the DAL chokepoint rejects unbound variables (comprehension iteration vars correctly treated as bound)

## Status Updates

- 2026-08-05: Filed from the architecture deep dive (DEEPDIVE.md consolidated risk register #5; cg-runtime report S1.6). Split out of CLOACI-T-0915 (completed) which fixed the tenant stub but explicitly left the watermark/dead-letter behavior and the lint for a separate change.
- 2026-08-06: DONE — merged to main in PR #239 (squash). Confirmed the Err arm was byte-for-byte identical to the Ok(false) arm. COUNTER LIVES IN THE DB, not memory (migration 048 adds predicate_error_count/_firing_id/last_predicate_error{,_at}/predicate_degraded to reactor_trigger_subscriptions, ADD COLUMN only, both backends) so it survives restart — and because the watermark is held, "consecutive errors" == "attempts on this firing", making the subscription row the natural home. A clean evaluation clears the count + degraded flag but deliberately NEVER clears last_predicate_error* — recovery must not erase the evidence. OBSERVABILITY IS HONESTLY SCOPED: there is no HTTP API for reactor subscriptions, so an operator sees the two counters, the log lines, and the DB row via list_reactor_subscriptions — whose Python binding now also exposes the five new fields plus predicate_expression (it exposed neither before). LINT: cel_interpreter's own Program::references() conflates comprehension iteration variables with free ones and would have FALSE-REJECTED valid expressions like payload.items.exists(i, i.price > 100), so the lint walks the cel-parser AST treating iter_var/iter_var2/accu_var as bound; new runtime dep cel-parser. ADJACENT PRE-EXISTING BUG FIXED: Program::compile PANICS (unreachable! in antlr4rust) on a malformed expression rather than returning Err, and T-0602's subscribe path called it bare — a typo'd predicate could unwind the subscriber task; new compile_predicate() catches it, used by both subscribe and the scheduler cache. Docs corrected: filter-reactor-firings-with-cel.md documented the OLD "error treated as false, watermark advances" behavior and was actively wrong. RESIDUALS (open): predicate_degraded self-heals so a permanent "data was dropped" alarm lives only in last_predicate_error* + the monotonic counter; the bound is not configurable; the counter is a two-statement RMW — exact for today's single poller, approximate (never unsafe) if a second ever races.
