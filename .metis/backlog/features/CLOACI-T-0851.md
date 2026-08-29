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
      → PARTIALLY PROVEN 2026-08-29 (assertion 6c): an inject at the NON-owner
      is 307-redirected to the owner, the owner accepts it, and the outbox
      stays at ZERO rows — single-buffer assembly with the hot path intact.
      Round-robin volume assembly not yet asserted.
- [ ] Replica death while owning a reactor → another replica resumes it from the persisted snapshot (bounded takeover time; no lost checkpointed state), **with a partially-filled accumulator window intact across the failover** — this is the criterion that distinguishes the chosen design from leadership alone.
      → PARTIALLY PROVEN 2026-08-29 (assertion 6d): owner killed → a new owner
      claimed within the window and REPUBLISHED its address. Window-content
      survival across the takeover not yet asserted (the restore machinery
      exists per A-0012 CORRECTION 1; the e2e does not yet fill a window,
      fail over, and read it back).
- [x] Multi-replica reactive validation added to the k8s-leader e2e lane (extends T-0818's harness).
      → **DONE 2026-08-29: assertion 6 GREEN on a real 2-replica k3s cluster**
      (`5/6 green; failed: []`, exit 0 — assertion 4 blocked by design without
      `--claiming`). Single owner (6a), published address (6b), non-owner 307 →
      owner accepts → zero outbox rows (6c), kill → new owner claims +
      republishes (6d).
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

  RESOLVED: the harness edit is now syntax-checked and its output verified.
  `advisory_lock_parts(8110127)` → `(0, 8110127, 1)`, so the tightened fleet
  query matches the identical row set — no behavior change to T-0818's existing
  assertions. Reactor keys split to distinct `(classid, objid)` pairs.

  While verifying, tightened it further: the query now matches the FULL key
  (`classid` + `objid` + `objsubid`), not `objid` alone. Partial matching is
  wrong in both directions and both are quiet — under-match returns no rows (and
  a lock query returning no rows PASSES a "never two holders" assertion, going
  green while proving the absence of what it should observe); over-match
  conflates two keys sharing their low 32 bits, which matters here because every
  reactor key shares its high bits by construction (forced sign bit), leaving
  the low word as the only discriminator.

- 2026-08-16 (cont.) — OWNERSHIP SESSION.
  `crates/cloacina/src/computation_graph/reactor_ownership.rs`: `ReactorId`
  (tenant + name, mirroring the scheduler's own map key so a claim and the
  reactor it guards cannot drift), `OwnershipState` (what this replica BELIEVES
  it owns), `OwnershipCheck`, and `SESSION_HELD_LOCKS_SQL`. 7 tests; full crate
  check green on `postgres,macros --all-targets`.

  **MAINTAINER DECISION 2026-08-16 — A-0012 NEEDS AMENDING.** A-0012 states
  session-scoped locks need "no lease/heartbeat bookkeeping." True for failover,
  NOT sufficient for long-held ownership. New failure mode, absent from the ADR:

  > The ownership connection drops (network blip, PgBouncer recycle, DB
  > restart) while the replica keeps running. Postgres releases every lock the
  > session held. Another replica legitimately claims the reactor. The original
  > replica NEVER NOTICES and keeps running it. Two replicas, one reactor, no
  > error raised anywhere.

  The fleet loop is immune only because it re-acquires every tick — a dropped
  connection just means it stops leading. Ownership that is ASSUMED rather than
  re-established must be re-verified.

  Decision: **self-check + halt.** The session periodically re-asserts its locks
  via `SESSION_HELD_LOCKS_SQL`; on loss the affected reactors are stopped
  locally BEFORE any re-claim. This is loss DETECTION, not lease renewal — no
  TTL, no clock assumption, no bookkeeping row, Postgres still the sole source
  of truth. (Fencing tokens were considered and deferred: strictly safer, but
  they need a schema change and touch the checkpoint write path.)

  Design details worth keeping:
    * `OwnershipCheck` is a 3-state enum, not a bool. "We lost locks" and "the
      check could not run" are different situations; conflating them yields
      either needless stops of healthy reactors or confident operation of
      unowned ones. `Indeterminate` must be treated as UNKNOWN, never healthy.
    * `SESSION_HELD_LOCKS_SQL` is scoped by `pid = pg_backend_pid()`, with a
      test asserting that predicate is present. Without it, ANOTHER replica's
      lock reads as our own and every liveness check passes while split-brained.
    * `diff_against_held_keys` is a pure function so the logic deciding whether
      reactors get stopped is testable with no database, including the
      everything-lost case (dropped session must report ALL reactors lost, not
      silently none).

  Pool: one dedicated connection per replica carrying ALL its reactor locks —
  O(1) in reactor count, documented as an operator-visible reservation
  (maintainer chose "take one and document it" over raising the default).

- 2026-08-16 (cont.) — `OwnershipSession` IMPLEMENTED (postgres-gated).
  `connect` / `claim` / `release` / `verify_owned`, holding the dedicated
  connection and driving `pg_try_advisory_lock` / `pg_advisory_unlock` /
  `SESSION_HELD_LOCKS_SQL` through deadpool's `interact`, mirroring
  `with_fleet_leadership`. Both feature builds green
  (`postgres,macros` and `sqlite,macros`, `--all-targets`); 14 unit tests pass.

  Three deliberate behaviours, each chosen because the alternative fails quietly:
    * A failed `claim` is NOT recorded as owned. `Ok(false)` means another
      replica owns it — an ordinary outcome, not an error — and recording it
      would make us believe we hold a lock we never got.
    * `release` forgets the reactor locally EVEN IF the unlock returns false.
      False means the lock was not held on this session, i.e. we had already
      lost it; continuing to believe we own it is strictly the more dangerous
      of the two options.
    * `verify_owned` forgets lost reactors immediately, before returning. The
      caller still has to stop them, but from that moment nothing in this
      process believes it owns them, whatever the caller does next.

  NEXT: periodic verify task + halt-on-loss wiring into
  `ComputationGraphScheduler`, then event routing to the owner, then accumulator
  persist + restore.

  GATE RESULT — `angreal test e2e k8s-leader`, real 2-replica k3s, **exit 1**,
  `3/5 green; blocked: ['4','5']`.

  **My lock-helper change is NOT the cause, and the evidence is specific.** The
  worry was the exact trap documented above: if the tightened query matched
  nothing, assertion 2 ("single fleet-lock holder") would still PASS, because
  zero holders satisfies "at most one" — a vacuous green. The log rules that
  out:

      samples with a holder: 6; max simultaneous holders: 1;
      holders observed: {…-nsp62: 3, …-dr8fs: 3}

  Six real catches across both pods, never simultaneous. The full-key query
  (`classid=0 AND objid=8110127 AND objsubid=1`) returns rows against a live
  Postgres, so `objsubid = 1` is confirmed correct for the
  `pg_try_advisory_lock(bigint)` form — which also validates the same assumption
  baked into `SESSION_HELD_LOCKS_SQL`.

  * Assertion 4 blocked BY DESIGN — it needs `--claiming`, which was not passed.
  * Assertion 5 blocked: `never caught the lock holder pre-kill`. **Pre-existing
    flakiness, not a regression.** The fleet lock is taken and released within a
    single control tick, so it is only briefly held; the whole sampling window
    caught it just 6 times. Assertion 5 must identify the holding pod at one
    specific instant before killing it, and it loses that race often.

  **THIS MATTERS FOR T-0851 IN A GOOD WAY.** Reactor ownership locks are held
  CONTINUOUSLY, not per-tick. The reactor failover assertion therefore does not
  inherit assertion 5's race at all: the holder is always there to be caught, so
  "kill the owner, watch the survivor claim it" should be reliable where the
  fleet equivalent is flaky. The reactor assertions should NOT copy assertion
  5's sampling approach — they can simply read the holder directly.

  Worth filing separately: assertion 5 is a CORE assertion that fails the lane
  (exit 1) yet cannot run reliably, so `k8s-leader` is red for reasons unrelated
  to any change under test. That is a trust problem for a gate — it trains
  people to ignore the result.

- 2026-08-17 — FLAKE FIXED AND VERIFIED. `angreal test e2e k8s-leader` now
  **exits 0, 4/5 green, failed: []**. Assertion 5 passes on a real 2-replica
  cluster:

      current lock holder: pod=…-m2nrl addr=10.42.0.3 pid=160 — deleting it
      failover: lock re-acquired by pod=…-wpb8c addr=10.42.0.6 pid=64

  (Assertion 4 still blocked purely because `--claiming` was not passed.)

  ROOT CAUSE was latency, not logic. Polling spawned TWO subprocesses per sample
  (`kubectl get pods` + `kubectl exec … psql`), hundreds of ms each, while the
  fleet lock is taken and released WITHIN one control tick — so the sample rate
  was latency-bound and the whole window caught it 6 times. Fix: move the poll
  inside Postgres. One exec runs a plpgsql loop sampling `pg_locks` every 10ms
  and returns the instant a holder appears — same predicate, ~1000x the density.
  Applied to BOTH the pre-kill catch and the post-kill survivor wait; the latter
  has the identical race and HARD-FAILS rather than blocking.

  TWO REAL BUGS IN MY OWN FIX, both found by running it, both the same family —
  a quiet fallback turning a missing observation into a WRONG one:

    1. **`::text` on inet appends the netmask.** Verified against live Postgres:
       `'10.42.0.3'::inet::text` → `10.42.0.3/32`, while `host(...)` → `10.42.0.3`.
       The `/32` form matches no key in `_server_pod_ips`. This is why the
       ORIGINAL code worked and my rewrite broke it — plain display omits the
       mask, the cast does not. Now uses `host()`.
    2. **`ip_to_pod.get(addr, addr)` returned the ADDRESS as a pod name** when
       resolution failed, which went straight to `kubectl delete pod
       10.42.0.3/32` and crashed the lane with a confusing
       `CalledProcessError`. This fallback PREDATES my change and would convert
       any future resolution failure into the same confusing crash. Now prints
       what failed to resolve and reports "no holder" instead of deleting
       something that does not exist.

  Also caught before the first cluster run, by testing the SQL against
  `cloacina-postgres:15432`: `client_addr` is NULL for unix-socket connections
  and `NULL || '|' || pid` is NULL, so a holder would have existed but rendered
  as nothing and the catcher would have reported "no holder found". Fixed with
  `coalesce` before it ever reached a cluster.

  BEARING ON T-0851: reactor ownership locks are held CONTINUOUSLY, so the
  reactor failover assertions do not inherit this race at all — the holder is
  always there to be caught. They should read the holder directly rather than
  copying assertion 5's sampling. The server-side catcher is still the right
  tool for the post-kill wait, since "wait until someone OTHER than the killed
  replica holds it" is inherently a wait.

- 2026-08-17 — OWNERSHIP IS NOW ENFORCED IN THE SCHEDULER. PR #255 (draft),
  11 commits. 90 `computation_graph` tests pass; both feature builds green.

  **`ReactorOwnership` trait + `Option<Arc<dyn …>>` on the scheduler.** `None`
  is the embedded / sqlite / single-replica path and runs NONE of this code —
  which is how A-0012's "byte-for-byte unchanged" requirement is actually
  guaranteed rather than merely intended. A trait (not the concrete
  `OwnershipSession`) so the scheduler is not postgres-gated and the loss paths
  are testable with a fake; the real failure modes — connection dropped, lock
  stolen, verification unavailable — cannot be produced on demand against a
  live database.

  **`ownership_watchdog_tick`** — verify → watchdog verdict → halt. A single
  tick, not a loop, so cadence stays with the caller and the test does not
  depend on timing.

  **`halt_unowned_reactors`** deliberately bypasses `unload_reactor`'s
  subscriber guard. That guard is right for an operator unload and wrong here:
  having lost the lock, another replica may already be running this reactor, so
  refusing to stop because a subscriber remains leaves two copies
  double-processing. Shares ONE `teardown_running` with the unload path — a
  second copy would drift by forgetting a deregistration, leaving a stopped
  reactor still advertised in the endpoint registry.

  **Claim at load.** Placed with the other "resolve what can fail before we
  spawn" work: losing a claim after the reactor and accumulators are live would
  mean tearing down a running reactor, and a partial teardown is how endpoints
  get orphaned. NOT winning is a normal outcome and the load still SUCCEEDS —
  erroring would report a correctly-functioning multi-replica deployment as a
  failed load on every replica but the owner. A claim ERROR does fail the load:
  if we cannot reach Postgres to claim, we equally cannot know nobody else
  holds it.

  **`foreign_reactors` set**, found by a test rather than by design. The first
  version returned `Ok(())` on claim loss and `load_graph` then tried to bind a
  graph to a reactor that was never started — `reactor 'rx' is not loaded`.
  Loaded-but-not-owned is a THIRD state, distinct from both loaded and absent,
  and it needs to be explicit. This set is also where routing will look:
  "where should this event go" begins with "is this reactor foreign to me".

  `ReactorId` now converts to/from `TenantKey` instead of paralleling it —
  `TenantKey`'s docs warn a deployment must never hold "two spellings of the
  same scope", and an ownership claim keyed differently from the scheduler's
  map would take a lock for one reactor while guarding another.

  **REMAINING — this is NOT nearly done. Honest estimate: multiple sessions.**
    1. **Server wiring.** Nothing constructs `PostgresOwnership` or calls
       `set_ownership`, and nothing drives `ownership_watchdog_tick` on a timer.
       Until that lands the feature is dormant everywhere — which is why it is
       safe to have merged this far, and also why none of it is proven in situ.
    2. **Event routing to the owner.** The largest remaining piece. Events
       landing on a non-owner currently go nowhere: today that replica does not
       run the reactor at all. Needs the delivery-substrate/outbox integration
       A-0012 assumes. Start from `foreign_reactors`.
    3. ~~**Accumulator persist + restore** — the maintainer's hard requirement,
       and untouched so far.~~ **CORRECTED 2026-08-18: accumulator state is
       ALREADY persisted and restored.** See [[CLOACI-A-0012]] CORRECTION 1.
       `CheckpointHandle::save`/`load` plus per-kind implementations: polling
       (~678/714), batch (~810/882) and — the kind A-0012 worried about —
       state, via `load_state_buffer` (~1079) / `save_state_buffer` (~1147) on
       every event. My "persisted nowhere" claim came from grepping
       `save_accumulator*`, a name this codebase never used, and reporting the
       empty result as an absence proof.

       So do NOT build item 3 as written; folding buffers into
       `persist_reactor_state` would duplicate a working mechanism. What
       remains is verification and gap-closing:
         a. Demonstrate a partially-filled window surviving a REAL takeover.
            Both halves exist; nobody has shown them working across an
            ownership change, and per-kind cadence differs (state saves per
            event, batch on flush), so the amount at risk differs by kind.
         b. Decide whether the reactor snapshot and accumulator checkpoints
            need mutual consistency — they are separate writes, so a takeover
            can restore a reactor snapshot from T1 beside buffers from T2.
         c. Confirm checkpoint keys are tenant-safe. They key by
            `(graph_name, accumulator_name)` and lean on schema-per-tenant DAL
            scoping — the same argument that makes `save_reactor_state` safe,
            and the same argument that did NOT hold for advisory locks.
    4. **k8s-leader reactor assertions** — own N reactors on one replica, kill
       it, assert all N release, the survivor claims them, AND a partially
       filled accumulator window survives. Read the holder directly (reactor
       locks are held continuously); use the server-side catcher only for the
       post-kill wait.

- 2026-08-24 — ROUTING RESHAPED; EDGE AFFINITY DESIGN SETTLED, NOT YET BUILT.
  See [[CLOACI-A-0012]] AMENDMENT 3. Maintainer caught that Decision item 2 puts
  the hot path on Postgres: with N replicas, ~(N−1)/N of injects land on a
  non-owner, so "forward via the outbox" makes a durable INSERT + NOTIFY + drain
  the COMMON case (ms vs µs for a channel send) and scales the database with
  event rate. That contradicts the ADR's own "accumulator hot path is unchanged,
  no throughput risk" claim — reintroducing the risk one layer earlier, at
  ingest.

  Outbox is therefore DEMOTED to the correctness backstop for the brief window
  where ownership is changing and the edge's view is stale. Built and tested but
  deliberately NOT wired into the inject routes — switching them before edge
  affinity exists would have made Postgres the default path for every non-local
  inject. `send_to_accumulator` remains the route path; no behaviour change.

  **Two findings that shape the remaining work:**

  a. **There is no replica-address concept anywhere.** `agent_registry` covers
     execution agents, not server replicas; replicas sit behind a Service and
     neither know nor publish a reachable address. So publication needs a new
     config input, a table, and a migration — this is not a small wiring job.
  b. **A WebSocket cannot be redirected mid-stream** (clients do not follow
     redirects post-upgrade), **but the handshake is ordinary HTTP**, so a 307
     at upgrade time works for clients that follow redirects. That splits the
     paths usefully: REST redirects per request, WS redirects ONCE and then
     stays hot for the connection's life — after which a pinned WS connection
     costs nothing per event. This makes edge affinity substantially more
     attractive than it first appeared, since the steady state is zero
     forwarding AND zero redirects.

  **Maintainer decision (2026-08-24): advertise a headless-service DNS name**
  (stable per-pod DNS) rather than pod IP or an operator-configured URL. Chosen
  for stability across IP churn; costs a chart change, since the current
  Deployment has no headless Service or per-pod identity (needs `hostname` +
  `subdomain`, or a StatefulSet).

  **Implementation plan, in order — steps 1–4 DONE as of 2026-08-24 (see the
  2026-08-24 cont. entry below); only step 5 (e2e) remains:**
    1. Chart: headless Service + per-pod DNS identity; inject the replica's own
       advertised name via env (downward API for pod name, composed with
       subdomain/namespace).
    2. Migration + DAL: `reactor_owner_addresses(tenant_id, reactor_name,
       address, claimed_at)`. **This table is a ROUTING HINT ONLY — never a
       second source of truth for ownership.** The advisory lock remains
       authoritative; a stale row must only cause a wasted redirect (the target
       redirects again, or the outbox backstop catches it), never a wrong
       ownership decision. Prefer ADD COLUMN / CREATE INDEX shapes on sqlite
       per repo convention.
    3. Publish on claim / delete on release + on watchdog halt, inside
       `PostgresOwnership` so address lifetime exactly matches lock lifetime.
    4. Edge: REST inject → 307 to the owner; WS upgrade → 307 at handshake.
       Fall back to the outbox when no address is published (mid-takeover).
    5. e2e: assert an inject to a NON-owner reaches the owner, and that the
       steady-state path does no outbox write (the whole point of the
       amendment — otherwise this silently regresses to the slow path).

- 2026-08-24 (cont.) — EDGE ROUTING BUILT (steps 1–4 of the plan). PR #255.
  94 computation_graph tests + 6 DAL tests green; server compiles.

  **Step 2–3 recap:** `reactor_owner_addresses` migration (both backends; the
  uniqueness is an expression index over `COALESCE(tenant_id,'')` because in
  SQL NULL <> NULL — a plain UNIQUE would admit many untenanted rows for one
  reactor; proven live against postgres). DAL with `publish` /
  `remove_if_ours` / `lookup`; `remove_if_ours` matches on the publisher's own
  address so the race (A claims → A dies → B claims+publishes → A's late
  release finally runs) cannot tear down B's row — pinned by test. Publication
  wired into `PostgresOwnership`: publish only AFTER a won claim, retract
  BEFORE unlock on release and on watchdog-detected loss; publication failure
  is logged, never returned, because refusing ownership over a HINT would
  invert the authority order. Server enables it from
  `CLOACINA_ADVERTISED_ADDRESS` (chart-injected under `reactorAffinity`).

  **Step 4, and the design gap it surfaced:** injects name an ACCUMULATOR but
  addresses are published per REACTOR, and only the graph declaration links
  the two — available exactly once, at load time. So losing a claim now also
  records `foreign_accumulators: (tenant, acc) → reactor key` in the
  scheduler, exposed as `foreign_reactor_for_accumulator`. Without it a
  non-owner cannot compute where to redirect and everything would silently
  fall back to the outbox.

  **REST inject resolution ladder** (health_graphs.rs), hottest first:
    1. local channel send (unchanged, zero new cost)
    2. known-foreign + address published → **307** (preserves method+body;
       not cacheable — ownership moves, and a cached redirect would keep
       steering at an ex-owner)
    3. known-foreign, no address (mid-takeover) → durable outbox, reported as
       delivered:0. Scoped to KNOWN-foreign only, so a typo'd name still 404s
       instead of enqueueing garbage forever.
    4. otherwise → not_found, byte-for-byte single-replica behaviour.

  **WS:** redirect at the HANDSHAKE (upgrade is plain HTTP; post-upgrade
  redirects are impossible) — the client pins to the owner ONCE and every
  subsequent frame is an in-process send. If no address exists at handshake
  (mid-takeover) the connection is accepted in fallback mode: `known_foreign`
  is decided once at handshake and passed in, and such frames route through
  `inject_event` (local try, then outbox). Ownership lost MID-connection →
  the 4404 close fires and the client's reconnect handshake gets the
  redirect, which is the correct recovery.

  **Hot-path discipline note:** twice during this step the easy fix was a
  `.clone()` on the per-event path (REST: clone before local send; WS: clone
  every frame for a fallback that almost never runs). Both were caught and
  restructured — REST re-encodes from the still-in-scope `req.event` only in
  the cold arm; WS decides foreignness once per CONNECTION, not per frame.
  The entire point of Amendment 3 is that the common case pays nothing; a
  quiet allocation per event would have eroded exactly that.

  REMAINING: step 5 only — the multi-replica e2e (k8s-leader reactor
  assertions): non-owner inject reaches the owner via redirect; steady state
  does NO outbox write; kill the owner → survivor claims, republishes, and a
  partially-filled accumulator window survives.

- 2026-08-25 — ASSERTION 6 WRITTEN; FIVE RUNS; CODE UNBEATEN BUT UNPROVEN.
  `k8s_leader.py` gained assertion 6 (best-effort like 4 — preconditions
  BLOCK, wrong behaviour FAILs): single reactor-lock holder → published
  address names the holder → inject at the NON-owner returns 307 naming the
  owner → owner accepts the followed redirect → `delivery_outbox` has ZERO
  `reactor_event` rows (the hot-path regression check) → kill owner →
  survivor claims AND republishes. Notables:
    * `reactor_lock_key` ported to Python, pinned at import to the SAME
      literals the Rust stability test pins — a divergent port would watch a
      lock nobody holds, and a lock query matching nothing PASSES a
      single-holder assertion.
    * Redirects are followed from an in-cluster curl probe pod: headless DNS
      only resolves inside the cluster, and a host-side DNS failure would be
      indistinguishable from a wrong redirect.
    * `_leader_values` now sets `reactorAffinity.enabled=true`.

  RUN LEDGER (honesty over optimism):
    * Runs 1–3: infrastructure, zero assertion signal (disk-full buildkit I/O
      error; docker daemon down; daemon crash mid-build — root cause per
      maintainer: parallel agent sessions fighting over docker).
    * Run 4 (fresh images): **exit 0, 4/6 green, failed: []** — the FIRST
      in-situ validation of the new server code: ownership session installed,
      reactorAffinity chart live, both replicas Ready, fleet failover intact.
      6 BLOCKED: no compiler image (lane probes `cloacina-demo-fleet-compiler`
      / `docker-compiler`; only `cloacina-demo-compiler` existed → tagged).
    * Run 5 (--skip-build): compiler deployed, upload 201, **BLOCKED: package
      never reached build_status=success in 6m**. Cause identified: the demo
      compiler image is dated 2026-07-17 — five weeks stale, predating fidius
      0.5.7 and the T-0897 interface-version bumps (5→6→7), so its output
      cannot satisfy today's server. NOT a code failure.

  NEXT ACTION: rebuild the compiler image from current source
  (`docker build -t cloacina-demo-fleet-compiler:latest
  -f docker/Dockerfile.compiler .`, ~2GB), then
  `angreal test e2e k8s-leader --skip-build` again. Assertion 6 has still
  never executed past upload; everything from the reactor claim onward is
  unproven in situ.