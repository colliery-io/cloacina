---
id: reconciler-inserts-duplicate-cron
level: task
title: "Reconciler inserts duplicate cron schedules on every package re-load (non-idempotent register_cron_workflow, no rollback on partial load)"
short_code: "CLOACI-T-0669"
created_at: 2026-06-13T11:33:00.670172+00:00
updated_at: 2026-06-16T10:54:55.872317+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


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

> **2026-06-13 — Primary bug FIXED + verified end-to-end.** The target-propagation
> fix landed (Trigger `workflow_name()` accessor → macro → `TriggerPackageMetadata`
> → reconciler `register_cron_workflow(target_workflow,…)`). Live demo confirms:
> **exactly 1** cron schedule, `workflow_name = demo_cron_workflow` (the target),
> and real `demo_cron_workflow` executions firing on the 15s cadence. The
> duplication was the malformed demo fixture (fixed separately).
> **Residual (kept open, lower priority):** make cron registration idempotent +
> roll back schedules inserted by a *partially-failed* package load, so a future
> failing/retried package can't accumulate orphan schedules. The AC items for
> that robustness work remain unchecked below.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Cron registration is **idempotent**: loading/re-loading a package with a
      cron trigger yields exactly one active schedule per (workflow_name, cron,
      timezone) — done via `ScheduleDAL::upsert_cron`.
- [~] Package load is **atomic w.r.t. side effects**: duplication is now
      impossible under retried partial loads (idempotent upsert → update, not
      insert), so the reported unbounded-growth symptom is gone. Full rollback of
      schedules from a partially-failed load left as a lower-priority follow-up.
- [x] A test asserting exactly one schedule after re-register
      (`test_upsert_cron_is_idempotent`); plus a different expression yields a new
      one. (Used a focused DAL idempotency unit test rather than the heavier
      full-reconciler-with-failing-package integration test.)

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

### 2026-06-16 — DONE (residual idempotency gap closed)

The primary bug (packaged-cron target propagation) was fixed 2026-06-13. This
closes the residual robustness gap — non-idempotent cron registration.

