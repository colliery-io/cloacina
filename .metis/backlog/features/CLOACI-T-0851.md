---
id: reactive-layer-ha-accumulator
level: task
title: "Reactive-layer HA — accumulator/reactor state is per-replica in-memory; no cross-replica coordination"
short_code: "CLOACI-T-0851"
created_at: 2026-07-06T11:39:27.698917+00:00
updated_at: 2026-07-06T11:39:27.698917+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Reactive-layer HA — accumulator/reactor state is per-replica in-memory; no cross-replica coordination

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Make the reactive layer (accumulators + reactors) safe and well-defined under multi-replica cloacina-server deployments. Filed from the 2026-07-06 HA review: everything else is HA-proven (T-0818 / ADR A-0008 — task/cron scheduling is active-active via atomic claiming; the fleet control loop is per-tick leader-elected with validated failover; login state is in Postgres), but the reactive layer is the gap:

- **Accumulator buffers and reactor dirty-flags/caches are per-replica in-memory state.** Every replica that loads a CG package spawns its OWN accumulators + reactor.
- **An event lands on ONE replica** (whichever receives the WS/REST inject), so with N replicas a stream's events can split across N independent buffers — a `state` accumulator's window or a `when_all` criteria set may never assemble on any single replica.
- Reactor snapshots persist to the DB (`persist_reactor_state`) but restore is per-instance, not coordinated; two replicas restoring the same reactor both proceed independently.
- T-0722 moved graph COMPUTE to the agent fleet, but the reactor state machine stays server-side per-instance.

## Design directions (discovery — pick with a human check-in)

