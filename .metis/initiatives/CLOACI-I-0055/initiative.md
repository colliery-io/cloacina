---
id: pipeline-claiming-horizontal
level: initiative
title: "Pipeline Claiming — Horizontal Scaling"
short_code: "CLOACI-I-0055"
created_at: 2026-03-26T17:16:34.096280+00:00
updated_at: 2026-03-26T17:16:34.096280+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: pipeline-claiming-horizontal
---

# Pipeline Claiming — Horizontal Scaling Initiative

## Context

Extracted from I-0050 (originally "Scheduling Features"). Pipeline claiming enables horizontal scaling — multiple runner instances can claim and execute pipelines without conflicts. This was previously implemented as part of I-0034 on the archive branch.

Currently, a single runner instance executes all pipelines. To scale, runners need to atomically claim pipelines before executing, with claim expiry handling runner crashes gracefully.

## Goals & Non-Goals

**Goals:**
- Claim-based execution model: runners atomically claim pipelines before executing
- SQLite and Postgres DAL support for claim records with expiry
- Prevent duplicate execution across multiple runner instances
- Graceful handling of runner crashes via claim expiry
- Multi-instance integration testing

**Non-Goals:**
- Server/daemon deployment infrastructure (I-0049)
- Package distribution or trigger system (I-0050)
- Continuous scheduling (I-0053)
- Auto-scaling or orchestration (future work)

## Detailed Design

### Claim Model
- Runners atomically claim pipelines before executing (compare-and-swap or SELECT FOR UPDATE)
- Claims have a TTL — if a runner crashes, the claim expires and another runner can pick it up
- Heartbeat mechanism to extend claims during long-running pipelines
- DAL abstraction: same claim API for SQLite (single-node dev) and Postgres (multi-node prod)

### Testing
- Unit tests for claim/release/expiry logic
- Integration tests with multiple concurrent claimants
- Failure scenarios: runner crash mid-execution, claim expiry, double-claim attempts

## Prior Art

Reference implementation on `archive/main-pre-reset`:
- Pipeline claiming: commit `ee32916`

## Alternatives Considered

- **Simple lock-based execution**: Rejected. Locks don't handle runner crashes — a crashed runner holds the lock forever. Claim-based model with expiry is more resilient.
- **External coordination (etcd/ZooKeeper)**: Rejected. Adds operational complexity. Database-backed claims are sufficient for the expected scale.

## Implementation Plan

1. **Claim DAL** — Claim table schema, SQLite and Postgres implementations
2. **Runner integration** — Claim acquisition before pipeline execution, heartbeat extension
3. **Expiry and recovery** — Background sweep for expired claims, re-queuing
4. **Multi-instance testing** — Concurrent runner tests with shared database
