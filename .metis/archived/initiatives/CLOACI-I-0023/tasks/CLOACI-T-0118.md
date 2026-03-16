---
id: foundation-types-datasource
level: task
title: "Foundation types: DataSource, DataConnection trait, DataSourceMap"
short_code: "CLOACI-T-0118"
created_at: 2026-03-15T11:46:26.242192+00:00
updated_at: 2026-03-15T11:59:52.376476+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# Foundation types: DataSource, DataConnection trait, DataSourceMap

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement `DataSource`, `DataConnection` trait, `ConnectionDescriptor`, `DataSourceMetadata`, and `DataSourceMap` with typed `connection<T>()` helper as specified in CLOACI-S-0003. These types represent external datasets and provide tasks with typed access to data source connections.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DataConnection` trait: `connect() -> Result<Box<dyn Any>>`, `descriptor() -> ConnectionDescriptor`, `system_metadata() -> Value`
- [ ] `ConnectionDescriptor` struct with `system_type: String`, `location: String`
- [ ] `DataSourceMetadata` struct with `description`, `owner`, `tags`
- [ ] `DataSource` struct with `name`, `connection: Box<dyn DataConnection>`, `detector_workflow: String`, `lineage: DataSourceMetadata`
- [ ] `DataSourceMap` with `get(name)`, typed `connection<T>(name) -> Result<&T, GraphError>`
- [ ] `GraphError::SourceNotFound` and `GraphError::ConnectionTypeMismatch` error variants
- [ ] Unit tests: DataSourceMap typed access, type mismatch error, missing source error
- [ ] Types in `crates/cloacina/src/continuous/datasource.rs`

## Implementation Notes

### Technical Approach
- `DataConnection` is `Send + Sync` (stored in heterogeneous graph)
- `connect()` returns `Box<dyn Any>` — typed access via `DataSourceMap::connection<T>()` which downcasts
- `DataSourceMap` internally holds `HashMap<String, DataSource>` — tasks receive `&DataSourceMap` at execution
- `GraphError` may be a new error type or extend existing `error.rs`

### Key Design Constraints (from S-0003)
- Tasks are pure compute — they don't declare inputs/outputs
- `detector_workflow` is a workflow name string (loose coupling)
- `connect()` returns `Box<dyn Any>` because generic `DataConnection<C>` can't be stored heterogeneously

### Dependencies
- T-0117 (ComputationBoundary — shared types module)

## Status Updates

- Created `continuous/datasource.rs` with all types
- `DataConnection` trait (Send + Sync), `ConnectionDescriptor`, `DataSourceMetadata`, `DataSource`, `DataSourceMap`
- `GraphError` with `SourceNotFound`, `ConnectionTypeMismatch`, `ConnectionError` variants
- Renamed `source` field to `source_name` in `ConnectionTypeMismatch` to avoid thiserror `#[source]` conflict
- `DataSourceMap::connection<T>()` connects and downcasts with clear error messages
- 6 passing tests: typed access, type mismatch, missing source, multiple sources, descriptor, debug
