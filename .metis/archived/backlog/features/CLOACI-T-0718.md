---
id: publish-operational-metrics-over
level: task
title: "Publish operational metrics over WS — event-driven Operations UI instead of polling"
short_code: "CLOACI-T-0718"
created_at: 2026-06-17T02:27:31.574720+00:00
updated_at: 2026-06-17T03:55:56.412713+00:00
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

## Acceptance Criteria

- [x] An operational-metrics channel on the existing WS substrate the UI
      subscribes to once and receives server/compiler/fleet/reconciler status
      without per-tile polling. ✅ `ops_metrics:global`, SDK `followOpsMetrics`.
- [x] The Operations page tiles update from pushed events; the per-tile ~5s
      pollers are removed. ✅ `useFleet`/`useCompilerStatus` deleted; the page
      uses `useLiveOpsMetrics`. (`useServerHealth` kept ONLY for the always-on
      header liveness dot — a single `/health` probe, not a tile poller.)
- [x] Updates are pushed on a coarse server-side tick (~5s), not client timers.
      ✅ And the publish is gated on a connected subscriber, so nothing is
      gathered/pushed when no Operations page is open.
- [x] Reconnect / resync behaves like the execution stream. ✅ Reuses the SDK
      `subscribeDelivery` (ticket re-mint + reconnect/backoff); ops metrics are
      latest-snapshot so a fresh push supersedes any missed one.
- [x] Tenant scoping respected. ✅ `ops_metrics:global` is tenant `None`; the
      sink matches `(recipient, tenant)` exactly, so only admin keys
      (`tenant_id=None`) receive it — same fence as the admin REST endpoints.
- [x] Message shape documented. ✅ `OpsMetricsEvent` in
      `cloacina-api-types::operations` (kind `"ops_metrics"` on the standard
      delivery `Push` envelope).

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

- 2026-06-17: ACTIVE. Scope = **substrate + all ops tiles** (server/compiler/
  fleet/reconciler). **Absorbs CLOACI-T-0717** (reconciler = one field in the
  ops payload, no separate poller). **CLOACI-T-0719** reuses this pattern next.

  **Design — direct WS publish, NO outbox rows.** Ops metrics are ephemeral
  latest-snapshot; the durable `delivery_outbox` (NOTIFY + sweeper + no retention)
  is the wrong tool and would leak rows. Instead publish straight to the
  in-memory `WsDeliverySink` (the `(recipient,tenant)→mpsc::Sender<ServerMessage>`
  registry the WS handler already pumps to the socket), bypassing the outbox +
  relay entirely:
  - `WsDeliverySink::push_direct(recipient, tenant, ServerMessage)` → send to the
    connected sender if present, no-op otherwise (subscriber-gating for free,
    zero rows when no Operations page is open).
  - Reuse everything else: same `/v1/ws/delivery/ops_metrics:global` route, same
    ticket auth, same SDK `subscribeDelivery`/resync. The durable outbox is
    untouched — still the right tool for execution events / work packets.
  - Scope/auth: recipient `ops_metrics:global`, tenant `None` (admin keys are
    `tenant_id=None`; sink matches `(recipient,tenant)` exactly → admin-only,
    same fence as the REST endpoints).
  - Push `id`: monotonic counter (NOT a DB row id) for the SDK's dedup; the
    client `ack` lands on `mark_acked(<unknown id>)` → benign no-op.
  - Publisher: dedicated `tokio` task (~5s tick) gathering ServerHealth
    (db/graph ready) + `build_queue_stats` (compiler) + `agent_registry.snapshot()`
    (fleet) + a DB-cheap reconciler signal (success/failed package counts +
    last_built_at); calls `push_direct`.

  ### Plan
  1. api-types: `OpsMetricsEvent { server, compiler, fleet, reconciler, ts }`
     (reuse `CompilerStatus`, `AgentInfo`; add `ServerHealthLite`,
     `ReconcilerStatus`).
  2. sink: `push_direct(recipient, tenant, ServerMessage) -> bool` +
     `has_recipient` (for an optional skip-gather optimization).
  3. server: `ops_metrics.rs` publisher; spawn in lib.rs; monotonic id counter.
  4. SDK: `followOpsMetrics(client, scope)` over `subscribeDelivery`.
  5. UI: `useLiveOpsMetrics()` replacing the 4 pollers; tiles push-driven +
     reconciler tile.
  6. Build/deploy/verify the Operations page updates live with no polling.

- 2026-06-17: DONE + verified. Implemented exactly the direct-WS-publish design.
  Files: `cloacina-api-types/src/operations.rs` (`OpsMetricsEvent`,
  `ServerHealthLite`, `ReconcilerStatus`); `workflow_registry::reconciler_stats`
  (built/failed/last_built); `WsDeliverySink::{push_direct,has_recipient}`;
  `cloacina-server/src/ops_metrics.rs` (publisher) + spawn in lib.rs; SDK
  `followOpsMetrics`; UI `useLiveOpsMetrics` + rewritten `Operations.tsx`
  (4 push-driven tiles incl the new **Reconciler** tile — **T-0717 absorbed**).
  Verified on the demo: Operations page shows LIVE + "updated just now", all
  tiles populated (Reconciler: built/available 10, 0 failed; Fleet: 3 agents
  with live heartbeats), and `SELECT COUNT(*) FROM delivery_outbox WHERE
  recipient LIKE 'ops_metrics%'` = **0** (no durable rows accrued).
  ACCEPTANCE MET. CLOACI-T-0719 will reuse this same direct-push pattern as a
  per-task-state channel.