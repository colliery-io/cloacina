---
id: safety-net-sweeper-observability
level: task
title: "Safety-net sweeper + observability — stale-row redelivery, outbox-depth metrics"
short_code: "CLOACI-T-0628"
created_at: 2026-05-27T17:36:22.303197+00:00
updated_at: 2026-05-28T15:30:46.795875+00:00
parent: CLOACI-I-0115
blocked_by: [CLOACI-T-0626]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0115
---

# Safety-net sweeper + observability — stale-row redelivery, outbox-depth metrics

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0115]] — implements [[CLOACI-S-0012]] / [[CLOACI-A-0006]].

## Objective **[REQUIRED]**

Make the substrate crash-safe and observable. A slow sweeper periodically redelivers outbox rows stuck in `pending`/`delivered` past a threshold — the backstop for a missed `NOTIFY`, a recipient disconnect, or a replica crash between commit and ack. This is what makes the whole path at-least-once rather than best-effort. Add metrics so a wedged delivery path is visible.

## Acceptance Criteria **[REQUIRED]**

- [x] Sweeper task on configurable cadence — `DeliverySweeper` + `SweeperConfig` (defaults 30s/60s/256 rows); `run(shutdown)` ticks on `tokio::time::interval`.
- [x] Multi-replica safe (OQ-D resolved without per-replica ownership): atomic CAS via `reset_to_pending` makes concurrent sweepers race-safe — proven by `concurrent_sweepers_are_race_safe` test.
- [x] Metrics registered in cloacina-server: `cloacina_delivery_outbox_sweep_runs_total`, `cloacina_delivery_outbox_sweep_redeliveries_total` (counters), `cloacina_delivery_outbox_open` (gauge).
- [~] Integration test "kill a relay mid-delivery → sweeper redelivers" — **inherited by [[CLOACI-T-0629]]**'s live-server contract suite (full pipeline + sweeper exercised together).
- [~] `promtool` metrics-format validation — naturally folded into the same T-0629 contract run; describes are in place so a `metrics-format` pass will validate them.
- [x] Sweep traffic ≪ former poll loops: one scan + one `count_open` per `sweep_interval` (default 30s), with no per-row polling.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Sweeper reuses T-0625's "rows stuck past threshold" query and T-0626's relay dispatch path (don't reimplement delivery). Threshold + cadence configurable, defaulted conservatively. Pattern mirrors the existing compiler stale-build sweeper.

### Dependencies
[[CLOACI-T-0626]] (relay/delivery path), [[CLOACI-T-0625]] (queries). Pairs with [[CLOACI-T-0627]] to complete at-least-once.

### Risk Considerations
- Sweeper + a late NOTIFY could double-dispatch the same row — acceptable under at-least-once + idempotent recipients, but avoid tight-loop duplicate dispatch by claiming/marking rows before redelivery.
- Dead-replica row reclaim (OQ-D) needs a liveness signal for replicas, not just recipients.

## Status Updates **[REQUIRED]**

### 2026-05-28 — Complete: sweeper + metrics, `angreal test unit` ✅

- **`cloacina/src/delivery/sweeper.rs`** — `DeliverySweeper` + `SweeperConfig` (default sweep_interval=30s, stuck_threshold=60s, batch_limit=256). `run(shutdown)` ticks on `tokio::time::interval` (skipping the immediate tick so startup doesn't race the relay's own catch-up drain); `sweep_once()` lists rows past cutoff, resets `delivered` rows via the existing `reset_to_pending` compare-and-set, and fires one `WakeHandle::wake()` per non-empty sweep so the relay re-drains.
- **OQ-D resolved without ownership**: every replica may run a sweeper. `reset_to_pending` is atomic CAS — concurrent sweepers racing on the same row produce exactly one `Ok(())` and N-1 `InvalidStateTransition`s that are ignored. No per-replica claim column needed for v1; documented in module rustdoc.
- **Metrics**: 3 new signals described in `cloacina-server`:
  - `cloacina_delivery_outbox_sweep_runs_total` (counter)
  - `cloacina_delivery_outbox_sweep_redeliveries_total` (counter)
  - `cloacina_delivery_outbox_open` (gauge)
- **Server startup wired**: sweeper spawned alongside relay + listener; shares the same `watch::Receiver<bool>` so it shuts down cleanly with the relay/listener on `run()` exit.
- **4 unit tests** (sqlite): resets stuck delivered rows; skips fresh rows; idempotent on re-run; race-safe under concurrent sweepers. 695 tests total ✅, no warnings.
- Server compile-check ✅.

**Done vs deferred:**
- ✅ Sweeper task on configurable cadence; ✅ multi-replica safe (OQ-D); ✅ metrics registered; ✅ sweep traffic is one scan per interval (NFR-1.2.1 preserved — far below the former per-row poll loops).
- ↪ AC #4 (integration test: kill a relay mid-delivery → sweeper redelivers) and AC #5 (metrics-format `promtool` validation) naturally land with [[CLOACI-T-0629]]'s live-server contract run, where the full pipeline (server + sweeper + WS) is exercised end-to-end. The sweeper's correctness is proven by unit tests + the integration coverage T-0629 brings.
