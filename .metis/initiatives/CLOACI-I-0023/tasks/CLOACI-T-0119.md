---
id: foundation-types-detectoroutput
level: task
title: "Foundation types: DetectorOutput, BufferedBoundary, PostgresConnection"
short_code: "CLOACI-T-0119"
created_at: 2026-03-15T11:46:27.586099+00:00
updated_at: 2026-03-15T12:02:26.175063+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# Foundation types: DetectorOutput, BufferedBoundary, PostgresConnection

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement `DetectorOutput` enum (S-0004), `BufferedBoundary` struct (S-0002), and a basic `PostgresConnection` implementing `DataConnection` (S-0003). These complete the foundation types needed by the accumulation and scheduling layers.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DetectorOutput` enum with variants: `Change { boundaries: Vec<ComputationBoundary> }`, `WatermarkAdvance { boundary: ComputationBoundary }`, `Both { boundaries, watermark }`
- [ ] `DetectorOutput` extractable from task output `Context` via well-known key `__detector_output`
- [ ] `BufferedBoundary` struct with `boundary: ComputationBoundary`, `received_at: DateTime<Utc>`
- [ ] `PostgresConnection` struct implementing `DataConnection` trait (host, port, database, schema, table)
- [ ] `PostgresConnection::connect()` returns a `deadpool_diesel` pool handle (or connection string for now)
- [ ] `PostgresConnection::descriptor()` returns `ConnectionDescriptor { system_type: "postgres", location: "host:port/schema.table" }`
- [ ] `PostgresConnection::system_metadata()` returns structured JSON with host, database, schema, table
- [ ] Unit tests for DetectorOutput serialization, BufferedBoundary lag calculation, PostgresConnection descriptor

## Implementation Notes

### Technical Approach
- `DetectorOutput` in `continuous/detector.rs`
- `BufferedBoundary` in `continuous/boundary.rs` alongside `ComputationBoundary`
- `PostgresConnection` in `continuous/connections/postgres.rs`
- Detector workflows are regular Cloacina workflows — the only new piece is the `DetectorOutput` they write to context

### Dependencies
- T-0117 (ComputationBoundary), T-0118 (DataConnection trait)

## Status Updates

- Created `continuous/detector.rs` with `DetectorOutput` enum (Change, WatermarkAdvance, Both)
- `DETECTOR_OUTPUT_KEY` constant, `from_context()` extraction, `boundaries()` and `watermark()` helpers
- Created `continuous/connections/mod.rs` and `connections/postgres.rs`
- `PostgresConnection` implements `DataConnection`, returns connection URL string from `connect()`
- `BufferedBoundary` was already in T-0117 — verified complete
- Fixed global state race in boundary tests by using unique schema names per test (removed `clear_custom_schemas()`)
- 29 total tests passing across all continuous modules (13 boundary + 6 datasource + 5 detector + 5 postgres)
