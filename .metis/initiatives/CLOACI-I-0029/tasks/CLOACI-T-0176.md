---
id: wire-defaultrunner-into-serve
level: task
title: "Wire DefaultRunner into serve lifecycle with mode-based service selection"
short_code: "CLOACI-T-0176"
created_at: 2026-03-16T01:35:08.548219+00:00
updated_at: 2026-03-16T12:34:42.777119+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# Wire DefaultRunner into serve lifecycle with mode-based service selection

## Objective

Connect the Cloacina `DefaultRunner` (the core task orchestration engine) into the `serve` command lifecycle so that the `--mode` flag controls which background services are started alongside (or instead of) the HTTP server. This enables deployment topologies where API, worker, and scheduler roles can run in a single process or be split across separate processes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Database connection pool created from `ServerConfig.database` settings at serve startup
- [ ] DAL (Data Access Layer) instantiated from the connection pool
- [ ] `DefaultRunner` constructed using config values from the TOML config
- [ ] Mode `all`: starts API server + scheduler + cron + recovery + executor + dispatcher
- [ ] Mode `api`: starts API server only (no background services)
- [ ] Mode `worker`: starts executor + dispatcher only (no API server, no scheduler)
- [ ] Mode `scheduler`: starts scheduler + cron + recovery only (no API server, no executor)
- [ ] Background services (`DefaultRunner`) started before the axum server begins accepting requests
- [ ] Shutdown order: stop accepting HTTP connections -> stop DefaultRunner services -> close DB pool
- [ ] Clean shutdown completes without errors or panics in all four modes
- [ ] Logs indicate which services were started based on mode selection

## Implementation Notes

The `DefaultRunner` from `cloacina::runner` already manages scheduler, executor, dispatcher, cron, and recovery services. The mode flag determines which services are enabled in the runner config before calling `start()`. For `api` mode, skip `DefaultRunner` entirely and just run axum. For `worker` and `scheduler` modes, skip axum and only run the relevant `DefaultRunner` services. For `all` mode, run both. Use `tokio::select!` or `JoinSet` to manage concurrent axum + runner lifecycles. The shutdown signal should propagate to both the HTTP server and the runner via a shared `tokio::sync::watch` or `CancellationToken`. Depends on CLOACI-T-0174 (config) and CLOACI-T-0175 (axum server).

## Status Updates

### 2026-03-16 — Completed
- `build_runner_config()` converts ServerConfig → DefaultRunnerConfig based on mode
- Mode-based service selection: `all` = API + runner, `api` = API only, `worker`/`scheduler` = runner only
- DefaultRunner created with `with_config()` when DB URL is configured and mode needs services
- Missing DB URL logs warn and continues (graceful degradation for api-only mode)
- Scheduler/cron/trigger enabled only for `all` and `scheduler` modes
- Continuous scheduling enabled when both config flag and scheduling mode are active
- Non-API modes wait on `shutdown_signal()` directly (no axum)
- Ordered shutdown: stop HTTP → stop runner → log clean exit
- Verified: api mode starts with no DB, scheduler mode warns about missing DB
- All 19 cloacinactl tests pass
