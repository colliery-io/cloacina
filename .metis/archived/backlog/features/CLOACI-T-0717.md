---
id: reconciler-monitoring-in-the
level: task
title: "Reconciler monitoring in the Operations UI (registry-load status tile)"
short_code: "CLOACI-T-0717"
created_at: 2026-06-17T02:16:28.142505+00:00
updated_at: 2026-06-17T02:16:28.142505+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Reconciler monitoring in the Operations UI (registry-load status tile)

> **ABSORBED BY [[CLOACI-T-0718]] (delivered 2026-06-17).** Rather than build a
> separate ~5s poller, the reconciler tile shipped as one field of the
> event-driven ops-metrics payload: a **Reconciler** tile (built/available,
> failed builds, last built) now lives on the Operations page, pushed over WS.
> See T-0718. This item is closed as superseded — no separate work remains.

## Objective

Surface the registry **reconciler** in the Operations page alongside the
existing Server / Compiler / Fleet tiles, so an operator can see whether
successfully-built packages are actually being loaded into the runnable
registry — closing the visibility gap between "compiler says built" and
"workflow is executable".

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (operational visibility; not blocking execution)

### Business Justification
- **User Value**: The pipeline is compiler → reconciler → runnable. The
  Operations page shows the compiler (build queue) and the fleet (workers) but
  nothing about the reconciler, which is the step that loads built packages into
  the in-runner registry. When a package builds `success` but never becomes
  executable (load failure, reconcile lag), there is currently no tile that
  shows it — you only find out by trying to run it.
- **Business Value**: Faster diagnosis of "I uploaded it, the build went green,
  but I can't run it" — a real failure mode (see the OOM/load issues hit while
  wiring the demo).
- **Effort Estimate**: M (server endpoint + UI tile).

## Acceptance Criteria

## Acceptance Criteria

- [ ] A "Reconciler" tile on the Operations page, polling on the same cadence as
      the others (~5s).
- [ ] Shows: packages loaded/registered (runnable), packages built-but-awaiting
      load, and a coarse status badge (synced / loading / lagging).
- [ ] Surfaces last reconcile activity (timestamp or "Xs ago") if feasible.
- [ ] Clearly distinguishes "built but not loaded" (reconciler's domain) from
      "build failed" (compiler's domain, already shown).

## Implementation Notes

### Technical Approach
Mirror the compiler-status pattern (`GET /v1/compiler/status`, which is
DB-derived via `cloacina::registry::workflow_registry::build_queue_stats`).
Add a `GET /v1/reconciler/status` returning a `ReconcilerStatus` api-type.

Open question to resolve in design — what is the authoritative "loaded" count?
- `workflow_packages` where `build_status='success' AND superseded=false` is the
  clean "available/built" count (10 in the demo).
- `workflow_registry` row count is NOT a clean "loaded" count — it is
  content-addressed/historical (24 rows for 10 packages in the demo).
- The true "loaded into memory" count is the in-runner registry
  (`WorkflowRegistryImpl` / `loaded_package_count()`), reachable per-tenant via
  `AppState.tenant_runners`. The `/v1/tenants/{t}/workflows` list already reads
  the loaded set (`registry.list_workflows()`), so the count there is the
  authoritative "registered/runnable" number.
- The reconciler does not currently persist a "last reconcile" timestamp; it
  logs `Reconciler: N package(s) to load`. Exposing last-run + the live "to
  load" backlog cleanly may need a small bit of reconciler instrumentation
  (in-memory status the endpoint reads) rather than pure DB derivation.

Decide: derive from DB counts (approximate, no new instrumentation) vs. read the
in-runner registry + add a reconcile heartbeat (accurate). Lean toward the
latter for an honest "last reconcile" signal.

### Related code
- `crates/cloacina-server/src/routes/compiler.rs` — pattern to mirror.
- `crates/cloacina-server/src/routes/workflows.rs` (`list_workflows`) — how the
  loaded/registered set is read.
- `crates/cloacina/src/registry/workflow_registry/{mod.rs,database.rs}` —
  `loaded_package_count()`, `list_packages()`, `build_queue_stats()`.
- `crates/cloacina-api-types/src/compiler.rs` — `CompilerStatus` shape to mirror
  for a new `ReconcilerStatus`.
- UI: `ui/src/api/operations.ts` (add `useReconcilerStatus`) and
  `ui/src/routes/Operations.tsx` (add the tile next to Compiler/Fleet).

### Context
Started inline during the UI-polish work, then deferred by request to keep the
session focused. No code committed for it yet.

## Status Updates

- 2026-06-17: Filed. Deferred from the live UI-polish session; investigation
  showed `workflow_registry` row count is not a clean "loaded" metric, so the
  design needs to decide between DB-approximate vs in-runner-registry + reconcile
  heartbeat.