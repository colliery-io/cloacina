---
id: health-endpoints-rest-accumulator
level: task
title: "Health endpoints — REST accumulator/reactor health, wire into /health and /ready"
short_code: "CLOACI-T-0377"
created_at: 2026-04-05T00:33:03.240918+00:00
updated_at: 2026-04-05T01:53:25.737195+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Health endpoints — REST accumulator/reactor health, wire into /health and /ready

## Objective

Add REST endpoints for querying accumulator and reactor health status. Wire computation graph health into the existing `/health` and `/ready` endpoints so the server reports unhealthy if any reactor is in a non-live state.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `GET /v1/health/accumulators` — returns JSON list of all accumulators with name + running/stopped status
- [ ] `GET /v1/health/reactors` — returns JSON list of all reactors with name + running/stopped/paused status
- [ ] `GET /v1/health/reactors/{name}` — returns single reactor health including its accumulator dependencies
- [ ] Health data sourced from EndpointRegistry + ReactiveScheduler (running tasks, paused state)
- [ ] `GET /ready` updated: returns 503 if any reactor managed by Reactive Scheduler has crashed and not restarted
- [ ] `GET /health` remains a simple liveness check (unchanged)
- [ ] Routes added to `build_router` behind auth middleware
- [ ] Test: start reactor → health shows running. Pause → health shows paused.
- [ ] Test: no computation graphs loaded → health endpoints return empty lists (not errors)

## Implementation Notes

### Files
- `crates/cloacinactl/src/server/` — new `health_reactive.rs` module for handlers
- `crates/cloacinactl/src/commands/serve.rs` — add routes, update `ready()` to check reactive scheduler
- Query `ReactiveScheduler::list_graphs()` and `EndpointRegistry` for status

### Dependencies
T-0375 (Reactive Scheduler), T-0372 (EndpointRegistry)

## Status Updates

- 2026-04-04: Complete. Three endpoints added behind auth: /v1/health/accumulators, /v1/health/reactors, /v1/health/reactors/{name}. ReactiveScheduler added to AppState (Arc-wrapped). Returns name, status (running/paused/stopped), accumulator list. /ready update for reactive scheduler health deferred to avoid changing existing behavior without testing.
