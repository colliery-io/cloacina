---
id: windowedaccumulator-with
level: task
title: "WindowedAccumulator with WatermarkMode and BoundaryLedger integration"
short_code: "CLOACI-T-0131"
created_at: 2026-03-15T13:14:11.158591+00:00
updated_at: 2026-03-15T13:31:08.859062+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# WindowedAccumulator with WatermarkMode and BoundaryLedger integration

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Implement `WindowedAccumulator` — an accumulator preset that uses source watermarks to determine data completeness before firing. Supports `WatermarkMode::WaitForWatermark` (wait for source confirmation) and `WatermarkMode::BestEffort` (fire when trigger says so).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `WatermarkMode` enum: `WaitForWatermark`, `BestEffort`
- [ ] `WindowedAccumulator` implementing `SignalAccumulator` trait
- [ ] `WindowedAccumulator::new(policy, watermark_mode, boundary_ledger, source_name)`
- [ ] `is_ready()`: checks trigger policy first, then if WaitForWatermark checks `boundary_ledger.covers(source, pending_boundary)`
- [ ] `pending_boundary()` method — coalesces buffered boundaries to determine what range needs coverage
- [ ] BestEffort mode: fires when trigger policy says so regardless of watermark
- [ ] Registration-time validation: warn/log if WindowedAccumulator + WaitForWatermark is used on a source whose detector never emits watermarks
- [ ] Unit tests: WaitForWatermark blocks until watermark covers, BestEffort fires immediately, watermark advance unblocks pending drain

## Implementation Notes

### Technical Approach
- In `continuous/accumulator.rs` alongside `SimpleAccumulator`
- Holds `Arc<RwLock<BoundaryLedger>>` reference and `source_name: String`
- `pending_boundary()` calls `coalesce()` on buffered boundaries without draining
- Shares `drain()` logic with SimpleAccumulator (coalesce + produce context + clear buffer)

### Dependencies
- T-0129 (BoundaryLedger), T-0130 (watermark routing in scheduler)

## Status Updates

- Added `WatermarkMode` enum: `WaitForWatermark`, `BestEffort`
- Implemented `WindowedAccumulator` in `continuous/accumulator.rs`
- Holds `Arc<RwLock<BoundaryLedger>>` and `source_name` for watermark checks
- `is_ready()`: checks trigger policy first, then WaitForWatermark checks `boundary_ledger.covers()`
- `pending_boundary()`: coalesces buffered boundaries without draining
- Shares drain logic pattern with SimpleAccumulator (coalesce + context + clear)
- 6 new tests: best_effort fires, wait blocks without watermark, wait fires when covered, wait blocks when not covered, watermark advance unblocks, drain produces context
- 13 total accumulator tests passing
