---
id: web-ui-visibility-overhaul
level: initiative
title: "Web UI visibility overhaul — operations health, execution drill-down, graph-as-source-of-truth"
short_code: "CLOACI-I-0124"
created_at: 2026-06-16T01:38:34.599962+00:00
updated_at: 2026-06-17T12:04:31.659395+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: web-ui-visibility-overhaul
---

# Web UI visibility overhaul — operations health, execution drill-down, graph-as-source-of-truth Initiative

Parent UI initiative: builds on [[CLOACI-I-0117]] (the tenant-scoped control-plane SPA). Source of findings: a programmatic UX walk of the seeded demo stack (2026-06-16). Full report + screenshots: `/tmp/cloacina-ui-uat/ux-report.md` (01-connect … 13-workflow-upload + node-click/2nd-graph variants).

## Context

The web UI ([[CLOACI-I-0117]]) covers the object surfaces (workflows, executions,
triggers, graphs, keys) but a question-first UX audit — driving the live seeded
UI with Playwright and reading every surface under "what *should* this tell an
operator / what does it / what would a naive user misread" — found the UI thin on
**visibility**: it lists objects but doesn't let an operator answer "why did this
run do that," "is my deployment healthy," or "what does this node actually do."

Audit evidence (per-surface) lives in the report; the load-bearing findings:
- **Execution detail is the weakest surface** — a flat "Event log" of
  `task_marked_ready`/`task_completed`/`workflow_completed`, each with an empty
  `{}` payload. No task list, durations, output/context, DAG, or failure reason.
  A naive user reads `{}` as "data is broken."
- **No deployment/operations health anywhere** — nothing on server, compiler,
  scheduler, executor-agent fleet, or reconciler. The data exists (`/health`,
  `/ready`, `/metrics`, agent roster, compiler status) but isn't surfaced.
- **The graph isn't the source of truth** — triggers/reactors and accumulators are
  shown as *text*, not as nodes in the DAG; clicking a node does nothing (no code,
  inputs/outputs, retry policy, routing rule).
- **Overview under-informs and misleads** — summary cards ("Recent status:
  COMPLETED 5", "3 loaded") instead of paginated workflow/graph lists; the counts
  read as totals and hide a WARMING graph.
- **Triggers = cron only**; non-cron (event/poll/reactor) triggers don't surface.
- Naive-confusion bugs: Settings prints "Built in T-0651." to users; graph
  packages show "Tasks: 0" (reads as broken, really they're CGs not task workflows);
  raw status enums (`"socket_only"`, WARMING, WHEN_ANY) shown unexplained.

Bright spots to preserve: the **Graphs list** (real LIVE/WARMING health + per-source
accumulator status) and **graph detail's branch-aware DAG** (labeled decision edges,
e.g. `decision → audit_logger [NoAction] / signal_handler [Trade]`).

Design north star: draw from **Airflow 2.0's visibility** (Grid + Graph + Gantt +
Log + Code per run; health/admin views) without replicating its UI wholesale.

## Goals & Non-Goals

**Goals:**
- **Execution drill-down** that answers "what ran, in what order, how long, with what output, and why" — task/node-level rows + a per-task-status DAG + per-task output/context/logs + failure reason, replacing the flat event log.
- **Operations/health view** — server, compiler, scheduler, executor-agent fleet (roster + heartbeats), reconciler lag; built on the existing health/metrics/agent surfaces.
- **Graph as the source of truth** — render triggers/reactors and accumulators as first-class nodes in both workflow DAGs and computation graphs (with next/last fire + accumulator source health on the node); click any node → its code/logic, inputs/outputs, dependencies, retry policy, routing rule.
- **Overview = paginated workflows + graphs lists** with type/health/last-run columns; remove the misleading summary-count cards.
- **Non-cron triggers** surfaced in the Triggers view (event/poll/reactor) with type + enable/disable/run-now.
- **Naive-user polish** — explain status vocabulary inline; stop leaking internal task codes; fix "Tasks: 0" on CG packages.

**Non-Goals:**
- Re-platforming the SPA or changing the auth/connection model (stays on [[CLOACI-I-0117]]'s foundation).
- New server *capabilities* — this surfaces data the server already exposes; any genuinely missing API is a scoped dependency, not the bulk of the work.
- Replicating Airflow's full feature set (inspiration, not parity).
- Expanding the **demo fixtures** to richer example graphs — valuable for showing structure, but it's a demo-stack change, tracked as a small separate item, not core UI work.

## Detailed Design

A **server-data audit precedes UI work**: each surface needs an inventory of what
the API already exposes vs. what's missing, because the non-goal is "no new server
capabilities" — so any gap (e.g. per-task output/duration on an execution, node
source) is either already in `/v1/...`/metrics or becomes a scoped server dependency
called out before the UI task starts.

Per workstream:
- **Execution drill-down:** replace the flat event-log with a task/node model
  derived from the execution's events + task records. Likely needs the server to
  expose per-task rows (status, start/end, attempts, output/context, error) if the
  events alone don't carry it — audit first. UI: a task table + a DAG colored by
  per-task status (reuse the existing react-flow renderer) + a task drawer (output,
  context diff, logs, failure reason).
