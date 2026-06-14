---
id: expose-stale-claim-threshold-stale
level: task
title: "Expose stale_claim_threshold / stale_claim_sweep_interval builder setters (currently fixed at 60s/30s); fix vestigial enable_recovery flag"
short_code: "CLOACI-T-0667"
created_at: 2026-06-12T20:02:24.089072+00:00
updated_at: 2026-06-14T00:34:47.650010+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Expose stale_claim_threshold / stale_claim_sweep_interval builder setters (currently fixed at 60s/30s); fix vestigial enable_recovery flag

## Objective **[REQUIRED]**

Make the stale-claim sweeper's timing tunable through `DefaultRunnerConfigBuilder`.
Today `stale_claim_threshold` (60s) and `stale_claim_sweep_interval` (30s) have
getters and `build()`-time validation but **no builder setters**, so they are
fixed for every consumer. Downstream daemons that want faster crash-recovery
(stale-claim reclaim) cannot tune them and are forced to consider reimplementing
recovery themselves. Add the two setters (mirroring `heartbeat_interval`), and
while here, resolve the vestigial `enable_recovery` flag that currently gates
nothing.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

P2: there is no correctness bug — the sweeper works at its fixed cadence. This is
an ergonomics/operability gap. It becomes P1 for any consumer that needs
sub-60s crash-recovery latency and is blocked from achieving it.

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: Operators tuning a daemon for fast failover can shorten the
  window between a runner crashing and its claimed tasks being reclaimed
  (default 60s). Today that window is hard-coded.
- **Business Value**: Avoids a far more expensive and risky alternative —
  reimplementing crash-recovery/reconciliation at the daemon layer to work
  around fixed cloacina timing.
- **Effort Estimate**: S (two builder methods + optional plumbing).

## Background / Why this ticket exists

Surfaced while evaluating a daemon crash-recovery fix. The proposed workaround
was: *"cloacina 0.6.1 doesn't expose stale_claim_threshold/stale_claim_sweep_interval
setters (fixed 60s/30s) … the robust answer is to own crash-recovery at the daemon
… enable_recovery(false) + a daemon startup reconciliation."*

Verified against the in-repo source (workspace `cloacina` 0.7.0). Findings:

1. **The "no setters" half is correct.** `crates/cloacina/src/runner/default_runner/config.rs`
   has getters `stale_claim_threshold()` (:255) and `stale_claim_sweep_interval()`
   (:250), defaults 60s/30s (:320-321), and `build()` validation that
   `stale_claim_threshold > heartbeat_interval` (:528) — but **no builder
   methods**. Compare `heartbeat_interval(:506)`, which has a setter. So the
   timing is genuinely untunable through the public builder.

2. **The `enable_recovery(false)` half is wrong, on two counts:**
   - The stale-claim sweeper is gated **solely by `enable_claiming()`**
     (`services.rs:93`), not by `enable_recovery`. The only thing
     `cron_enable_recovery` gates is the *cron* recovery service
     (`services.rs:82`). So `enable_recovery(false)` would not stop the sweeper.
   - **`enable_recovery` (the plain flag) is dead code.** It has a getter
     (config.rs:143), a builder setter (:362), and three *test-only* references
     (:805/:943/:959) — and is read **nowhere** in the runner. It gates nothing.
     `enable_recovery(false)` is a complete no-op today.
   - Actually disabling the sweeper requires `enable_claiming(false)`, which also
     disables task claiming entirely (horizontal scaling) and would break the
     monitor's normal claim/retry path. Not a viable "tune the timing" lever.

Conclusion: since cloacina is in-workspace, the robust fix is to expose the two
knobs here and bump the daemon — **not** to reimplement recovery at the daemon.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `DefaultRunnerConfigBuilder` gains `stale_claim_threshold(Duration)` and
      `stale_claim_sweep_interval(Duration)` setters, mirroring `heartbeat_interval`.
- [x] `build()` validation still enforces `stale_claim_threshold > heartbeat_interval`;
      added `test_stale_claim_builder_setters` (rejects too-small threshold; honors a
      valid override end-to-end — the values flow into `StaleClaimSweeperConfig` via
      the unchanged getters in `services.rs`).
- [x] `enable_recovery` decision: **WIRED** (not deleted). The stale-claim sweeper
      gate in `services.rs` is now `enable_claiming() && enable_recovery()`, so the
      flag gates the recovery sweeper as its name implies — `enable_recovery(false)`
      is no longer a silent no-op. Chose wiring over deletion to avoid a
      source-breaking API change (the `cloacina-python` bindings forward it) and to
      give the flag its expected meaning. Default stays `true` → no behavior change
      for existing configs.
- [~] (Optional) Python/CLI plumbing of the two knobs — deferred (no demand stated;
      core Rust builder fix is done; the python bindings already forward
      `enable_recovery`).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Two ~4-line builder methods in
`crates/cloacina/src/runner/default_runner/config.rs` next to `heartbeat_interval`
(:505-509). The values already flow into `StaleClaimSweeperConfig` at
`crates/cloacina/src/runner/default_runner/services.rs:287-290`; no wiring change
needed there. For `enable_recovery`, grep confirms zero non-test readers — prefer
deletion unless a near-term consumer is identified, in which case gate the sweeper
on `enable_claiming() && enable_recovery()`.

### Risk Considerations
Low. Existing `build()` validation prevents a threshold ≤ heartbeat. Deleting
`enable_recovery` is source-breaking for any external caller that sets it — but it
is a no-op today, so behavior is unchanged; note it in the changelog.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented.** Added the two builder setters
(`stale_claim_threshold`, `stale_claim_sweep_interval`) next to
`heartbeat_interval` in `config.rs`; the values already flow into
`StaleClaimSweeperConfig` via the getters, so no `services.rs` plumbing was
needed there. Wired `enable_recovery` into the sweeper gate
(`enable_claiming() && enable_recovery()`) so it's no longer dead. Added
`test_stale_claim_builder_setters`. Optional python/CLI plumbing deferred.

**2026-06-12 — Filed.** Created while reviewing a daemon crash-recovery proposal.
Confirmed the "no builder setters for stale_claim_threshold/sweep_interval" claim
against the in-repo source, and disproved the proposed `enable_recovery(false)`
workaround (sweeper is gated by `enable_claiming()`; `enable_recovery` is dead
code read nowhere in the runner). Recommended fix: add the two setters + resolve
the dead flag, rather than reimplementing recovery at the daemon.
