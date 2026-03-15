---
id: route-watermarkadvance-through
level: task
title: "Route WatermarkAdvance through ContinuousScheduler to BoundaryLedger"
short_code: "CLOACI-T-0130"
created_at: 2026-03-15T13:14:09.710991+00:00
updated_at: 2026-03-15T13:28:57.717194+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# Route WatermarkAdvance through ContinuousScheduler to BoundaryLedger

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Extend the `ContinuousScheduler` run loop to handle `DetectorOutput::WatermarkAdvance` and `DetectorOutput::Both` by routing watermark advances to the `BoundaryLedger`. Currently the scheduler only processes `Change` boundaries.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ContinuousScheduler` holds `Arc<RwLock<BoundaryLedger>>` field
- [ ] Run loop step 3a: extract `WatermarkAdvance` from `DetectorOutput`, call `boundary_ledger.advance(source, watermark)`
- [ ] Run loop step 3a: extract watermark from `DetectorOutput::Both`, advance ledger before processing change boundaries
- [ ] Match detector task name back to data source name (use `DataSource.detector_workflow` mapping)
- [ ] Log watermark advances at debug level
- [ ] Unit test: detector emits WatermarkAdvance → ledger updated
- [ ] Unit test: detector emits Both → ledger updated AND boundaries routed to accumulators

## Implementation Notes

### Technical Approach
- Add `boundary_ledger` field to `ContinuousScheduler` struct
- Build a reverse lookup map: `detector_workflow_name → data_source_name` at construction
- In `process_detector_output()`, handle all three DetectorOutput variants
- Pass `BoundaryLedger` through constructor

### Dependencies
- T-0129 (BoundaryLedger)

## Status Updates

- Added `boundary_ledger: Arc<RwLock<BoundaryLedger>>` and `detector_to_source: HashMap` to ContinuousScheduler
- Constructor builds reverse lookup map from `DataSource.detector_workflow` → source name
- `process_detector_output()` now handles all 3 DetectorOutput variants:
  - WatermarkAdvance → advances boundary ledger
  - Both → advances ledger first, then routes boundaries
  - Change → routes boundaries (unchanged)
- Added `boundary_ledger()` accessor for WindowedAccumulator integration
- Uses detector_to_source map for targeted routing (falls back to broadcast for unmatched detectors)
- 5 passing tests: original 3 + watermark advance test + Both output test