- **Operations/health:** new top-level nav section. Data sources already present —
  `/health`, `/ready` (DB + crashed-graph check), `/metrics` (scheduler/fleet
  counters), the agent roster + heartbeats (fleet), compiler status. UI: status
  tiles per service + fleet roster table + reconciler/scheduler liveness.
- **Graph-as-source-of-truth:** extend the DAG renderer to inject trigger/reactor
  and accumulator nodes (distinct node types/styling) upstream of the compute
  nodes, carrying next/last-fire and source health. Node click → detail drawer with
  source/logic + I/O + retry/routing. Shared node-drawer component across workflow
  DAGs and CGs.
- **Overview:** swap summary cards for paginated, faceted workflow + graph lists
  (type, health, last-run); keep a small genuine health rollup if useful.
- **Triggers:** generalize the Triggers view beyond cron (type column already
  exists) — event/poll/reactor rows, detail, enable/disable/run-now.
- **Polish:** status-vocabulary tooltips/legends; remove the Settings task-code
  placeholder; CG packages show type instead of "Tasks: 0".

## UI/UX Design

Inspiration (not replication) from **Airflow 2.0**: execution drill-down ≈ Grid +
Graph + Gantt + Log + Code; ops view ≈ Admin/health; trigger-as-graph-node ≈ the
DAG schedule header + calendar. Reuse the existing react-flow graph renderer and the
current table/badge design language ([[CLOACI-I-0117]]) — this is additive depth on
the established SPA, not a redesign. UAT is the same programmatic Playwright walk
re-run against each shipped surface (the audit harness lives at `ui/e2e/walk*.spec.ts`).

Key user flows to satisfy:
1. "A run failed — why?" → execution detail shows the failed task, its error, its output/context, and where it sits in the DAG.
2. "Is my deployment healthy?" → ops view shows server/compiler/scheduler/fleet at a glance.
3. "What does this node do / what feeds it?" → click a node → code + I/O; trigger/accumulator visible as upstream nodes.

## Alternatives Considered

- **Patch the existing event-log in place** (just pretty-print the `{}` payloads) —
  rejected: the payloads are empty and the model is event-stream, not task-centric;
  operators need the task/DAG framing, not nicer events.
- **A separate Grafana/Prometheus dashboard for ops health instead of in-UI** —
  rejected as the primary answer: operators shouldn't leave the control plane to
  learn the control plane is up; `/metrics` can still back deeper dashboards.
- **Replicate Airflow's UI** — rejected: too heavy; cherry-pick its visibility
  patterns onto Cloacina's primitives (reactors/accumulators have no Airflow analog).
- **One big rewrite of all surfaces** — rejected: ship per-workstream, audit server
  data first, keep the branch green; the surfaces are largely independent.

## Implementation Plan

**PROPOSED decomposition (pending review before tasks are created).** Ordered by the
P0/P1/P2 from the audit; each starts with a server-data audit spike.

- **WS-0 — Server-data audit (spike).** For execution drill-down, ops health, and
  node code/IO: confirm what `/v1/*` + metrics + fleet/compiler already expose; list
  any genuinely missing endpoints as scoped dependencies. Gates the rest.
- **WS-1 (P0) — Execution drill-down.** Task table + status-colored DAG + task drawer
  (output/context/logs/error), replacing the event log.
- **WS-2 (P0) — Operations/health view.** New nav section: server/compiler/scheduler/
  fleet roster + heartbeats/reconciler.
- **WS-3 (P0) — Overview → paginated workflows + graphs lists** with type/health/last-run.
- **WS-4 (P1) — Trigger/reactor + accumulator as graph nodes** (shared renderer change).
- **WS-5 (P1) — Node/task detail drawer** (code/logic, I/O, retry, routing) — shared component.
- **WS-6 (P1) — Non-cron triggers** in the Triggers view (type, detail, enable/disable/run-now).
- **WS-7 (P2) — Naive-user polish** (status-vocab tooltips, kill "T-0651" placeholder, "Tasks: 0" → type).
- **WS-8 (P2, separate) — Expand demo fixtures** to richer example graphs (full-pipeline/routing/accumulators) — demo-stack, not UI.

Definition of done: an operator can answer "why did this run do that," "is my
deployment healthy," and "what does this node do" from the UI; the graph shows
triggers/reactors/accumulators as nodes; non-cron triggers surface; the audit's
naive-confusion bugs are gone; each surface re-passes the Playwright walk.

## Status Updates
- 2026-06-17: **Completed.** All WS-* children (T-0702–T-0714) landed and are
  marked completed; the operations-health view, execution drill-down, graph-as-
  source-of-truth nodes, non-cron triggers, node detail drawer, list-page health,
  meaningful event log, and richer demo CG data all shipped on branch
  `authoring-cruft-i0125`. Closing the initiative (was stuck in discovery while
  its tasks were all done).