---
id: daemon-mode-continuous-scheduler
level: task
title: "Daemon mode — continuous scheduler and mixed-load scenarios"
short_code: "CLOACI-T-0259"
created_at: 2026-03-26T02:36:46.936607+00:00
updated_at: 2026-03-26T03:18:24.854698+00:00
parent: CLOACI-I-0046
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0046
---

# Daemon mode — continuous scheduler and mixed-load scenarios

## Parent Initiative

[[CLOACI-I-0046]]

## Objective

Add server-mode scenarios to `scheduler_bench.py`. Uses stdlib urllib to hit a running `cloacinactl serve` instance — uploads packages, triggers executions via POST /executions, polls GET /executions/{id} until complete, measures real HTTP round-trip latency.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `execute` scenario: POST /executions with workflow name, poll GET /executions/{id} until terminal, measure full round-trip
- [ ] `execute-concurrent` scenario: N concurrent submissions (threading), measure throughput under load
- [ ] `cron-via-api` scenario: POST /schedules to create cron, wait for execution, measure schedule → complete
- [ ] HTTP client helper with configurable base URL and API key
- [ ] Graceful error when server is not running (clear message, not crash)
- [ ] `--mode server --base-url` and `--api-key` CLI flags

## Implementation Notes

### Server API endpoints
- `POST /executions` — `{"workflow_name": "...", "context": {}}` → `{"execution_id": "...", "status": "accepted"}`
- `GET /executions/{id}` — returns `{"status": "Completed|Running|Failed|...", ...}`
- `POST /workflows/{name}/schedules` — create cron schedule
- `GET /health` — check server is up before running
- Package upload via multipart or the workflow upload endpoint

### Prerequisites
- Server running: `angreal services up` + `cloacinactl serve`
- Package must be uploaded first

### Dependencies
- T-0258 (script structure, stats, reporting already in place)

## Status Updates

- 2026-03-26: Added server mode to scheduler_bench.py — HTTP client (stdlib urllib), execute + execute-concurrent scenarios with threading, health check, graceful error on unreachable server. Syntax check passes.
