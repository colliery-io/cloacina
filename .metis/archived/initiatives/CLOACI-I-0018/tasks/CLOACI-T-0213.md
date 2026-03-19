---
id: server-schedule-management-api
level: task
title: "Server Schedule Management API — cron CRUD endpoints"
short_code: "CLOACI-T-0213"
created_at: 2026-03-18T02:21:27.040088+00:00
updated_at: 2026-03-18T03:09:41.488943+00:00
parent: CLOACI-I-0018
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0018
---

# Server Schedule Management API — cron CRUD endpoints

## Parent Initiative

[[CLOACI-I-0018]] Cloacina Server - Deployable Workflow Infrastructure

## Objective

Expose cron schedule management through the server's REST API. `DefaultRunner` already has full cron CRUD (`register_cron_workflow`, `list_cron_schedules`, `get_cron_schedule`, `delete_cron_schedule`, `get_cron_execution_stats`). This task wires those methods to HTTP endpoints so workflows uploaded via the API can be scheduled without touching code.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /workflows/{name}/schedules` — create a cron schedule for a registered workflow (body: `{"cron": "...", "timezone": "UTC"}`)
- [ ] `GET /workflows/{name}/schedules` — list all cron schedules for a workflow
- [ ] `GET /workflows/schedules/{id}` — get a single schedule by ID
- [ ] `DELETE /workflows/schedules/{id}` — delete a cron schedule
- [ ] `GET /workflows/schedules/{id}/history` — list recent executions for a schedule
- [ ] All endpoints require auth (existing PAK middleware)
- [ ] Soak test updated to create a schedule via API and verify scheduled execution completes
- [ ] Tutorial 21 updated with schedule management examples
- [ ] Swagger/OpenAPI annotations on the new handlers

## Implementation Notes

### Existing code to wire

All in `crates/cloacina/src/runner/default_runner/cron_api.rs`:
- `register_cron_workflow(&self, workflow_name, cron_expression, timezone) -> Result<Uuid>` (line 51)
- `list_cron_schedules(&self, active_only, limit, offset) -> Result<Vec<CronSchedule>>` (line 124)
- `get_cron_schedule(&self, schedule_id) -> Result<CronSchedule>` (line 209)
- `delete_cron_schedule(&self, schedule_id) -> Result<()>` (line 183)
- `get_cron_execution_history(&self, schedule_id, limit) -> Result<Vec<CronExecution>>` (line 312)

### Files to modify
- `crates/cloacinactl/src/routes/workflows.rs` — add schedule handlers
- `crates/cloacinactl/src/commands/serve.rs` — wire new routes into the Router
- `tests/soak/soak_test.py` — add schedule creation + verification
- `docs/content/tutorials/21-server-workflow-management.md` — add schedule section

### Effort
S — thin HTTP wrappers around existing methods, same pattern as the existing execution endpoints.

## Status Updates

*To be added during implementation*
