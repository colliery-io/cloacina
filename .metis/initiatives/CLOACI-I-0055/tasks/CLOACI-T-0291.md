---
id: stale-claim-sweep-background
level: task
title: "Stale claim sweep — background service for expired claim recovery"
short_code: "CLOACI-T-0291"
created_at: 2026-03-29T12:33:50.668863+00:00
updated_at: 2026-03-29T12:33:50.668863+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Stale claim sweep — background service for expired claim recovery

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

Implement a background service that periodically scans for tasks with stale heartbeats (crashed runners), releases their claims, and re-queues them for execution. Must handle scheduler restart gracefully — avoid falsely declaring tasks as stale during the scheduler's own startup window.

## Acceptance Criteria

- [ ] `StaleClaimSweeper` background service runs on a configurable interval (default 30s)
- [ ] Calls `find_stale_claims(threshold)` to discover tasks with expired heartbeats
- [ ] For each stale task: releases the claim, resets status to `ready` for re-execution
- [ ] **Startup grace period**: the sweeper records when it became ready (`ready_at` timestamp). During its first `threshold` duration after startup, it does NOT sweep. This prevents false positives when the scheduler restarts and the previous runner's tasks look stale because no sweep was running.
- [ ] Configurable stale threshold (default 60s — must be > heartbeat interval)
- [ ] Logs: "Sweep found N stale claims", "Released claim on task X (runner Y, last heartbeat Z ago)"
- [ ] Integrates into DefaultRunner as an optional background service (like cron recovery)
- [ ] Shutdown-aware: stops on shutdown signal

## Implementation Notes

### Startup grace period problem
In a distributed system:
1. Scheduler crashes
2. Runners keep heartbeating to the DB
3. Scheduler restarts (takes N seconds)
4. Sweep service starts, sees tasks heartbeated M seconds ago
5. If M > threshold but M < N (startup time), those tasks are NOT stale — the runners are fine, the scheduler just wasn't watching

**Solution**: The sweeper ignores all claims until `now - ready_at > threshold`. This means the sweeper needs to be "warmed up" for one full threshold duration before it starts evicting. During warmup, it only logs — no action.

### Files to create/modify
- `crates/cloacina/src/task_scheduler/stale_claim_sweeper.rs` — new file
- `crates/cloacina/src/runner/default_runner/services.rs` — start sweeper as background service
- `crates/cloacina/src/runner/default_runner/config.rs` — add sweeper config fields

### Depends on
- T-0289 (Claim DAL — `find_stale_claims`)

## Status Updates

*To be added during implementation*
