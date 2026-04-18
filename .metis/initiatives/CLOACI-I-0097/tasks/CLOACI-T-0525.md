---
id: t7-cloacinactl-compiler-noun-start
level: task
title: "T7: cloacinactl compiler noun — start/stop/status/health"
short_code: "CLOACI-T-0525"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-18T15:03:31.090573+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0524]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T7: cloacinactl compiler noun — start/stop/status/health

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Add a `compiler` noun to `cloacinactl` with the same verb set as `server` (from I-0098). `start` execs `cloacina-compiler`; `stop` uses a PID file; `status`/`health` probe the compiler's own HTTP endpoint.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacinactl compiler start [--bind] [--database-url] [--poll-interval-ms] [--heartbeat-interval-s] [--stale-threshold-s] [--sweep-interval-s]` — execs `cloacina-compiler` with flags, writes `$home/compiler.pid`.
- [ ] `cloacinactl compiler stop [--force]` — SIGTERM via `$home/compiler.pid`. Mirror behavior from server stop; exit 0 + diagnostic message when PID file missing.
- [ ] `cloacinactl compiler status` — HTTP `GET /v1/status` on `compiler.local_addr` (default `127.0.0.1:9000`); human output with queue depth + last build + last heartbeat. `-o json` returns the raw object.
- [ ] `cloacinactl compiler health` — HTTP `GET /health`; terse `up`/`down`, exit 0/2.
- [ ] Top-level `cloacinactl status` composite grows a third row for compiler status (daemon + server + compiler side by side).
- [ ] Help output on `cloacinactl compiler --help` matches the spec command tree.
- [ ] Added to the e2e harness from T-0518 so CI exercises compiler start/stop/status/health.

## Implementation Notes

### Structure

Add under `crates/cloacinactl/src/nouns/compiler/`:
```
mod.rs    — CompilerCmd + verb dispatch
start.rs  — exec cloacina-server pattern, lifted verbatim
stop.rs   — PID-file SIGTERM, same as server/stop.rs
status.rs — HTTP probe against compiler.local_addr
health.rs — HTTP probe
```

### Config key

New `[compiler]` section in `~/.cloacina/config.toml`. CLI reads `compiler.local_addr` for the status/health probe URL; flag `--bind` overrides.

### Composite status

`nouns::top_level_status` extends to call `compiler::status::run()` after `server::status::run()` and prints both.

### Compiler's /v1/status endpoint

Compiler's HTTP side (library added in T2) needs to grow the `/v1/status` handler. Returns:
```json
{
  "pending": 3,
  "building": 1,
  "last_success_at": "2026-04-17T12:34:56Z",
  "last_failure_at": null,
  "heartbeat_at": "2026-04-17T12:35:06Z"
}
```

## Status Updates

*To be added during implementation*
