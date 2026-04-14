---
id: add-daemon-health-check-endpoint
level: task
title: "Add daemon health check endpoint"
short_code: "CLOACI-T-0476"
created_at: 2026-04-11T13:45:01.709844+00:00
updated_at: 2026-04-13T18:47:03.855239+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add daemon health observability — Unix socket + structured log pulse

## Objective

Make the daemon's internal health observable via two channels:

1. **Unix domain socket** — `cloacinactl status` connects and gets a JSON health snapshot. Operator-friendly, no HTTP server, no port allocation.
2. **Structured log pulse** — periodic `info!` line with machine-parseable health fields. Integrates with log-based monitoring (Datadog, Loki, journald grep) with zero extra infrastructure.

## Review Finding References

OPS-001 (from architecture review `review/10-recommendations.md` REC-004)

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P3 - Low (when time permits)

## Design

### Unix socket

- Daemon creates `$CLOACINA_HOME/daemon.sock` on startup, removes on shutdown
- Simple protocol: client connects, daemon writes JSON response, closes connection (no request needed — single-purpose socket)
- `cloacinactl status` command connects and pretty-prints the response
- If socket doesn't exist → "daemon not running"
- `--no-socket` flag to disable if unwanted

### Health response shape

```json
{
  "status": "healthy | degraded | unhealthy",
  "pid": 12345,
  "uptime_seconds": 3600,
  "database": {
    "connected": true,
    "backend": "sqlite"
  },
  "scheduler": {
    "running": true,
    "consecutive_errors": 0,
    "last_success_at": "2026-04-13T12:00:00Z"
  },
  "reconciler": {
    "packages_loaded": 5,
    "last_run_at": "2026-04-13T12:00:00Z"
  },
  "active_pipelines": 3
}
```

**Status derivation:**
- `healthy` — DB connected, scheduler consecutive_errors == 0
- `degraded` — DB connected but scheduler has errors (circuit breaker tripped)
- `unhealthy` — DB unreachable or scheduler not running

### Structured log pulse

- Emitted every 60s (configurable via `--health-interval`)
- Uses tracing structured fields for machine parsing:
  ```
  info!(
      target: "cloacina::health",
      status = "healthy",
      uptime_s = 3600,
      db_connected = true,
      scheduler_errors = 0,
      active_pipelines = 3,
      packages_loaded = 5,
      "health pulse"
  );
  ```
- Appears in journald, log files, or any tracing subscriber

### Shared health state

- `Arc<DaemonHealth>` struct with atomic/mutex fields, updated by scheduler loop and reconciler
- Scheduler loop already tracks `consecutive_errors` — expose it
- Reconciler updates `packages_loaded` and `last_run_at` after each scan

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DaemonHealth` struct collects scheduler, reconciler, and DB state
- [ ] Unix socket at `$CLOACINA_HOME/daemon.sock` serves JSON health on connection
- [ ] Socket cleaned up on graceful shutdown (and stale socket detected on startup)
- [ ] `cloacinactl status` command reads socket and pretty-prints health
- [ ] Periodic structured health log line emitted at configurable interval
- [ ] Status derived from component states (healthy/degraded/unhealthy)

## Key Files

- `crates/cloacinactl/src/commands/daemon.rs` — socket listener, health pulse task
- `crates/cloacinactl/src/commands/status.rs` — new `cloacinactl status` command
- `crates/cloacina/src/execution_planner/scheduler_loop.rs` — expose consecutive_errors to DaemonHealth

## Status Updates

*To be added during implementation*
