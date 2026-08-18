---
id: 001-reactive-layer-ha-per-reactor
level: adr
title: "Reactive-layer HA — per-reactor leadership with durable accumulator state"
number: 1
short_code: "CLOACI-A-0012"
created_at: 2026-08-16T14:45:32.910934+00:00
updated_at: 2026-08-16T14:47:12.603232+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Reactive-layer HA — per-reactor leadership with durable accumulator state

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Filed from the 2026-07-06 HA review ([[CLOACI-T-0851]]). Everything else in
cloacina-server is HA-proven — task/cron scheduling is active-active via atomic
claiming, the fleet control loop is per-tick leader-elected with validated
failover ([[CLOACI-A-0008]]), login state is in Postgres. The reactive layer is
the remaining gap.

**Verified against the code (2026-08-16), not inferred:**

1. **Reactor and accumulator state is per-replica, in-process.**
   `ComputationGraphScheduler` holds its `reactors` / `graph_to_reactor` maps as
   in-memory `HashMap`s. [[CLOACI-T-0924]] re-keyed those by tenant, which fixes
   collisions WITHIN one process and does nothing about coordination BETWEEN
   processes.
2. **An event lands on exactly one replica** (whichever terminates the WS/REST
   inject), so with N replicas a stream's events split across N independent
   buffers. A `state` accumulator's window or a `when_all` criteria set may
   never assemble on any single replica.
3. **Half the state is already durable; the other half is not.**
   `persist_reactor_state` (`reactor.rs:1180`) snapshots the reactor's
   `InputCache`, `DirtyFlags` and `SeqQueue` through
   `save_reactor_state(graph_name, ..)`, with failure counters and a `Degraded`
   health transition already wired. **Accumulator buffers are persisted
   nowhere** — there is no `save_accumulator*` / `persist_accumulator` anywhere
   in the tree.

Point 3 is the one that shapes this decision, and it is not in the original
ticket. Reactor leadership ALONE would give failover in which the reactor
resumes from its snapshot while every accumulator restarts empty — a
half-filled window silently gone. Maintainer decision (2026-08-16): that loss
is **not acceptable**; accumulator buffers must survive failover.

Single-replica and embedded deployments are unaffected by any of this and must
stay byte-for-byte unchanged.

## Decision **[REQUIRED]**

**Per-reactor leadership, plus accumulator buffers folded into the existing
reactor checkpoint.**

1. **Ownership.** Each reactor instance is claimed by exactly one replica via a
   per-reactor lease, using the `pg_try_advisory_lock` pattern already proven
   in `autoscaler/leader.rs` (`FLEET_CONTROL_LOCK_KEY`) — a distinct lock key
   per reactor rather than one global key.
2. **Routing.** A replica that receives a reactive event for a reactor it does
   not own forwards it to the owner over the existing delivery
   substrate/outbox, which already provides an at-least-once inter-replica
   channel. Non-owners run no accumulators and no reactor state machine.
3. **Durability.** `persist_reactor_state` is extended to include accumulator
   buffers alongside `InputCache`/`DirtyFlags`/`SeqQueue`, written in the same
   checkpoint.
4. **Failover.** Lease expiry → another replica claims the reactor and restores
   BOTH the reactor snapshot and the accumulator buffers before accepting
   forwarded events.
5. **Unchanged paths.** Single-replica and embedded modes keep today's
   behavior; with one replica the lease is always acquired locally and routing
   never engages.

**The load-bearing consequence of ordering (1) before (3):** because leadership
guarantees a SINGLE WRITER per reactor, accumulator durability needs no
concurrency protocol, no locking, and no shared mutable store. It is strictly
"more bytes in a checkpoint only one process is allowed to write." This is why
the maintainer's "accumulators must survive" requirement does NOT force the
externalized-state alternative: that option's cost lives almost entirely in
making buffers safe for CONCURRENT access, which leadership makes unnecessary.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

