---
id: reliability-shutdown-channels
level: initiative
title: "Reliability — Shutdown Channels, Atomic Completions, and Error Handling"
short_code: "CLOACI-I-0086"
created_at: 2026-04-08T10:46:48.310014+00:00
updated_at: 2026-04-08T23:30:00.723897+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: S
initiative_id: reliability-shutdown-channels
---

# Reliability — Shutdown Channels, Atomic Completions, and Error Handling Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 2: Reliability*

## Context

The review identified several reliability gaps where good patterns exist in one subsystem but are missing from an adjacent one. `SchedulerLoop::run()` is the only background loop without a shutdown channel. Task completion spans two non-atomic DB operations. Error conversions lose infrastructure failure context. The scheduler loop has no circuit breaker for sustained database outages.

## Goals & Non-Goals

**Goals:**
- Add shutdown channel to SchedulerLoop (COR-03, OPS-04)
- Fix server graceful shutdown to drain the runner (OPS-08)
- Make task completion atomic (COR-02)
- Fix lossy error conversion across crate boundary (COR-04, LEG-05, EVO-04)
- Add circuit breaker to scheduler loop (OPS-05)

**Non-Goals:**
- Scheduler architecture redesign
- New error framework

## Detailed Design

### REC-05: Shutdown Channel for SchedulerLoop (COR-03, OPS-04, OPS-08)
**Effort**: 3-4 hours

Add `watch::Receiver<bool>` to `SchedulerLoop`, integrate via `tokio::select!` in the run loop (following `StaleClaimSweeper` pattern). In `serve.rs`, call `runner.shutdown()` with timeout in graceful shutdown sequence.

### REC-06: Atomic Task Completion (COR-02)
**Effort**: 2-3 hours

Combine `save_task_context` and `mark_task_completed` into a single DB transaction in `complete_task_transaction` (`executor/thread_task_executor.rs:498-511`). Use the existing `dispatch_backend!` transaction pattern.

### REC-13: Fix Lossy Error Conversion (COR-04, LEG-05, EVO-04)
**Effort**: 2-3 hours

Add `Database(String)` and `ConnectionPool(String)` variants to `cloacina_workflow::ContextError`. Update `From<ContextError> for TaskError` to use them instead of mapping to `KeyNotFound`.

### REC-12: Circuit Breaker for Scheduler Loop (OPS-05)
**Effort**: 2-3 hours

Add consecutive error counter with exponential backoff (1s base, 30s max). Follow the `ReactiveScheduler::check_and_restart_failed()` pattern.

## Implementation Plan

Sequential: REC-05 first (enables clean shutdown testing), then REC-06 and REC-13 in parallel, REC-12 last. Target: 1-2 weeks.
