---
id: computationboundary-data-slice
level: specification
title: "ComputationBoundary - Data Slice Description"
short_code: "CLOACI-S-0002"
created_at: 2026-03-10T18:18:19.267508+00:00
updated_at: 2026-03-10T18:18:19.267508+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# ComputationBoundary - Data Slice Description

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

A serializable message describing what slice of data a signal or execution covers. It is **advisory data, not an enforced contract** — the framework carries it between components, coalesces it when signals pile up, and puts it in context for tasks to read. Tasks are responsible for actually scoping their work to the boundary.

The boundary is a struct with an enum, not a trait. The universe of merge strategies is small (range extension, latest-wins, identity), and the merge logic belongs on the accumulator, not the boundary. The boundary is just data.

## Core Types

```rust
struct ComputationBoundary {
    kind: BoundaryKind,
    metadata: Option<serde_json::Value>,  // domain-specific context that isn't about merging
    emitted_at: DateTime,                 // when the detector created this boundary
}

enum BoundaryKind {
    /// Classic Airflow-style intervals — coalesces via min(start), max(end)
    TimeRange { start: DateTime, end: DateTime },
    /// Kafka-style partition offsets — coalesces via min(start), max(end)
    OffsetRange { start: i64, end: i64 },
    /// Opaque "resume from here" token — coalesces via latest-wins
    Cursor { value: String },
    /// Entire dataset is the unit of change — coalesces via latest-wins
    /// Value is a user-provided state identifier (hash, version counter, commit SHA, etc.)
    FullState { value: String },
    /// User-defined boundary type with schema-validated payload
    Custom { kind: String, value: serde_json::Value },
}
```

## Coalescing Rules

Built-in coalesce rules implemented by the framework on the four known variants:

| Variant | Coalesce strategy |
|---|---|
| `TimeRange` | `min(starts)..max(ends)` |
| `OffsetRange` | `min(starts)..max(ends)` |
| `Cursor` | Latest wins (by signal timestamp) |
| `FullState` | Latest wins (by signal timestamp) — also enables dedup on identical values |
| `Custom` | Delegated to the accumulator (framework does not merge custom boundaries) |

For `Custom` boundaries, the accumulator owns the merge logic. If no custom merge is provided, the accumulator holds all signals unmerged and lets `drain()` decide how to present them.

Boundaries compose naturally through coalescing:
- `TimeRange [14:00, 15:00) + [15:00, 16:00)` → `TimeRange [14:00, 16:00)`
- 24 hourly `TimeRange` signals → `TimeRange [00:00, 24:00)` for a daily rollup
- `FullState("v7") + FullState("v8")` → `FullState("v8")` (latest wins)

## Custom Boundary Schema Enforcement

`Custom` boundaries require a registered schema definition that validates the `value` payload. This prevents malformed data from propagating through the graph and ensures that producers and consumers of a custom boundary type agree on its structure.

```rust
struct CustomBoundarySchema {
    /// Unique type name (must match `Custom { kind }`)
    kind: String,
    /// JSON Schema defining the expected shape of `value`
    schema: serde_json::Value,
}
```

Custom boundary types are registered at startup alongside their schemas:

```rust
register_custom_boundary("sequence_range", json!({
    "type": "object",
    "required": ["table", "min_id", "max_id"],
    "properties": {
        "table": { "type": "string" },
        "min_id": { "type": "integer" },
        "max_id": { "type": "integer" }
    }
}));
```

The framework validates `Custom` boundary payloads against their registered schema at two points:
- **Signal ingestion** — when a detector emits a signal with a `Custom` boundary, the payload is validated before the accumulator receives it. Invalid payloads are rejected with a descriptive error.
- **Accumulator drain** — when `drain()` produces a `Custom` boundary in the output context, the payload is re-validated to catch corruption from custom merge logic.

Unregistered custom boundary kinds are rejected at signal ingestion — the framework will not carry a `Custom` boundary it has no schema for.

## Backpressure Observability

The accumulator (CLOACI-S-0005) timestamps each boundary on receipt, stored internally as:

```rust
struct BufferedBoundary {
    boundary: ComputationBoundary,
    received_at: DateTime,             // when the accumulator received this
}
```

`received_at - boundary.emitted_at` = ingestion lag per boundary. If this delta is growing, the system is falling behind. This provides backpressure measurement without an explicit backpressure protocol — it's simply observable from the accumulator's buffer.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Struct+enum, not trait | Universe of merge strategies is small. Merge logic belongs on the accumulator, not the boundary. |
| `FullState` requires `value: String` | "The whole thing changed" is useless without identifying what state it changed to. Hash, version counter, commit SHA — semantic but inherently useful. |
| `emitted_at` on boundary | Paired with `received_at` on accumulator gives backpressure measurement for free. |
| Custom requires JSON Schema | Prevents malformed data propagation. Producers and consumers must agree on structure. |
| Boundary is advisory | Framework carries it; tasks are responsible for actually scoping work. No enforcement. |
