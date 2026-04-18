---
id: t6-workflow-execution-graph-verbs
level: task
title: "T6: workflow + execution + graph verbs"
short_code: "CLOACI-T-0515"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:08.831219+00:00
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0513]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T6: workflow + execution + graph verbs

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Implement the `workflow`, `execution`, and `graph` nouns. These three are the main "driving the platform" surface: users trigger workflows, watch executions, and manage computation graphs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `workflow list [--package <ID>]` — GET `/v1/workflows/index`. Columns: NAME, PACKAGE, TENANT, ENABLED, NEXT_RUN.
- [ ] `workflow inspect <NAME>` — GET `/v1/workflows/by-name/{name}`. Tasks, deps, trigger rules, schedules.
- [ ] `workflow run <NAME> [--context <FILE|->]` — POST `/v1/workflows/{name}/run` with context JSON (file path or stdin `-`). Returns execution ID.
- [ ] `workflow enable <NAME>` / `workflow disable <NAME>` — POST `/v1/workflows/{name}/{enable|disable}`. Idempotent.
- [ ] `execution list [--workflow <N>] [--status <S>] [--limit <N>]` — GET `/v1/executions`. Columns: ID (truncated), WORKFLOW, STATUS, STARTED, DURATION.
- [ ] `execution status <ID>` — GET `/v1/executions/{id}`. Human summary: state, per-task grid (name / state / duration / attempts).
- [ ] `execution events <ID> [--follow] [--since <DUR>]` — GET `/v1/executions/{id}/events` (SSE when `--follow`). Streams newline-delimited events.
- [ ] `execution cancel <ID>` — POST `/v1/executions/{id}/cancel`.
- [ ] `graph list` — GET `/v1/graphs`. Columns: NAME, PACKAGE, STATE, ACCUMULATORS, LAST_EMISSION.
- [ ] `graph status <NAME>` — GET `/v1/graphs/{name}`. Health: reactor state, per-accumulator backlog, last emission, last reaction.
- [ ] `graph pause <NAME>` / `graph resume <NAME>` — POST `/v1/graphs/{name}/{pause|resume}`.
- [ ] Integration tests for the happy path of each verb.

## Implementation Notes

### `workflow run --context`

Accepts:
- `--context path.json` — read from file.
- `--context -` — read from stdin.
- Absent — use `{}` as empty context.

Defer `--param KEY=VALUE` scalar overrides to v1.1.

### `execution events --follow`

Audit the server for SSE vs WebSocket. If only WebSocket today, this task adds an SSE wrapper (server side) or pivots to WebSocket client-side. Surface the decision in status updates.

### Per-task grid in `execution status`

Reuse the renderable infrastructure but nested — the parent execution row plus a child table of tasks.

### 404 mapping

Map server 404s to `CliError::NotFound` so the shell exits 3 cleanly.

## Status Updates

*To be added during implementation*
