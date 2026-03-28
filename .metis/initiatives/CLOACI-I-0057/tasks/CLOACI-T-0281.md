---
id: graceful-shutdown-signal-handling
level: task
title: "Graceful shutdown — signal handling, drain in-flight pipelines"
short_code: "CLOACI-T-0281"
created_at: 2026-03-28T15:30:09.535944+00:00
updated_at: 2026-03-28T15:30:09.535944+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# Graceful shutdown — signal handling, drain in-flight pipelines

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Implement clean shutdown for the daemon process. On SIGINT/SIGTERM: stop accepting new work, stop schedulers, wait for in-flight pipelines to complete (with timeout), stop the directory watcher, close the database.

## Acceptance Criteria

- [ ] `tokio::signal::ctrl_c()` (SIGINT) triggers shutdown
- [ ] SIGTERM also triggers shutdown (Unix signal handler)
- [ ] Shutdown broadcasts to all components via `tokio::sync::watch` channel
- [ ] Cron scheduler stops polling on shutdown signal
- [ ] Trigger scheduler stops polling on shutdown signal
- [ ] Directory watcher stops on shutdown signal
- [ ] Reconciler stops its loop on shutdown signal
- [ ] `DefaultRunner::shutdown()` called to drain in-flight pipelines
- [ ] Shutdown timeout (configurable, default 30s) — force-exit if drain doesn't complete
- [ ] Logs shutdown progress: "Shutting down...", "Waiting for N pipelines...", "Shutdown complete"

## Implementation Notes

### Files to modify
- `crates/cloacinactl/src/commands/daemon.rs` — shutdown orchestration

### Key design points
- Use `tokio::sync::watch::channel(false)` as shutdown signal, broadcast `true` on signal
- All components already accept `watch::Receiver<bool>` for shutdown (reconciler, cron scheduler, trigger scheduler)
- `DefaultRunner::shutdown()` already drains in-flight work
- Second Ctrl+C during shutdown should force-exit immediately

### Depends on
- T-0278 (daemon subcommand — the loop to add shutdown to)
- T-0280 (scheduler wiring — schedulers must be started to be stopped)

## Status Updates

*To be added during implementation*
