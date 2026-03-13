---
id: datasource-external-dataset-handle
level: specification
title: "DataSource - External Dataset Handle"
short_code: "CLOACI-S-0003"
created_at: 2026-03-10T18:18:20.584684+00:00
updated_at: 2026-03-10T18:18:20.584684+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# DataSource - External Dataset Handle

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

A named handle to an external dataset. The `DataSource` is more than a name — it carries a connection implementation, change detection, and metadata for lineage tracking. This makes connection information shareable across detectors and tasks rather than reimplemented in each callback.

## Core Types

```rust
struct DataSource {
    name: String,
    connection: Box<dyn DataConnection>,
    detector_workflow: String,    // workflow name — must produce DetectorOutput in output context
    lineage: DataSourceMetadata,
}

struct DataSourceMetadata {
    description: Option<String>,
    owner: Option<String>,
    tags: Vec<String>,
}
```

## DataConnection (trait)

External data systems have structurally different metadata (e.g., `database.schema.table` vs `topic.partition.consumer_group`). The `DataConnection` trait provides typed access for system-specific concerns and a generic descriptor for framework-level lineage.

```rust
trait DataConnection: Send + Sync {
    /// Connect/get a usable handle to the data source
    fn connect(&self) -> Result<Box<dyn Any>>;

    /// Generic lineage descriptor — enough for framework-level graphs (no secrets)
    fn descriptor(&self) -> ConnectionDescriptor;

    /// System-specific metadata as structured Value for detailed lineage
    fn system_metadata(&self) -> serde_json::Value;
}

struct ConnectionDescriptor {
    system_type: String,     // "postgres", "kafka", "s3", "http"
    location: String,        // human-readable canonical identifier
}
```

The framework reads `descriptor()` for generic lineage graphs ("this is a postgres source at this location"). Consumers needing typed detail read `system_metadata()` — since it's `Value`, it can be stored, queried, and displayed without the framework knowing the schema.

## Framework-Provided Implementations

```rust
// Example: PostgresConnection
struct PostgresConnection { host, port, database, schema, table, ... }
impl DataConnection for PostgresConnection {
    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "postgres".into(),
            location: format!("{}:{}/{}.{}", self.host, self.port, self.schema, self.table),
        }
    }
    fn system_metadata(&self) -> Value {
        json!({ "host": self.host, "database": self.database,
                "schema": self.schema, "table": self.table })
    }
}

// Example: KafkaConnection
struct KafkaConnection { brokers, topic, partition, consumer_group, ... }
impl DataConnection for KafkaConnection {
    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "kafka".into(),
            location: format!("{}/{}", self.brokers.join(","), self.topic),
        }
    }
    fn system_metadata(&self) -> Value {
        json!({ "brokers": self.brokers, "topic": self.topic,
                "partition": self.partition, "consumer_group": self.consumer_group })
    }
}
```

Users implement the trait for internal/custom systems.

## Lineage

Lineage is derived from two sources:
- **Accumulator edges**: `DataSource` → `Accumulator` → `Task` (reads from)
- **LedgerTrigger observation**: `Task` → (observed by) → `DataSource` (writes to, inferred)

Together with `ConnectionDescriptor` on each data source, this provides a complete lineage map without tasks explicitly declaring their outputs. The `system_metadata()` method provides drill-down detail without the framework needing to understand each system's structure.

## Tasks Are Pure Compute

Tasks in the continuous graph do not declare inputs or outputs. The graph wiring (which data sources connect to which tasks via accumulators) is configured separately. Tasks receive their input data sources injected at execution time:

```rust
#[continuous_task]
async fn aggregate_hourly(
    ctx: &mut Context,
    inputs: &DataSourceMap,     // name → &DataSource, can call .connection.connect()
) -> Result<()> {
    let boundary = ctx.get("__boundary")?;  // from accumulator's drain()
    let conn = inputs.get("raw_events").connection.connect()?;
    // query within the boundary, write results
    // task doesn't know or care about downstream consumers
}
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| DataConnection as trait, not enum | External systems are structurally different — can't enumerate them all |
| Generic `descriptor()` + typed `system_metadata()` | Framework needs generic lineage; consumers need typed detail. Both without the framework knowing every schema. |
| `detector_workflow` is a workflow name string | Detectors are workflows (CLOACI-S-0004), not framework components. Loose coupling via name. |
| Tasks don't declare inputs/outputs | Graph wiring determines data flow. Tasks are pure compute receiving injected DataSourceMap. |
| `connect()` returns `Box<dyn Any>` | Generic `DataConnection<C>` can't be stored in a heterogeneous graph. `DataSourceMap` provides a typed `connection<T>()` helper that downcasts with a clear `GraphError::ConnectionTypeMismatch` on wiring bugs. Tasks always know their concrete connection type; mismatches are caught on first run. |

## DataSourceMap Typed Access

`DataSourceMap` provides an ergonomic typed accessor so tasks don't manually downcast:

```rust
impl DataSourceMap {
    /// Get a typed connection handle, with a clear error on wiring mismatch.
    fn connection<T: 'static>(&self, name: &str) -> Result<&T, GraphError> {
        let source = self.get(name)
            .ok_or(GraphError::SourceNotFound(name.into()))?;
        let handle = source.connection.connect()?;
        handle.downcast_ref::<T>()
            .ok_or(GraphError::ConnectionTypeMismatch {
                source: name.into(),
                expected: std::any::type_name::<T>().into(),
            })
    }
}
```

Task usage:

```rust
let pool = inputs.connection::<PgPool>("raw_events")?;
```
