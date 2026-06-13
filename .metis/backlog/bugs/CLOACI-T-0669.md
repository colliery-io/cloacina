---
id: reconciler-inserts-duplicate-cron
level: task
title: "Reconciler inserts duplicate cron schedules on every package re-load (non-idempotent register_cron_workflow, no rollback on partial load)"
short_code: "CLOACI-T-0669"
created_at: 2026-06-13T11:33:00.670172+00:00
updated_at: 2026-06-13T11:33:00.670172+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Reconciler inserts duplicate cron schedules on every package re-load (non-idempotent register_cron_workflow, no rollback on partial load)

## Objective **[REQUIRED]**

A packaged cron trigger can register **many duplicate schedule rows** for the
same workflow, all firing it on the same cron — multiplying executions and
bloating the DB (and, in Docker, the VM disk). Root cause: cron registration is
non-idempotent **and** runs as the first step of a non-transactional package
load, so any later load-step failure (which the reconciler retries every tick)
re-inserts the schedule each retry.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

P1: unbounded schedule growth → an execution storm (N copies of every cron fire)
→ runaway DB/disk growth. Observed contributing to a host-disk exhaustion that
crash-looped Docker.

### Impact Assessment **[CONDITIONAL: Bug]**
- **Reproduction**:
  1. Upload a package with a cron trigger (e.g. `demo-cron-rust`,
     `#[trigger(cron = "*/15 * * * * *")]`) to a server whose reconciler also has
     another package that **fails to load** (so the reconciler retries every tick).
  2. `GET /v1/tenants/public/triggers` → many duplicate rows, same
     `workflow_name` + `cron_expression`, distinct `id`, `created_at` seconds apart.
  - **Observed**: 20× `demo_cron_trigger` schedules in a single run → ~20× the
    intended cron executions.
- **Expected**: exactly one active schedule per (workflow, cron) regardless of
  how many times the package is loaded/retried.

### Root cause
- `register_cron_workflow` (`crates/cloacina/src/runner/default_runner/cron_api.rs`)
  does an **unconditional INSERT**: `dal.schedule().create(new_schedule)` — no
  existing-schedule check, no upsert, no unique constraint.
- `step_load_cron_triggers` (`crates/cloacina/src/registry/reconciler/loading.rs`,
  "Step 1") calls it during package load, **before** later steps (custom
  triggers, reactors, CGs) that can fail.
- The reconciler only marks a package loaded (in `loaded_packages`) on **full**
  success; a failure in a later step leaves it unloaded, so the next reconcile
  tick **re-loads** it — re-running Step 1 and inserting another schedule. The
  already-inserted schedule from the failed attempt is never rolled back.
- Net: every retry of a partially-failing load adds one more duplicate cron row.
  (Even without failures, the non-idempotent INSERT is a latent bug on any
  legitimate re-load.)

## Acceptance Criteria **[REQUIRED]**

- [ ] Cron registration is **idempotent**: loading/re-loading a package with a
      cron trigger yields exactly one active schedule per (workflow_name, cron,
      timezone) — via a pre-insert existence check, an upsert, or a unique
      constraint + ON CONFLICT.
- [ ] Package load is **atomic w.r.t. side effects**: a failure in a later load
      step rolls back / cleans up the cron schedules (and other registrations)
      created earlier in the same load — or crons are registered only after the
      full load succeeds.
- [ ] A test: upload a cron package alongside a perpetually-failing package;
      assert `/triggers` holds exactly one schedule after several reconcile ticks.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Simplest robust fix: make `register_cron_workflow` upsert on
(workflow_name, cron_expression, timezone) (or add a unique index + ON CONFLICT
DO UPDATE next_run_at). Additionally, move cron registration to the end of the
load pipeline (after the steps that can fail), or have the reconciler call
`unregister_cron_workflow` for the package's schedule ids when a load attempt
fails partway. The reconciler already tracks `cron_schedule_ids` per load and has
`unregister_cron_workflow` on unload — reuse it on the failure path.

### Dependencies
Surfaced alongside CLOACI-T-0665/0666 (Python packaging) — the failing package
that triggered the retries here was the mis-laid-out Python CG. Independent fix.

## Status Updates **[REQUIRED]**

**2026-06-13 — Filed.** Found while investigating fast disk growth during the UI
demo. `/v1/tenants/public/triggers` showed 20 duplicate `demo_cron_trigger`
schedules; traced to `register_cron_workflow`'s unconditional `create()` +
non-atomic load + the reconciler retrying the (then-failing) Python CG package
every tick. Each retry re-ran cron Step 1 and inserted another schedule.
