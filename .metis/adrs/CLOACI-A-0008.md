---
id: 001-fleet-control-loop-leadership-in
level: adr
title: "Fleet control-loop leadership — in-process advisory-lock leader, not a singleton controller"
number: 1
short_code: "CLOACI-A-0008"
created_at: 2026-06-28T03:14:05.225219+00:00
updated_at: 2026-06-28T04:31:30.230406+00:00
decision_date: 
decision_maker: 
parent: CLOACI-I-0127
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Fleet control-loop leadership — in-process advisory-lock leader, not a singleton controller

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

The fleet control loop (autoscaler + reconcile, CLOACI-T-0811) mutates per-tenant `desired_count` and actuates (spawns/scales agents). A horizontally-scaled (multi-replica) server must run this loop on **exactly one replica** — otherwise replicas race on `desired_count` and double-provision (and on the K8s actuator, flap the per-tenant Secret key). The server is a **deployment-layer** service (embedded is the *library* layer only and runs no fleet loop), so running multiple replicas is a normal HA/scale expectation, not an edge case.

## Decision **[REQUIRED]**

Serialize the fleet control loop with an **in-process Postgres advisory-lock leader**: every replica runs the loop but gates each tick on `pg_try_advisory_lock(<fleet key>)`, so exactly one replica leads per tick (auto-failover on connection loss). The **API (stateless)** and the **scheduler/dispatch (per-task DB claiming)** scale freely across all replicas — only the fleet loop is leader-gated. The fleet loop cannot use the scheduler's per-task claiming because it is a global per-tenant control decision, not a stream of claimable work units.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

{Delete if there's only one obvious solution}

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **(A) In-process advisory-lock leader** (chosen) | One deployment; API + scheduler scale freely; cheap; free failover; fits embedded-first single binary | Every replica carries mostly-idle loop code; multi-replica safety depends on the lock (must be validated, T-0818) | Low | Done (T-0811) |
| (B) Separate singleton fleet-controller Deployment (replicas=1), API scaled independently | Clean separation; no lock | Second deployment + run mode; more moving parts; less natural | Medium | Medium |
| (C) No serialization, pin server to replicas=1 | Simplest code | Caps the server at 1 replica — no HA; a silent double-provision footgun if scaled | Low | Low |

## Rationale **[REQUIRED]**

(A) keeps a **single deployment** that scales freely (API + scheduler) with a thin gate around the one irreducibly-singleton action; the advisory lock is cheap and failover is free (session-scoped — frees automatically if the leader's connection drops; no lease/heartbeat bookkeeping). (B) is cleaner separation-of-concerns but adds a second deployment + run mode. (C) caps the server at one replica (no HA). Since embedded does not constrain the server (it is library-only), there is no reason to special-case the loop into a separate process; A is the simplest HA-capable shape.

## Consequences **[REQUIRED]**

### Positive
- A single server deployment scales horizontally (API + scheduler) with no extra components.
- HA-capable: kill the leader, another replica acquires the lock and the loop resumes.
- Dormant/free at 1 replica (the common case): the lock is a no-op the one replica always wins.

### Negative
- Every replica links the (mostly-idle) control-loop code.
- Multi-replica correctness depends on the lock + per-task claiming, which is only ever run at 1 replica today — must be validated (CLOACI-T-0818: 2-replica soak proving disjoint claiming, single-writer provisioning, and leader failover). The Helm chart should make `replicas: N` a documented, supported story.

### Neutral
- At 1 replica the advisory lock holder reads `null`/trivial in instrumentation (observed in the T-0815 soak) — expected, not a defect.

## Validation

CLOACI-T-0818 built `angreal test e2e k8s-leader` (a `replicaCount=2` k3s deployment of the chart) and ran it green — 4/5; the 5th blocked by environment, not design:

- **Single leader** — sampling `pg_locks` for the fleet advisory key (8110127) at ~10 Hz over 60s: **never more than one simultaneous holder**. Leadership *rotates per tick* between the two replicas (the lock is taken and released each control tick), which satisfies "exactly one leader per tick."
- **Single-writer provisioning** — `provision N=3` scaled the tenant Deployment to exactly 3 (not 2×N) despite two server replicas.
- **Failover** — deleting the lock-holder pod handed leadership to the survivor (different pid/addr) within a bounded wait; provisioning resumed under the new leader; the killed replica rescheduled and rejoined 2/2 with holders still ≤1.
- **Disjoint scheduler claiming** — not exercised end-to-end (a helm-only server ships no compiler, so uploaded packages never build); the property is enforced in cloacina-core by the `task_outbox` claim (`DELETE … FOR UPDATE SKIP LOCKED` + `claimed_by` CAS) and both replicas run the per-tenant scheduler unconditionally. An opt-in `--claiming` e2e path is scaffolded for when a matching-ABI in-cluster compiler is available.

**Refinement surfaced:** leadership rotates per tick (no sticky leader), and the autoscaler's cooldown / last-action state is in-memory per replica — so the cooldown is **not** globally coordinated (a replica leading a later tick has no memory of a peer's recent scale action). Manual provisioning is unaffected (single-writer holds). To strictly honor the autoscaler cooldown at multi-replica, persist its last-action timestamp in the DB (or hold the lock across ticks for sticky leadership). A candidate autoscaler refinement, not a correctness defect. **Implemented (CLOACI-I-0127):** migration 038 adds `last_autoscaled_at` to `agent_desired_counts`, stamped via SQL `now()` by `set_desired_autoscaled` and read back to gate a wall-clock `should_act_at` — the cooldown now holds across replicas regardless of which replica leads a given tick.