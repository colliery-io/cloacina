---
id: pipeline-claiming-horizontal
level: initiative
title: "Pipeline Claiming — Horizontal Scaling"
short_code: "CLOACI-I-0055"
created_at: 2026-03-26T17:16:34.096280+00:00
updated_at: 2026-03-29T13:45:55.337744+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


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
- Task-level claiming: runners atomically claim individual tasks before executing
- `claimed_by` and `heartbeat_at` columns on `task_executions` table
- SQLite and Postgres DAL support for claim/heartbeat/release/expiry operations
- Prevent duplicate task execution across multiple runner instances
- Graceful handling of runner crashes via heartbeat expiry
- Background stale claim sweep service
- Multi-instance integration testing

**Non-Goals:**
- Server/daemon deployment infrastructure (I-0049)
- Package distribution or trigger system (I-0050)
- Continuous scheduling (I-0053)
- Auto-scaling or orchestration (future work)

## Detailed Design

### Task-Level Claiming
Claiming happens at the task level — the task is the atomic executable primitive. This means:
- Multiple runners can work on the same pipeline concurrently (different tasks)
- A runner claims a specific task, heartbeats while executing, releases on completion
- If a runner crashes, the claim expires and another runner picks up the task
- No separate pipeline-level claims needed — the task scheduler decides which tasks are ready

### Schema Changes
Add two columns to `task_executions` table:
- `claimed_by` — nullable UUID identifying the runner instance that claimed this task
- `heartbeat_at` — nullable timestamp, updated periodically while the task is executing

### Claim Operations
- `claim_task(task_id, runner_id)` — atomic compare-and-swap: only succeeds if `claimed_by` is NULL
- `heartbeat_task(task_id, runner_id)` — update `heartbeat_at` (only if `claimed_by` matches)
- `release_claim(task_id)` — clear `claimed_by` and `heartbeat_at` on completion/failure
- `find_stale_claims(threshold)` — tasks where `heartbeat_at` is older than threshold (crashed runners)

### DAL Support
Same claim API for SQLite (single-node dev) and Postgres (multi-node prod):
- SQLite: simple UPDATE with WHERE clause (single-writer, no contention)
- Postgres: SELECT FOR UPDATE or compare-and-swap for true atomic claiming

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

1. **Schema migration** — Add `claimed_by` and `heartbeat_at` to `task_executions`, update Diesel schema
2. **Claim DAL** — `claim_task()`, `heartbeat_task()`, `release_claim()`, `find_stale_claims()` for SQLite + Postgres
3. **Runner integration** — Task executor claims before executing, heartbeats during, releases on completion/failure
4. **Stale claim sweep** — Background service that finds expired claims and re-queues tasks
5. **Integration tests** — Concurrent claimants, crash recovery, double-claim prevention
