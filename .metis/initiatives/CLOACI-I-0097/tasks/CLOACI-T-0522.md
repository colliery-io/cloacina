---
id: t4-heartbeat-writer-stale-build
level: task
title: "T4: Heartbeat writer + stale-build sweeper"
short_code: "CLOACI-T-0522"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-19T00:23:10.373168+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0521]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T4: Heartbeat writer + stale-build sweeper

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Keep `build_claimed_at` fresh during long builds so the sweeper doesn't reset them, and reset genuinely stuck rows that a dead compiler left behind.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Heartbeat task: spawned at the start of each build, cancelled when the build finishes (success or failure). Fires every `compiler.heartbeat_interval_s` (default 10) and calls `dal.heartbeat_build(id)`.
- [ ] Sweeper task: runs on the compiler's main tokio runtime every `compiler.sweep_interval_s` (default 30). Calls `dal.sweep_stale_builds(stale_threshold_s)` (default 60).
- [ ] Heartbeat task uses a `CancellationToken` (or `tokio::sync::broadcast::Receiver`) so the final `mark_success`/`mark_failed` never races against a heartbeat UPDATE.
- [ ] Sweeper emits a tracing event per row reset, including `package_id` and the staleness duration.
- [ ] Config knobs honored: flag overrides > config file > defaults.
- [ ] Integration test: simulate a stuck build by calling `claim_next_build` then waiting past the stale threshold without heartbeating; assert the sweeper resets it to `pending` with a "reset after stale heartbeat" error message.
- [ ] Integration test: run a build with a heartbeat interval short enough to fire multiple times; assert `build_claimed_at` advances across heartbeats.

## Implementation Notes

### Task structure

```rust
async fn run_build_with_heartbeat(id: Uuid, dal: Dal, config: &Config) -> Result<()> {
    let cancel = CancellationToken::new();
    let hb_cancel = cancel.clone();
    let hb_dal = dal.clone();
    let hb_interval = config.heartbeat_interval;
    let hb = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(hb_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = hb_cancel.cancelled() => return,
                _ = ticker.tick() => {
                    if let Err(e) = hb_dal.heartbeat_build(id).await {
                        tracing::warn!(%e, "heartbeat update failed");
                    }
                }
            }
        }
    });

    let result = execute_build(id, &dal).await;  // T3 logic
    cancel.cancel();
    let _ = hb.await;
    result
}
```

### Sweep race safety

Sweeper's UPDATE filters on `build_claimed_at < NOW() - threshold`; a row whose heartbeat just fired before the sweep runs will not match. The window is bounded by heartbeat clock skew between compiler instances. Threshold of 6× heartbeat leaves plenty of headroom.

### Sweep running outside compiler too

T1 exposes `sweep_stale_builds` as a DAL helper — the server could call it too if desired. For v1 we only run it on the compiler.

## Status Updates

*To be added during implementation*