{Delete if there's only one obvious solution}

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Reactor leadership (CHOSEN)** | Reuses the A-0008 advisory-lock pattern already proven in production; reactor state machine needs no redesign; single-writer makes accumulator durability trivial; routing rides the existing delivery substrate | Adds a lease/routing hop; events for a non-owned reactor pay one forward; lock-key allocation per reactor needs care | Medium | M |
| Sticky routing only | Cheapest — mostly docs plus an ingress/chart affinity change | A deployment constraint, not a fix: reactor state still lost on replica death, and a misconfigured LB silently splits streams with no in-code defense. Fails the maintainer's durability requirement outright | Low to build, **High in production** | S |
| Externalized accumulator state | Best semantics; no reactive state in process memory at all | Touches the accumulator hot path, so it carries real throughput risk on high-rate streams; most of its cost buys concurrent-access safety that leadership renders unnecessary | High | L |

## Rationale **[REQUIRED]**

**Why leadership over sticky routing.** Sticky routing cannot satisfy the
maintainer's requirement that accumulator buffers survive failover — replica
death loses them by construction. It also relocates a correctness property into
LB configuration, where nothing in the codebase can enforce or even detect a
misconfiguration. Silent stream-splitting with no in-code defense is precisely
the failure mode this ADR exists to remove.

**Why leadership over externalized state.** These were framed as competing
options, but they are not symmetric once ordering is considered. Externalized
state is expensive mainly because concurrent access to accumulator buffers has
to be made safe on the hot path. Establishing single-ownership FIRST deletes
that requirement: with one writer per reactor, "durable" collapses to "write
more bytes into the checkpoint that already exists." We get the durability the
maintainer asked for at a fraction of the cost, and the hot path is untouched.

**Why extend `persist_reactor_state` rather than add a parallel mechanism.**
That function already owns checkpointing for this exact reactor, already has
failure counters, a bounded failure streak, and a `Degraded` health transition
attributing failures per branch (`cache_serialize`, `dirty_serialize`,
`seq_serialize`, `save`). A second persistence path would duplicate all of it
and introduce the possibility of the two halves of one reactor's state being
checkpointed at different instants — i.e. a torn snapshot. One checkpoint, one
consistency point.

## Consequences **[REQUIRED]**

### Positive
- Multi-replica reactive deployments become correct rather than
  accidentally-correct-if-the-LB-is-configured-right.
- Accumulator buffers become durable for the first time — this is a gap that
  exists TODAY even on a single replica: a process restart currently loses
  every accumulator window silently. Leadership is the motivation, but
  single-replica restarts benefit immediately.
- One coordination pattern across the codebase (fleet control and reactors both
  use advisory locks), so there is one thing to understand and one to operate.
- The accumulator hot path is unchanged, so no throughput risk on high-rate
  streams.

### Negative
- Events arriving at a non-owner replica pay a forwarding hop; latency under
  multi-replica is no longer uniform.
- Checkpoints grow by the size of the accumulator buffers. A large `state`
  window makes each checkpoint materially bigger, and the write is synchronous
  with respect to the persist cadence. Buffer size may need a bound, or the
  cadence may need tuning — to be measured, not guessed.
- Per-reactor advisory lock keys must be allocated collision-free across
  tenants. `save_reactor_state` keys by `graph_name` alone and relies on
  schema-per-tenant isolation for separation; the lock-key space has no such
  schema boundary, so this needs explicit design. **Flagged as the most likely
  source of a subtle cross-tenant bug in the implementation.**
- Takeover is not instantaneous: there is a lease-expiry window during which a
  reactor processes nothing.

### Neutral
- Single-replica and embedded behavior is unchanged; the lease is always
  acquired locally and routing never engages.
- Nothing in existing CI would catch a regression here — every demo and e2e
  stack runs single-replica. The 2-replica harness is required work, not a
  nice-to-have.

## Review Schedule **[CONDITIONAL: Temporary Decision]**

Permanent as a direction, but two triggers would reopen the durability half:

### Review Triggers
- **Checkpoint size or persist latency becomes a problem on high-rate streams.**
  Folding accumulator buffers into the checkpoint trades write size for
  simplicity. If a `state` window grows large enough that checkpointing becomes
  a bottleneck, the externalized-state option comes back on the table for
  accumulators specifically — leadership stays either way.
- **A reactive workload needs sub-lease-expiry takeover.** The lease window is
  acceptable for the current model; a latency-sensitive consumer would need
  hand-off rather than expiry-based failover.

### Open Question For Implementation — RESOLVED 2026-08-16
Per-reactor advisory lock-key allocation across tenants (see Consequences →
Negative). Resolved in `computation_graph::reactor_lock_key`: a fixed-constant
FNV-1a over a length-delimited `(tenant, reactor)` encoding, sign bit forced so
reactor keys occupy the negative i64 half and cannot collide with hand-picked
small positive keys like `FLEET_CONTROL_LOCK_KEY`. The hazard was real —
tenants are isolated by SCHEMA within one database, while advisory locks are
database-wide.

Note for anyone extending this: a seeded hasher (`DefaultHasher`/`RandomState`)
would have been a split-brain bug, since it is seeded PER PROCESS — every
replica would compute a different key, every replica would win "the lock", and
all of them would run the reactor. The stability test pins exact key literals
for that reason.

---

## AMENDMENT 1 (2026-08-16) — long-held ownership DOES need a liveness check

**What this amends:** the Decision's reliance on the A-0008 property that
session-scoped advisory locks need no lease or heartbeat bookkeeping, because a
dead replica's locks auto-release. That reasoning is sound for FAILOVER and it
is NOT sufficient for ownership held across many ticks. This was found while
implementing [[CLOACI-T-0851]]; it does not change the chosen design, but the
original text would lead an implementer to omit something load-bearing.

**The gap.** The fleet control loop re-acquires its lock EVERY TICK, so a
dropped connection simply means it stops being leader — self-correcting. Reactor
ownership is acquired once and then assumed. That admits a failure the fleet
loop cannot have:

> The ownership connection drops (network blip, PgBouncer recycle, database
> restart) while the replica keeps running. Postgres releases every lock that
> session held. Another replica legitimately claims the reactor. **The original
> replica never notices and keeps running it.** Two replicas, one reactor, no
> error raised anywhere.

That is the exact split-brain this ADR exists to prevent, re-entering through
the side door — and presenting as intermittent duplicate work rather than as an
error.

**Amended decision (maintainer, 2026-08-16): self-check and halt.** The
ownership session periodically re-asserts that Postgres still reports every lock
it believes it holds (`SESSION_HELD_LOCKS_SQL`, scoped by
`pid = pg_backend_pid()` so another replica's lock cannot read as our own). On
loss, the affected reactors are stopped locally BEFORE any re-claim is
attempted.

This is loss DETECTION, not lease renewal. There is still no TTL, no clock
assumption, and no bookkeeping row; Postgres remains the single source of truth
and we only ask whether what we believe is still true. The "no lease/heartbeat
bookkeeping" property survives in substance — what does not survive is the
inference that *nothing* need be checked.

A three-state result is required, not a boolean: "we lost locks" and "the check
could not run" are different situations, and conflating them yields either
needless stops of healthy reactors or confident operation of unowned ones. An
indeterminate check must be treated as UNKNOWN, never as healthy.

**Considered and deferred: fencing tokens.** A monotonic token per claim,
carried on every checkpoint write and rejected when stale, is strictly safer —
a zombie replica could not corrupt state even inside the detection window. It
needs a schema change and touches the checkpoint write path, so it is not in
v1. Revisit if the detection window proves too wide in practice.

## AMENDMENT 2 (2026-08-16) — ownership is ONE session holding many locks

**What this amends:** an unstated assumption that reactor locks would be taken
the way `autoscaler/leader.rs` takes its lock. They cannot be, and the reason is
resource exhaustion rather than correctness.

`with_fleet_leadership` holds one pooled connection for the duration of ONE
TICK — lock, work, unlock, return to pool. Advisory locks are session-scoped, so
a lock survives only while its connection is held. Reactor ownership must
persist for as long as the replica runs the reactor, so the same shape would pin
one pooled connection PER OWNED REACTOR for the process lifetime, exhausting the
pool as reactor count grows.

**Amended decision:** ONE dedicated ownership session per replica, carrying ALL
of that replica's reactor locks — a Postgres session may hold many. Connection
cost is O(1) in reactor count instead of O(reactors), and replica death still
ends the session and releases every reactor lock at once, which is exactly the
failover behaviour the Decision relies on.

Operator-visible consequence (maintainer chose "take one and document it" over
raising the default): each replica permanently reserves one connection from the
pool. Pool sizing must account for it.