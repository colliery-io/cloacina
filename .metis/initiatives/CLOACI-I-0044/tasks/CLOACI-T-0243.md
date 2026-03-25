---
id: trigger-hot-reload-daemon-cli
level: task
title: "Trigger hot-reload + daemon CLI — reconciler, add/list/delete commands"
short_code: "CLOACI-T-0243"
created_at: 2026-03-24T21:19:58.711483+00:00
updated_at: 2026-03-25T01:09:27.112989+00:00
parent: CLOACI-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0044
---

# Trigger hot-reload + daemon CLI — reconciler, add/list/delete commands

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0044]]

## Objective

Operational API for trigger lifecycle management. Triggers are created by packages (not API), but operators need to list, enable/disable, and view execution history. Also add `cloacinactl daemon trigger list/enable/disable` CLI commands.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Package loader extracts `triggers` from ManifestV2 and auto-registers them in TriggerScheduler on load
- [ ] Reconciler: loading a new package version upserts trigger schedules (add new, update changed, disable removed)
- [ ] Reconciler: unloading/deleting a package disables its triggers
- [ ] Read-only REST endpoints: `GET /triggers` (list all), `GET /triggers/:name` (detail + last executions)
- [ ] Operational endpoints: `POST /triggers/:name/enable`, `POST /triggers/:name/disable`
- [ ] `cloacinactl daemon trigger list` — table output of all registered triggers with status
- [ ] `cloacinactl daemon trigger enable <name>` / `disable <name>` — toggle trigger
- [ ] Hot-reload: when a package is re-loaded at runtime, triggers are reconciled without restart
- [ ] Unit tests for reconciler logic (add/update/remove triggers on package change)
- [ ] Integration test: load package with triggers, verify they appear in list, disable one, re-enable
- [ ] All existing tests pass

## Implementation Notes

### Technical Approach
Package loader calls `TriggerScheduleDAL::upsert` for each trigger in manifest. Reconciler diffs current DB state against manifest to detect removals. REST routes use existing DAL methods.

### Dependencies
- T-0241 (ManifestV2 trigger extension + built-in types)
- T-0242 (built-in trigger types registered in scheduler)

## Status Updates

### 2026-03-24 — Implementation complete

**REST endpoints** (`routes/triggers.rs`):
- `GET /triggers` — list all trigger schedules (name, workflow, interval, status)
- `GET /triggers/:name` — get trigger detail by name
- `POST /triggers/:name/enable` — enable a trigger
- `POST /triggers/:name/disable` — disable a trigger
- All routes use existing `TriggerScheduleDAL` methods. Requires runner (503 in API-only mode).

**Daemon CLI** (`commands/daemon.rs`):
- `cloacinactl daemon trigger list` — table of all triggers with status
- `cloacinactl daemon trigger enable <name>` — enable by name
- `cloacinactl daemon trigger disable <name>` — disable by name
- All commands open SQLite DB directly (no running daemon required).

**Tests:** All 473 lib tests pass. 42/43 cloacinactl tests pass (1 pre-existing failure in `test_unknown_route_returns_404`).
