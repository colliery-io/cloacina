---
id: 001-fleet-control-loop-leadership-in
level: adr
title: "Fleet control-loop leadership — in-process advisory-lock leader, not a singleton controller"
number: 1
short_code: "CLOACI-A-0008"
created_at: 2026-06-28T03:14:05.225219+00:00
updated_at: 2026-06-28T03:14:05.225219+00:00
decision_date: 
decision_maker: 
parent: CLOACI-I-0127
archived: false

tags:
  - "#adr"
  - "#phase/draft"


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

## Review Schedule **[CONDITIONAL: Temporary Decision]**

{Delete if decision is permanent}

### Review Triggers
- {Condition that would trigger review 1}
- {Condition that would trigger review 2}

### Scheduled Review
- **Next Review Date**: {Date}
- **Review Criteria**: {What to evaluate}
- **Sunset Date**: {When this decision expires if not renewed}