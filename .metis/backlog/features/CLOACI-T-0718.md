---
id: publish-operational-metrics-over
level: task
title: "Publish operational metrics over WS — event-driven Operations UI instead of polling"
short_code: "CLOACI-T-0718"
created_at: 2026-06-17T02:27:31.574720+00:00
updated_at: 2026-06-17T02:27:31.574720+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Publish operational metrics over WS — event-driven Operations UI instead of polling

## Objective

Deliver operational metrics (server / compiler / scheduler / fleet / reconciler
health and counters) to the web UI as **WS push events** over the existing
interservice communication substrate, so the Operations page becomes
**event-driven and reactive** instead of polling each endpoint on a fixed ~5s
timer. The data already exists; this changes how it reaches the browser — from
"the client asks every 5s" to "the server publishes on change / on a tick and
the UI subscribes once".

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (operational-visibility UX + scaling; not blocking execution)

### Business Justification
- **User Value**: The Operations view today is a set of independent ~5s pollers
  (server/compiler/fleet tiles; the proposed reconciler tile in [[CLOACI-T-0717]]
  is the same pattern). That means up to ~5s of staleness, a thundering-herd of
  per-tile requests per open tab, and no "something just changed" reactivity. A
  WS subscription gives near-real-time tiles, one connection per client, and the
  same live-feel the execution stream already has.
- **Business Value**: Lower server load under many open dashboards (N tabs ×
  M tiles × 12 req/min collapses to N subscriptions), faster incident diagnosis
  (a fleet member dropping / a build failing shows up immediately), and a
  consistent transport story (everything operational rides the same outbox/WS
  rails rather than half-WS / half-poll).
- **Effort Estimate**: L (server-side metrics publisher + WS topic/envelope +
  UI subscription hooks replacing the pollers).

## Acceptance Criteria

- [ ] An operational-metrics channel/topic on the existing WS substrate that the
      UI can subscribe to once and receive server/compiler/scheduler/fleet/
      reconciler status without per-tile polling.
- [ ] The Operations page tiles update from pushed events; the per-tile ~5s
      `useQuery` pollers are removed (or fall back to poll only when the WS is
      disconnected).
- [ ] Updates are pushed on meaningful change (fleet roster churn, build-status
      transition, reconcile activity) and/or on a coarse server-side tick — not
      driven by client timers.
- [ ] Reconnect / resync behaves like the execution stream: on WS drop the UI
      reconnects and re-receives current state (no permanently stale tiles).
- [ ] Multi-tenant scoping is respected — a client only receives metrics for
      tenants it is authorized for, consistent with the existing WS auth model.
- [ ] Documented WS message shape for the metrics envelope (mirrors the
      execution-event envelope / `ws-protocol.md`).

## Implementation Notes

### Technical Approach
Reuse the interservice communication substrate from [[CLOACI-I-0115]] /
[[CLOACI-S-0012]] (transactional outbox + NOTIFY-on-commit + per-replica LISTEN
+ WS push, with the versioned envelope and ack/resync from
[[CLOACI-T-0627]]). The execution-event consumer migration ([[CLOACI-T-0629]])
and the UI live execution stream ([[CLOACI-T-0656]]) are the working precedent
for "UI subscribes over WS" — model the metrics channel on those.

Open questions to resolve in design:
- **Source of the metrics.** Operational status today is read on demand from
  `/health`, `/ready`, `/metrics`, the agent roster/heartbeats, and compiler
  status (`build_queue_stats`). Decide whether the publisher (a) snapshots these
  on a server-side tick and pushes deltas, or (b) emits genuine change-events
  where the data is event-shaped (fleet join/leave/heartbeat-miss, build-status
  transition, reconcile run) and ticks only the gauge-like counters.
- **Outbox vs. ephemeral broadcast.** Execution events use the durable
  transactional outbox (replayable, acked). Operational metrics are mostly
  "latest value wins" gauges — durability/replay may be unnecessary and even
  wasteful (outbox-depth pressure). Consider an ephemeral broadcast topic
  alongside the durable outbox rather than persisting every tick.
- **Granularity / topic design.** One "operations" topic vs. per-service topics
  the UI selectively subscribes to. Keep it coarse to start (one ops topic) unless
  payload size argues otherwise.
- **Relationship to [[CLOACI-T-0717]].** The reconciler tile is currently specced
  as a ~5s poller. If this lands, the reconciler signal should be one of the
  pushed metrics rather than its own poll — coordinate so we don't ship a new
  poller we immediately rip out.

### Related code
- `crates/cloacina-server/src/` — WS push relay + outbox machinery from I-0115;
  `routes/compiler.rs` (compiler status), health/ready/metrics handlers, and the
  fleet roster/heartbeat surfaces are the data sources.
- `crates/cloacina/src/registry/workflow_registry/` — `build_queue_stats()` /
  loaded-package counts (compiler + reconciler signals).
- `docs/.../ws-protocol.md` + WS envelope JSON schemas ([[CLOACI-T-0644]]) — add
  the operational-metrics message type here.
- `ui/src/api/operations.ts` — replace `useQuery` pollers with a WS subscription
  hook; `ui/src/routes/Operations.tsx` — tiles consume pushed state.
- UI WS plumbing established by [[CLOACI-T-0656]] (live execution stream) — reuse
  the connection/reconnect machinery.

### Context
The WS push substrate ([[CLOACI-I-0115]]) and the UI's WS live-follow
([[CLOACI-T-0656]]) already exist, but operational/health visibility
([[CLOACI-I-0124]] WS-2 → [[CLOACI-T-0704]], plus the reconciler tile
[[CLOACI-T-0717]]) was built poll-first. This ticket closes that gap: bring
operational metrics onto the same event-based rails so the control plane's own
health is reactive, not polled. Relates to the broader visibility overhaul
[[CLOACI-I-0124]] and the web-UI control plane [[CLOACI-I-0117]].

## Status Updates

- 2026-06-17: Filed. No existing ticket covered event-driven operational metrics
  over WS — the Operations tiles ([[CLOACI-T-0704]]) and the proposed reconciler
  tile ([[CLOACI-T-0717]]) are poll-based, while the WS push substrate
  ([[CLOACI-I-0115]]) used for execution events ([[CLOACI-T-0656]]) is the
  natural transport to move them onto.
