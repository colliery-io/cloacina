---
id: python-cloaca-continuous-task
level: initiative
title: "Python/Cloaca Continuous Task Support"
short_code: "CLOACI-I-0026"
created_at: 2026-03-13T02:44:41.234817+00:00
updated_at: 2026-03-13T02:44:41.234817+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: python-cloaca-continuous-task
---

# Python/Cloaca Continuous Task Support

## Context

CLOACI-I-0023 through I-0025 deliver continuous reactive scheduling for Rust. Cloacina has Python bindings via Cloaca (CLOACI-I-0020), and the S-0001 decisions log confirms Python/Cloaca support for the continuous graph as a second-phase goal. This initiative extends the continuous scheduling system to Python developers.

**Depends on**: CLOACI-I-0023 (core continuous scheduling), stable Cloaca bindings
**Specification**: CLOACI-S-0001 (decisions log), S-0003 (DataConnection)

## Goals & Non-Goals

**Goals:**
- Python `@continuous_task` decorator equivalent to the Rust `#[continuous_task]` macro
- Python detector workflows that produce `DetectorOutput` in their output context
- Python `DataConnection` protocol class for user-defined data sources
- `DataSourceMap` bridge to Python — typed access to connections from Python task functions
- Framework-provided Python connection classes (Postgres, Kafka, S3) wrapping Rust impls
- Python continuous tasks and Rust continuous tasks coexisting in the same graph
- End-to-end example: Python continuous task reacting to data changes

**Non-Goals:**
- Python-native accumulator or trigger policy implementations (Rust only — Python tasks consume, they don't customize scheduling)
- Python graph assembly API (graph is assembled from registrations on the Rust side; Python tasks register via decorators)
- Pure-Python scheduler (the ContinuousScheduler is always Rust)

## Detailed Design

### `@continuous_task` Decorator

```python
from cloaca import continuous_task, DataSourceMap, Context

@continuous_task(
    id="aggregate_hourly",
    sources=["raw_events"],
    referenced=["config_table"],
)
async def aggregate_hourly(ctx: Context, inputs: DataSourceMap) -> None:
    conn = inputs.connection("raw_events")  # returns a Python connection object
    boundary = ctx.get("__boundary")
    # ... query within boundary, write results ...
```

The decorator generates registration metadata that the Rust runtime consumes during graph assembly. Python continuous tasks participate in the same `DataSourceGraph` as Rust tasks.

### Python DataConnection Protocol

```python
from typing import Protocol, Any

class DataConnection(Protocol):
    def connect(self) -> Any:
        """Return a usable connection handle."""
        ...

    def descriptor(self) -> ConnectionDescriptor:
        """Generic lineage descriptor."""
        ...

    def system_metadata(self) -> dict:
        """System-specific metadata as dict."""
        ...
```

Users implement this protocol for custom data sources. The framework provides convenience classes wrapping the Rust implementations:

```python
from cloaca.connections import PostgresConnection, KafkaConnection

pg = PostgresConnection(host="localhost", database="analytics", schema="public", table="events")
```

### DataSourceMap Bridge

The Rust `DataSourceMap` is exposed to Python via PyO3. The `connection()` method returns a Python object — either a framework-provided wrapper (e.g., `PostgresConnection` exposes a connection pool) or a user's custom `DataConnection` implementation.

No typed generic helper in Python (no generics) — `connection()` returns the concrete Python object directly. Type checking is the user's responsibility (or use `isinstance` checks).

### Mixed Rust/Python Graphs

A continuous graph can contain both Rust and Python tasks. The `ContinuousScheduler` doesn't care — it submits work to the `TaskScheduler`/`Dispatcher`, which routes to the appropriate executor (Rust native or Python via Cloaca's `PythonTaskExecutor`).

Data sources can be shared between Rust and Python tasks. A Rust-defined `PostgresConnection` is accessible from Python via the bridge, and vice versa.

## Alternatives Considered

- **Python-native accumulators**: Rejected — accumulators are scheduling infrastructure that runs in the hot path. Python overhead is unnecessary; customization should happen at the Rust level. Python tasks consume boundaries, they don't control scheduling.
- **Separate Python graph**: Rejected — mixed graphs are more valuable. A Python ML task can react to a Rust ETL task's output in the same graph.
- **Python-first continuous scheduling**: Rejected — Rust-first ensures performance. Python is a consumer, not the foundation.

## Implementation Plan

### Phase 1: Core Python Bindings
- [ ] `ComputationBoundary` and `BoundaryKind` exposed to Python via PyO3
- [ ] `DetectorOutput` Python class with serialization to context
- [ ] `DataSourceMap` Python wrapper with `connection()` method
- [ ] `ConnectionDescriptor` Python class

### Phase 2: `@continuous_task` Decorator
- [ ] Decorator implementation with `sources` and `referenced` attributes
- [ ] Registration metadata generation for Rust graph assembly
- [ ] Integration with Cloaca's task discovery and executor
- [ ] `DataSourceMap` injection into Python task execution path

### Phase 3: Python DataConnection
- [ ] `DataConnection` protocol class definition
- [ ] `PostgresConnection` Python wrapper (wrapping Rust impl)
- [ ] `KafkaConnection` and `S3Connection` Python wrappers
- [ ] User-defined DataConnection support via protocol

### Phase 4: Integration & Example
- [ ] Mixed Rust/Python continuous graph integration test
- [ ] Python detector workflow producing DetectorOutput
- [ ] End-to-end example: Python continuous task in a reactive graph
- [ ] Documentation for Python continuous scheduling usage
- [ ] Cloaca tutorial for continuous scheduling
