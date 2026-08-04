---
id: cg-runtime-defects-cel-tenant-stub
level: task
title: "CG runtime defects — CEL tenant stub, sequential-queue restore, lock-held supervisor backoff"
short_code: "CLOACI-T-0915"
created_at: 2026-08-02T16:33:18.144884+00:00
updated_at: 2026-08-03T01:32:47.536198+00:00
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

# CG runtime defects — CEL tenant stub, sequential-queue restore, lock-held supervisor backoff

## Objective

Fix three localized bug-shaped defects in the computation-graph runtime found by the 2026-08-02 architecture deep dive (single-node verdict otherwise production-grade).

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Findings

1. CEL `tenant` VARIABLE IS A STUB BOUND TO "". Reactor→workflow subscription predicates advertise a `tenant` variable (docs + API docs), but evaluation binds it to the empty string (crates/cloacina/src/cron_trigger_scheduler.rs:1403-1405). Combined with fail-closed evaluation and watermark-always-advances-on-skip, ANY predicate referencing `tenant` silently never fires and the firings are permanently skipped. Fix: bind the real tenant id (it is on the firing row); add a predicate test per tenant; consider a load-time lint for predicates referencing unbound variables.
2. SEQUENTIAL-QUEUE PERSISTENCE IS WRITE-ONLY. The executor persists `_seq_queue` (and dirty flags) before draining so a crash mid-drain "does not lose items", but Reactor::run restores ONLY the cache — `_dirty_data` and `_seq_queue` are discarded on start (crates/cloacina/src/computation_graph/reactor.rs:622). input_strategy=sequential's no-loss/ordering promise and WhenAll's restored dirty state hold only within one process lifetime. Fix: restore both on reactor start (the rows already exist), or stop persisting them and document the ephemerality.
3. SUPERVISOR SLEEPS BACKOFF HOLDING THE REACTORS WRITE LOCK. Restart backoff (up to 60s, 5-failure circuit breaker) sleeps while holding the scheduler's reactors write lock (crates/cloacina/src/computation_graph/scheduler.rs:1165, 1222, 1405), so during a restart storm all loads, listings, and /v1/health/* graph reads block behind it — the health surface goes dark exactly when operators need it. Fix: drop the lock before sleeping; re-acquire for the restart attempt.

Related but tracked elsewhere: cross-replica reactive state is T-0851; the crashed-CG-503s-/ready-replica-wide behavior is noted in T-0916.

## Acceptance Criteria

## Acceptance Criteria

- [x] Predicate referencing `tenant` matches the firing tenant (cel_predicate_tenant_binds_real_tenant_id: match, non-match, and stub-value-no-longer-matches)
- [x] Reactor restart restores seq queue + dirty flags (test_reactor_restart_restores_sequential_queue: restored items drain first in order; test_reactor_restart_restores_dirty_flags: WhenAll fires from restored flag, stale de-scoped flag not overlaid)
- [x] Health reads responsive during restart backoff (test_health_reads_responsive_during_restart_backoff: <500ms mid-backoff vs ~1.5s blocked before)

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (cg-runtime report; DEEPDIVE.md risk register). Findings verified against main @ 5216e632.
- 2026-08-02: DONE — merged to main in PR #230 (squash). (1) tenant threaded from the subscription row through evaluate_predicate into the CEL context. (2) Both persisted states now restore: seq queue seeds the executor VecDeque; dirty flags overlay the expected-source seed, only for sources still expected (stale flag cannot wedge all_set). (3) check_and_restart_failed restructured to plan-under-lock -> classify/record/sleep unlocked -> re-acquire per restart with re-validation (entry unloaded or live handle installed while unlocked -> skip); restart bodies verbatim. Intended nuance: crashes during another component's backoff wait for the next supervision tick. NOT done here (deliberate): the load-time lint for unbound predicate variables (finding 1 'consider') and the CEL-error watermark/dead-letter behavior — that is DEEPDIVE register #5, still unticketed as its own item.
