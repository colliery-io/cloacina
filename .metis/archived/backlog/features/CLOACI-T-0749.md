---
id: universal-pause-pause-resume
level: task
title: "Universal pause — pause/resume controls for workflows, triggers, and executions"
short_code: "CLOACI-T-0749"
created_at: 2026-06-20T02:26:30.274166+00:00
updated_at: 2026-06-20T13:09:33.105570+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Universal pause — pause/resume controls for workflows, triggers, and executions

## Origin

Surfaced during a live demo (2026-06-18/19). There is currently no way to
temporarily stop something that is running or scheduled without deleting it or
tearing down config. If a workflow shouldn't be running right now (bad input, a
downstream outage, an in-flight investigation), the operator has no "off switch"
short of deletion. We want a pause button on "everything" so a human can hold a
thing in place and resume it later.

## Objective

Add first-class pause/resume to the operational entities so an operator can
suspend activity without destroying state, and resume cleanly afterward. "Pause"
means: stop initiating new work for the paused entity and surface its paused
state clearly; resume returns it to normal scheduling/execution.

## Backlog Item Details

### Type
- [x] Feature — operational control (server + UI)

### Priority
- [x] P1 — High (operators need a safe "stop now" that isn't deletion; demo-visible)

### Business Justification
- **User Value**: A safe, reversible "stop this" for when a workflow shouldn't be
  running — no delete-and-recreate, no lost config/state.
- **Business Value**: Incident-friendly operations; reduces risk of destructive
  workarounds during an issue.
- **Effort Estimate**: M–L (state model + scheduler/runtime honoring it + UI;
  semantics differ per entity — see below).

## Scope — what "pause everything" actually means per entity

Pause semantics are not uniform; each needs a defined behavior:

- **Trigger / cron (scheduled)**: stop firing while paused; do not enqueue new
  executions; resume re-arms on the normal schedule (define catch-up policy:
  skip missed fires vs. fire once on resume).
- **Workflow**: block new manual/triggered executions of this workflow; existing
  in-flight runs handled per the execution policy below.
- **Execution (in-flight)**: define whether pause means "stop dispatching new
  tasks in this run and hold" (pausable at task boundaries) vs. only applying to
  not-yet-started runs. In-flight task pause is the hardest case — may be scoped
  out of v1.
- **(Stretch) Agent / tenant**: pause an agent or a tenant's workloads
  wholesale — overlaps with CLOACI-T-0748; cross-reference rather than duplicate.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Persisted paused/active state on the relevant entities (at minimum:
      triggers and workflows).
- [ ] Scheduler/runtime honors paused state — no new executions are initiated for
      a paused entity.
- [ ] Resume returns the entity to normal scheduling/execution; documented
      catch-up policy for missed scheduled fires.
- [ ] UI pause/resume control on the relevant detail/list views, with clear
      paused-state indication.
- [ ] API endpoints for pause/resume (so it's scriptable, not UI-only).
- [ ] Defined and documented behavior for in-flight executions when a parent is
      paused (even if that behavior is "in-flight runs complete; only new runs are
      blocked" for v1).

## Related work

- **CLOACI-I-0117** — web UI (where the controls live).
- **CLOACI-T-0743** — Timer-driven cron scheduling. Pause for triggers interacts
  with the sleep-until-next-due scheduler; coordinate so a paused trigger doesn't
  get scheduled and the change-notify path clears/re-arms timers on
  pause/resume.
- **CLOACI-T-0748** — agent/tenant-level pause is the stretch overlap.

## Open Questions

- Scope of v1: triggers + workflows (new-execution gating) only, deferring
  in-flight execution pause? Recommend yes — ship the high-value, lower-risk
  surface first.
- Catch-up policy on resume for missed cron fires: skip vs. single catch-up fire.

## Status Updates

### 2026-06-20 — Decision: v1 scope = triggers + workflows (new-execution gating)

v1 scope locked to **triggers + workflows only**: persist paused state and have
the scheduler/runtime stop *initiating* new executions for a paused entity.
In-flight executions complete normally (no task-boundary hold in v1). Pausing
in-flight executions is explicitly deferred to a follow-up. This is the
lower-risk, high-value slice and ships without runtime/dispatcher resumability
changes.

### 2026-06-20 — Design (seams mapped): two paused flags, gate at the callers

Recon finding: triggers/cron are rows in the unified `schedules` table, but
manual workflow execution gates on `workflow_packages` (by name). So v1 uses
**two `paused` flags**, not one:

- `schedules.paused` — gates trigger/cron firing (scheduler skips paused
  schedules; covers triggered executions of a paused trigger).
- `workflow_packages.paused` — gates workflow execution regardless of source.

Gating is placed at the **caller chokepoints** (not inside the core
`execute_async` hot path):
- Manual REST: `crates/cloacina-server/src/routes/executions.rs:~128` (before
  `execute_async`) → refuse with `409 workflow_paused` if the workflow's package
  is paused.
- Scheduler fire: `crates/cloacina/src/cron_trigger_scheduler.rs` — skip a
  schedule whose `paused` is set; and before firing a workflow, skip if the
  target workflow package is paused.

Storage: one migration per backend (postgres + sqlite) ALTERing BOTH tables —
`ADD COLUMN paused <bool> NOT NULL DEFAULT false` + `paused_at` (nullable ts) +
partial index on `paused`. ADD COLUMN only (no DROP/CREATE), per project
migration convention. Next numbers: postgres `029_*`, sqlite `025_*`.

Resume catch-up policy (open question answered): **skip missed fires** — resume
re-arms on the normal schedule; no catch-up burst for fires missed while paused.

Plumbing: `schema.rs` (both tables) → `models/schedule.rs` + the
`workflow_packages` model → DAL pause/resume + paused getters →
two caller gates → 4 REST endpoints (workflow + trigger × pause/resume) +
registration → api-types `paused`/`paused_at` on `WorkflowSummary`/`Detail` and
`TriggerScheduleSummary`/`Info`. Touches cloacina + cloacina-server +
cloacina-api-types. Coordinates with T-0743 (timer scheduler): the change-notify
path should re-arm/clear timers on pause/resume.

### 2026-06-20 — IMPLEMENTED + VERIFIED: schedules-pause half (cron + trigger)

The **schedule (cron + trigger) pause vertical is done and verified** on branch
`feat/i0126-legibility`. Built incrementally with `cargo check` after each layer
(angreal's wrapper exits 1 on a graceful network self-check, so used cargo
directly).

Done:
- **Migrations** `postgres/029_add_pause_to_scheduling` + `sqlite/025_*` — ADD
  COLUMN `paused`/`paused_at` + partial index on BOTH `schedules` and
  `workflow_packages` (workflow_packages columns staged for the deferred half).
- **schema.rs** (`unified_schema` only — diesel uses explicit column lists, so
  the native postgres/sqlite modules don't need it) + `UnifiedSchedule`/
  `UnifiedWorkflowPackage` Queryable structs + `Schedule` domain model with
  `is_paused()` / `is_active()`.
- **DAL**: `ScheduleDAL::pause/resume`; paused-exclusion `.filter` added to
  get_due_cron / next_cron_due_time / claim_and_update_cron / get_enabled_triggers
  — so a paused cron is invisible to the firing path AND the T-0743 next-due
  timer (busy-loop guard). Scheduler needs no code change; gating is at the DAL.
- **REST**: `POST /v1/tenants/{tid}/triggers/{name}/pause` + `/resume`
  (resolves by trigger or workflow name; works for cron + trigger schedules;
  `can_write` gated) → `TriggerPauseResponse`. `paused`/`paused_at` surfaced on
  the triggers list + detail responses. Registered + OpenAPI updated; spec
  regenerated and `spec-check` passes.

Verification: `cargo test -p cloacina --lib` → **708 passed, 0 failed**, incl.
2 new gating tests (`test_pause_resume_excludes_due_cron`,
`test_pause_excludes_enabled_trigger`). cloacina-api-types + cloacina-server
`cargo check` clean. SQLite migration verified (test DBs apply it). **Postgres
migration NOT yet run** (needs the integration/Docker lane).

Catch-up policy: **skip** (resume re-arms on normal schedule; no missed-fire
burst). Resume latency: takes effect at the next scheduler tick / poll; no
explicit T-0743 change-notify poke yet (acceptable for v1; noted).

### 2026-06-20 — COMPLETE: workflow_packages-pause half (manual execute gate)

The deferred half is now done and verified — T-0749 is **complete** (both
halves). Added on `feat/i0126-legibility`:
- **Registry/core**: `WorkflowRegistryImpl::set_package_paused(id, paused)`
  (diesel update of `workflow_packages.paused`/`paused_at`),
  `set_workflow_paused(name, …)` + `is_workflow_paused(name)` (resolve by
  `workflow_name` or `package_name` via the active list). `paused` added to
  `WorkflowMetadata` (`#[serde(default)]`) and populated from the row at every
  list/inspect construction site.
- **Execute gate**: `routes/executions.rs` refuses a paused workflow before
  `execute_async` with **409 `workflow_paused`** (fails open on registry error
  so a transient fault never wedges execution). Covers manual + triggered runs.
- **REST**: `POST /v1/tenants/{tid}/workflows/{name}/pause` + `/resume` →
  `WorkflowPauseResponse` (resolves by workflow or package name; `can_write`).
  `paused` surfaced on `WorkflowSummary` + `WorkflowDetail`. Registered +
  OpenAPI; spec regenerated and `spec-check` passes.

Verification: `cargo test -p cloacina --lib` → **709 passed, 0 failed**, incl.
new `test_workflow_pause_resume` (pause/resume by name reflected in
`list_all_packages` + `is_workflow_paused`; unknown workflow → not paused / no-op).
cloacina + cloacina-api-types + cloacina-server `cargo check` clean.

### All Acceptance Criteria met (v1)
- [x] Persisted paused state — `schedules.paused` + `workflow_packages.paused`.
- [x] Scheduler honors paused — paused schedules excluded from all fire paths
  (incl. the T-0743 next-due timer); paused workflows refused at execute.
- [x] Resume restores normal scheduling/execution; catch-up policy = **skip**.
- [x] API endpoints for pause/resume (triggers + workflows), scriptable.
- [x] In-flight behavior documented: in-flight runs complete; only new runs are
  blocked (no task-boundary hold in v1 — deferred follow-up).
- UI controls (AC line) are intentionally **out of scope** here (designer is
  mid-review); the data + endpoints are ready for the UI to consume later.

### 2026-06-20 — Postgres integration lane: GREEN

`angreal test integration` (real Postgres via Docker) → **exit 0, 0 failed**
across the integration suites (312 + 92 + 6). The `029_*` migration applied
cleanly against real Postgres and the pause integration scenarios (incl.
`pause_fail_test_pipeline`) ran green. `angreal test unit` also green
(709 cloacina + 47 cloacina-workflow). Closes the prior "needs postgres lane"
caveat — both migration backends now verified.