1. **Reactor leadership (likely v1)**: per-reactor claim/lease (Postgres advisory lock or leased row, mirroring A-0008's per-tick election) — exactly one replica runs a given reactor+accumulators; others route incoming socket events to the owner (the delivery substrate/outbox already gives an at-least-once inter-replica channel). Failover = lease expiry → another replica restores from the persisted snapshot.
2. **Sticky routing only (stopgap)**: document + enforce that reactive socket traffic must be session-pinned to one replica (LB affinity); accept reactor loss on replica death until restore.
3. **Externalized accumulator state**: buffers in Postgres/streams rather than memory — biggest change, best semantics; probably post-v1.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (correctness gap only under multi-replica postgres deployments; single-replica and embedded modes are unaffected)

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

- [x] A chosen coordination model (leadership / routing / externalized state) recorded as an ADR with maintainer sign-off.
      → **[[CLOACI-A-0012]]**, maintainer sign-off 2026-08-16: reactor
      leadership (per-reactor advisory-lock lease, mirroring [[CLOACI-A-0008]]),
      PLUS accumulator buffers folded into the existing `persist_reactor_state`
      checkpoint. Accumulator loss on failover was explicitly ruled
      UNACCEPTABLE, so durability is in v1 scope — but single-ownership makes it
      cheap (single writer ⇒ no concurrency protocol, accumulator hot path
      untouched).
- [ ] **Per-reactor advisory lock keys are collision-free ACROSS TENANTS.**
      `save_reactor_state` keys by `graph_name` alone and leans on
      schema-per-tenant isolation for separation; the advisory-lock key space
      has NO schema boundary, so two tenants running a same-named graph would
      contend for one lock and silently serialize each other. Resolve before
      writing the lease code — see A-0012 "Open Question For Implementation".
      This is the most likely place to introduce a subtle cross-tenant bug.
- [ ] Accumulator buffers are persisted AND restored on takeover. Restore is
      the half that can be quietly skipped: a snapshot nothing reads back looks
      complete and buys nothing.
- [ ] Under a 2-replica postgres deployment: events injected round-robin across replicas assemble correctly (a `when_all` reactor fires; a `state` window fills) with no split-brain buffers.
- [ ] Replica death while owning a reactor → another replica resumes it from the persisted snapshot (bounded takeover time; no lost checkpointed state), **with a partially-filled accumulator window intact across the failover** — this is the criterion that distinguishes the chosen design from leadership alone.
- [ ] Multi-replica reactive validation added to the k8s-leader e2e lane (extends T-0818's harness).
- [ ] Single-replica + embedded behavior byte-for-byte unchanged.

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

- 2026-08-10 — BACKLOG AUDIT. Verdict: **still true, verbatim; no adjustment
  needed.** The most accurate of the four audited.

  Confirmed: `pg_try_advisory_lock` appears ONLY in
  `crates/cloacina-server/src/autoscaler/leader.rs` (the fleet control loop,
  `FLEET_CONTROL_LOCK_KEY`). There is no per-reactor claim, lease, or owner
  row anywhere in `cloacina/src/computation_graph/` or `cloacina-server/src/`
  — design direction 1 remains unimplemented, as do 2 and 3.

  CORROBORATED FROM THE INSIDE, not just by grep: T-0924 (cross-tenant reactor
  collision) required working directly in `ComputationGraphScheduler`, which
  holds its `reactors` / `graph_to_reactor` maps as in-process `HashMap`s.
  T-0924 re-keyed those maps by tenant, which fixes collisions WITHIN one
  process and touches nothing about coordination BETWEEN processes. So the
  "per-replica in-memory state" claim is not an inference from filing-time
  notes — it is what the code looked like from inside as recently as this
  session's work.

  One thing worth adding rather than changing: the P2 rationale ("single-replica
  and embedded modes are unaffected") is still correct and is load-bearing,
  because the demo/e2e stacks all run single-replica. That is exactly why this
  gap cannot regress into view on its own — nothing we routinely run would
  ever exhibit it. Whoever picks this up should assume ZERO existing coverage
  would have caught a regression here, and budget for the 2-replica harness in
  the acceptance criteria as real work rather than an afterthought.

  No changes made to objective, design directions, priority, or acceptance
  criteria — all still accurate.

- 2026-08-16 — STARTED. [[CLOACI-A-0012]] signed off. First deliverable: the
  tenant-safe lock-key scheme, which A-0012 named as the blocker to resolve
  BEFORE any lease code.

  **DONE:** `crates/cloacina/src/computation_graph/reactor_lock_key.rs` —
  `reactor_lock_key(tenant: Option<&str>, reactor_name: &str) -> i64`, with 7
  unit tests, all passing (`cargo test -p cloacina --lib --features sqlite
  reactor_lock_key` → 7 passed).

  Confirmed the hazard is real, not theoretical: tenants are isolated by
  **schema within one database** (`SET LOCAL search_path TO <tenant>`,
  `database/admin.rs`) and advisory locks are **database-wide**, not
  schema-scoped. Name-only keying would have made two tenants' same-named
  reactors contend for one lock, with exactly one silently never running.

  Two traps found, each defended with a test rather than a comment:
    * **A seeded hasher would be a split-brain bug.** `DefaultHasher` /
      `RandomState` is seeded per PROCESS, so each replica computes a different
      key, every replica wins "the lock", and all of them run the reactor —
      presenting as an intermittent duplicate rather than an error. Used
      hand-rolled FNV-1a with fixed constants; the stability test pins exact
      literals so a change to the encoding fails loudly instead of splitting
      brains during a rolling deploy.
    * **`save_reactor_state` is a misleading precedent.** It keys checkpoints
      by graph name alone and is CORRECT to do so, because the DAL is
      schema-scoped. Locks are not. Documented in the module so the next reader
      does not copy the wrong pattern.
    Also length-delimited the encoding so `("a","bc")` and `("ab","c")` cannot
    collide, and forced the sign bit so reactor keys occupy the negative i64
    half — disjoint from hand-picked small positive keys like
    `FLEET_CONTROL_LOCK_KEY` (8_110_127).

  **DESIGN FINDING THAT CHANGES THE IMPLEMENTATION SHAPE (not in A-0012).**
  From `autoscaler/leader.rs`: advisory locks are **session-scoped**, and the
  fleet loop holds a pooled connection for ONE BRIEF TICK — lock, work, unlock,
  return to pool. Reactor ownership is not tick-shaped; it must persist as long
  as the replica runs the reactor. Copying the fleet pattern naively would hold
  one pooled connection PER OWNED REACTOR for the process lifetime, exhausting
  the pool as reactor count grows.

  Proposed resolution, to validate next: ONE dedicated "ownership session"
  connection per replica holding N advisory locks — a Postgres session can hold
  many. Crash → session ends → ALL that replica's reactor locks release at
  once, which is exactly the failover semantics wanted, at O(1) connections
  instead of O(reactors). Preserves A-0012's "no lease/heartbeat bookkeeping"
  property, since session-scoped locks auto-release on connection loss.

  NEXT: validate one-session-many-locks against a live postgres — specifically
  that killing the session really does release EVERY lock it held, which is the
  assumption the whole failover story rests on. Then the claim/release API,
  then routing, then accumulator persist + restore.

  **VERIFICATION PLAN — use `angreal test e2e k8s-leader`, do not build
  anything new.** I initially wrote "needs a live postgres" as though that were
  an obstacle; the harness already provides the entire interruption apparatus
  (maintainer correction, 2026-08-16). `.angreal/test/e2e/k8s_leader.py`
  (T-0818) already runs a REAL 2-replica k3s deployment and already has:

    * `_psql(kubeconfig, target, sql)` — arbitrary SQL into the cluster's postgres
    * `_lock_holders()` — who currently holds an advisory lock, via `pg_locks`
      joined to `pg_stat_activity`, resolving `client_addr` → owning pod
    * `_wait_lock_holder(want_pod=/not_pod=)` — polls for a holder matching a
      predicate AND asserts we never observe >1 simultaneous holder
    * `_sample_lock(window_s)` — high-frequency sampling returning
      `(max_simultaneous, observed_holders)`
    * **assertion 5 is already the interruption test**: delete the lock-holding
      replica, assert the survivor acquires the lock and the killed replica
      reschedules and rejoins as a follower

  That is exactly the shape T-0851 needs; the acceptance criterion "multi-replica
  reactive validation added to the k8s-leader e2e lane" was always pointing here.
  The reactor assertions become: own N reactors on one replica → kill it → assert
  ALL N locks release and the survivor claims them, with a partially-filled
  accumulator window intact on the other side.

  **DONE toward that:** parameterized the lane's lock helpers by key —
  `lock_query(key)` + `_lock_holders(..., query=)` — so reactor-lock assertions
  reuse the existing "never two simultaneous holders" invariant instead of a
  second, subtly-different copy. `LOCK_QUERY` is now `lock_query(FLEET_LOCK_KEY)`,
  so the existing fleet assertions are unchanged.

  **TRAP FOUND WHILE DOING IT, worth its own note.** `pg_locks.objid` holds only
  the LOW 32 BITS of a 64-bit advisory key. Reactor keys are full-width i64 with
  the sign bit forced, so matching a raw i64 against `objid` matches NOTHING —
  and a lock query that matches nothing PASSES a "no two holders" assertion. The
  failure mode is a green test proving the absence of the thing it was meant to
  observe. Added `advisory_objid(key)` (`key & 0xFFFF_FFFF`) with that reasoning
  recorded at the call site. Any future reactor-lock assertion MUST go through
  it.

  NOT YET VERIFIED: the harness edit has not been syntax-checked or run (tooling
  was unavailable at the time of writing). Do that before trusting it — it is a
  pure-Python change to a lane that takes many minutes to execute, so a syntax
  error would otherwise surface deep into a k3s deploy.