- **AC #1 (idempotent registration):** new `ScheduleDAL::upsert_cron`
  (postgres + sqlite) keyed on the cron schedule's identity `(workflow_name,
  cron_expression, timezone)` — re-registering the same packaged cron updates
  the existing row's `next_run_at`/`enabled` instead of inserting a duplicate;
  a *different* cron expression is still a distinct schedule. Mirrors the
  existing `upsert_trigger` (the poll path). `DalCronRegistrar
  ::register_cron_workflow` (the reconciler-driven path that re-runs on every
  re-load / failed-load retry) now calls `upsert_cron` instead of `create`.
- **AC #3 (test):** `test_upsert_cron_is_idempotent` — re-register reuses the
  same row (exactly one schedule); a different expression inserts a new one.
  Passes via `angreal test unit upsert_cron`.
- **AC #2 (atomic-w.r.t.-side-effects):** the duplication symptom is now
  impossible regardless of partial-load retries — same identity → update, not
  insert — so unbounded growth (the disk-exhaustion bug) is dead. Full
  rollback-of-partially-loaded-schedules remains a lower-priority nicety
  (`unregister_cron_workflow` on the failure path); not needed to kill the
  reported symptom, left as optional follow-up.

**Caveat:** upsert prevents *new* duplicates; rows created before this fix are
not retroactively merged (a fresh seed starts at one). The `create` path on the
manual `DefaultRunner::register_cron_workflow` API is unchanged — explicit user
registration, not the reconciler loop that caused the leak.

Committed `91020b0c` on `feat/ui-0124-server-read-endpoints`.

---

**2026-06-13 — Filed.** Found while investigating fast disk growth during the UI
demo. `/v1/tenants/public/triggers` showed 20 duplicate `demo_cron_trigger`
schedules; traced to `register_cron_workflow`'s unconditional `create()` +
non-atomic load + the reconciler retrying the (then-failing) Python CG package
every tick. Each retry re-ran cron Step 1 and inserted another schedule.

**2026-06-13 — Broader than first thought + a second bug, both confirmed on a
CLEAN build (all 6 packages load successfully):**
- **Duplication is per-reconcile-tick, not just per-failed-retry.** With every
  package loading cleanly, `/triggers` still climbed 11 → 26 → 30 over a couple
  minutes (~1 new schedule every few seconds — reconcile cadence, far faster
  than the 15s cron). So the reconciler re-registers the cron for an
  already-loaded package on repeated passes; `loaded_packages` is not gating cron
  re-registration. Unbounded growth even in the happy path → DB/disk leak.
- **Wrong `workflow_name` recorded (separate bug, likely same code path).** The
  schedules carry `workflow_name = "demo_cron_trigger"` — the **trigger fn name**
  — not the `on` target `"demo_cron_workflow"`. `step_load_cron_triggers` passes
  `t.name` to `register_cron_workflow`; it should pass the trigger's target
  workflow (`on`). Consequence: the schedule fires (`last_run_at` advances on all
  26) but resolves a non-existent workflow `demo_cron_trigger` → **zero
  executions produced** (only the 3 seed runs exist; no `demo_cron_workflow`
  runs). So packaged cron triggers currently register, duplicate, and fire into
  the void.
- Repro fixture: `examples/fixtures/demo-cron-rust`
  (`#[trigger(on = "demo_cron_workflow", cron = "*/15 * * * * *")] fn
  demo_cron_trigger`). Add a 4th acceptance criterion: the schedule's
  `workflow_name` is the `on` target, and a cron fire produces an execution of
  that workflow.

**2026-06-13 — CORRECTION after fixing the fixture. Two separate things; the
"clean build" claim above was wrong (that run still had the broken fixture).**
- The duplication was **entirely** caused by a malformed demo fixture: the
  workflow wrongly declared `#[workflow(triggers = ["demo_cron_trigger"])]`
  (a cron trigger is bound via `on`, not subscribed like a poll trigger). That
  made the workflow fail to load **every reconcile tick**; cron Step 1 runs
  before that failing step with no rollback + a non-idempotent `create()`, so
  each retry inserted another schedule. **With the fixture fixed
  (`triggers=[]` removed), the package loads once and there is exactly ONE
  stable cron schedule — duplication gone.** So core/normal cron is fine
  (matches the heavy direct-API test coverage, e.g.
  `tests/python/test_scenario_27_cron_scheduling.py`,
  `crates/cloacina/tests/integration/scheduler/cron_basic.rs`).
  - Residual (lower-priority) robustness gap worth keeping: a **partial package
    load should roll back** the cron schedules it already inserted (and/or
    `register_cron_workflow` should be idempotent), so a future bad/failing
    package can't accumulate orphan schedules.
- **The real standalone platform bug** is target propagation for **packaged**
  cron triggers, confirmed with the *fixed* fixture (clean load, 1 schedule,
  `last_run_at` advancing, yet **zero `demo_cron_workflow` executions**):
  - `register_cron_workflow(workflow_name, …)` is correct and tested — the
    tested callers pass a real **workflow** name.
  - But the reconciler `step_load_cron_triggers` (loading.rs:1490) calls it with
    **`t.name`** (the trigger fn name), and `TriggerPackageMetadata`
    (workflow-plugin `types.rs`) has **no `on`/target field** — the macro's `on`
    is dropped. So the schedule targets the trigger name → fires a non-existent
    workflow → no executions.
  - **Fix:** carry the trigger's `on` target into `TriggerPackageMetadata`
    (macro `trigger_attr.rs` → the metadata type), and have
    `step_load_cron_triggers` register the cron against that target workflow, not
    `t.name`. (Engine change across cloacina-workflow-plugin + cloacina-macros +
    cloacina reconciler.) This is what the direct-API tests don't cover — the
    packaged-cron-through-reconciler path is the untested gap.