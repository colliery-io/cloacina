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
estimated_complexity: XL
initiative_id: python-cloaca-continuous-task
---

# Python/Cloaca Continuous Task Support

## Context

CLOACI-I-0023 through I-0025 deliver continuous reactive scheduling for Rust. Cloacina has Python bindings via Cloaca (CLOACI-I-0020), and the S-0001 decisions log confirms Python/Cloaca support for the continuous graph as a second-phase goal. This initiative extends the continuous scheduling system to Python developers.

**Depends on**: CLOACI-I-0023 (core continuous scheduling), stable Cloaca bindings
**Specification**: CLOACI-S-0001 (decisions log), S-0003 (DataConnection)

## Goals & Non-Goals

**Goals:**
- Full Python API for constructing, configuring, and running continuous scheduling graphs
- Python `@continuous_task` decorator equivalent to the Rust `#[continuous_task]` macro
- Python detector workflows that produce `DetectorOutput` in their output context
- Python `DataConnection` protocol class for user-defined data sources
- `DataSourceMap` bridge to Python — typed access to connections from Python task functions
- Framework-provided Python connection classes (Postgres, Kafka, S3) wrapping Rust impls
- All 6 trigger policies exposed as configurable Python classes with composition (Any/All)
- Accumulator selection (Simple vs Windowed) and configuration from Python
- Watermark mode configuration and BoundaryLedger read access
- Late arrival policy configuration per edge
- Full scheduler lifecycle from Python: construction, config, startup restore, run, shutdown
- Crash recovery configuration: DAL wiring, detector state store access, `__last_known_state` convention
- Observability: `graph_metrics()`, `AccumulatorMetrics`, `EdgeMetrics` from Python
- Custom boundary type registration with JSON Schema from Python dicts
- LedgerTrigger configuration for derived data source feedback loops
- Mixed Rust/Python graphs — Python and Rust tasks coexisting in the same graph
- End-to-end examples: Python detector + continuous task, derived data source, watermark usage
- Integration with existing `DefaultRunner` or standalone scheduler management

**Non-Goals:**
- Python-native accumulator implementations (Rust only — Python configures, Rust executes)
- Python-native trigger policy implementations (use composition of framework policies)
- Pure-Python scheduler loop (ContinuousScheduler is always Rust, Python manages lifecycle)
- Python-side lock management or buffer internals

## Detailed Design

### Layer 1: Boundary and Detector Types (Data Plane)

These are the types Python tasks read from and write to context. Pure data — no scheduling logic.

```python
from cloaca.continuous import (
    ComputationBoundary, BoundaryKind, DetectorOutput,
    DETECTOR_OUTPUT_KEY, register_custom_boundary,
)

# Create a boundary
boundary = ComputationBoundary(
    kind=BoundaryKind.offset_range(start=0, end=100),
    metadata={"row_count": 100},
)

# Detector writes output to context
output = DetectorOutput.change(boundaries=[boundary])
ctx[DETECTOR_OUTPUT_KEY] = output

# Or with watermark
output = DetectorOutput.both(
    boundaries=[boundary],
    watermark=ComputationBoundary(kind=BoundaryKind.offset_range(0, 500)),
)

# Detector persists state for crash recovery
ctx["__last_known_state"] = {"cursor": 100}

# Custom boundary registration
register_custom_boundary("sequence_range", {
    "type": "object",
    "required": ["table", "min_id", "max_id"],
    "properties": {
        "table": {"type": "string"},
        "min_id": {"type": "integer"},
        "max_id": {"type": "integer"},
    }
})
```

**PyO3 bindings needed:**
- `ComputationBoundary` (#[pyclass]) — kind, metadata, emitted_at
- `BoundaryKind` — static factory methods: `offset_range()`, `time_range()`, `cursor()`, `full_state()`, `custom()`
- `DetectorOutput` (#[pyclass]) — static factories: `change()`, `watermark_advance()`, `both()`
- `DETECTOR_OUTPUT_KEY` constant
- `register_custom_boundary(kind, schema_dict)` (#[pyfunction])
- `validate_boundary(boundary)` (#[pyfunction])

### Layer 2: Data Sources and Connections

```python
from cloaca.continuous import (
    DataSource, DataSourceMetadata, ConnectionDescriptor,
    PostgresConnection, KafkaConnection, S3Connection,
)

# Framework-provided connections
pg = PostgresConnection("localhost", 5432, "analytics", "public", "events") \
    .with_username("app_user") \
    .with_max_connections(20)

kafka = KafkaConnection(brokers=["kafka:9092"], topic="events", partition=0)

s3 = S3Connection(bucket="data-lake", prefix="raw/events/", region="us-east-1")

# Register data source
source = DataSource(
    name="raw_events",
    connection=pg,
    detector_workflow="detect_raw_events",
    lineage=DataSourceMetadata(
        description="Raw event stream",
        owner="data-platform",
        tags=["events", "raw"],
    ),
)

# Custom Python DataConnection
class MyApiConnection:
    def connect(self):
        return {"url": "https://api.example.com/v2"}

    def descriptor(self):
        return ConnectionDescriptor(system_type="http", location="api.example.com")

    def system_metadata(self):
        return {"url": "https://api.example.com/v2", "version": "v2"}
```

**PyO3 bindings needed:**
- `DataSource` (#[pyclass]) — name, connection, detector_workflow, lineage
- `DataSourceMetadata` (#[pyclass]) — description, owner, tags
- `ConnectionDescriptor` (#[pyclass]) — system_type, location
- `PostgresConnection` (#[pyclass]) — with builder methods
- `KafkaConnection` (#[pyclass])
- `S3Connection` (#[pyclass])
- `DataConnection` protocol support — Python objects implementing connect/descriptor/system_metadata
- `DataSourceMap` (#[pyclass]) — .get(name), .connection(name), .names()

### Layer 3: Graph Construction and Configuration

```python
from cloaca.continuous import (
    assemble_graph, ContinuousTaskRegistration,
    JoinMode, LateArrivalPolicy,
)

# Declare task topology
task_a = ContinuousTaskRegistration(
    id="aggregate_hourly",
    sources=["raw_events"],
    referenced=["config_table"],
)

task_b = ContinuousTaskRegistration(
    id="build_index",
    sources=["raw_events", "config_table"],
    referenced=[],
)

# Assemble graph (validates cycles, unknown sources)
graph = assemble_graph(
    data_sources=[events_source, config_source],
    task_registrations=[task_a, task_b],
)

# Post-assembly configuration: late arrival policies
for edge in graph.edges:
    if edge.source == "raw_events":
        edge.late_arrival_policy = LateArrivalPolicy.RETRIGGER
    else:
        edge.late_arrival_policy = LateArrivalPolicy.DISCARD

# JoinMode is set per-task in the graph
# (defaults to Any — fire when any source has data)
```

**PyO3 bindings needed:**
- `assemble_graph(sources, registrations)` (#[pyfunction]) → DataSourceGraph or raises
- `ContinuousTaskRegistration` (#[pyclass]) — id, sources, referenced
- `DataSourceGraph` (#[pyclass]) — edges, tasks, data_sources, edges_for_task(), edges_for_source()
- `GraphEdge` (#[pyclass]) — source, task, late_arrival_policy (read/write)
- `JoinMode` (#[pyclass]) — Any, All
- `LateArrivalPolicy` (#[pyclass]) — Discard, AccumulateForward, Retrigger
- `GraphAssemblyError` mapped to Python ValueError

### Layer 4: Trigger Policies

```python
from cloaca.continuous import (
    Immediate, BoundaryCount, WallClockWindow, WallClockDebounce,
    AnyPolicy, AllPolicy,
)
from datetime import timedelta

# Simple: fire on every boundary
policy = Immediate()

# Fire every 100 boundaries
policy = BoundaryCount(100)

# Fire every 5 minutes
policy = WallClockWindow(timedelta(minutes=5))

# Fire when burst is over (30s silence)
policy = WallClockDebounce(timedelta(seconds=30))

# Compose: fire every 5 minutes OR 100 boundaries (whichever first)
policy = AnyPolicy([
    WallClockWindow(timedelta(minutes=5)),
    BoundaryCount(100),
])

# Compose: fire after 1000 boundaries AND at least 1 minute
policy = AllPolicy([
    BoundaryCount(1000),
    WallClockWindow(timedelta(minutes=1)),
])
```

**PyO3 bindings needed:**
- `Immediate` (#[pyclass])
- `BoundaryCount` (#[pyclass]) — count
- `WallClockWindow` (#[pyclass]) — duration (accepts timedelta)
- `WallClockDebounce` (#[pyclass]) — duration (accepts timedelta)
- `AnyPolicy` (#[pyclass]) — list of policies
- `AllPolicy` (#[pyclass]) — list of policies
- Duration conversion: Python `timedelta` → Rust `std::time::Duration`

### Layer 5: Accumulators and Watermarks

```python
from cloaca.continuous import (
    SimpleAccumulator, WindowedAccumulator, WatermarkMode,
)

# Simple: fire based on policy alone (no watermark awareness)
acc = SimpleAccumulator(policy=BoundaryCount(50))
acc = SimpleAccumulator(policy=Immediate(), max_buffer_size=5000)

# Windowed: wait for source watermark before firing
acc = WindowedAccumulator(
    policy=WallClockWindow(timedelta(minutes=10)),
    watermark_mode=WatermarkMode.WAIT_FOR_WATERMARK,
    boundary_ledger=scheduler.boundary_ledger(),  # shared reference
    source_name="raw_events",
    max_buffer_size=10000,
)

# Assign to graph edge after assembly
graph.edges[0].set_accumulator(acc)
```

**PyO3 bindings needed:**
- `SimpleAccumulator` (#[pyclass]) — policy, max_buffer_size
- `WindowedAccumulator` (#[pyclass]) — policy, watermark_mode, boundary_ledger, source_name, max_buffer_size
- `WatermarkMode` (#[pyclass]) — WaitForWatermark, BestEffort
- `GraphEdge.set_accumulator(acc)` — replace default accumulator
- `ContinuousScheduler.boundary_ledger()` — returns shared reference for WindowedAccumulator

### Layer 6: Scheduler Lifecycle

```python
from cloaca.continuous import (
    ContinuousScheduler, ContinuousSchedulerConfig, ExecutionLedger,
)
from datetime import timedelta

# Configure
config = ContinuousSchedulerConfig(
    poll_interval=timedelta(milliseconds=100),
    max_fired_tasks=10000,
    task_timeout=timedelta(minutes=5),
)

# Create
ledger = ExecutionLedger()  # or ExecutionLedger(max_events=50000)
scheduler = ContinuousScheduler(graph, ledger, config)

# Optional: enable persistence
scheduler.with_dal(dal)

# Register task implementations
scheduler.register_task(my_aggregate_task)
scheduler.register_task(my_index_task)

# Startup restore (order matters)
await scheduler.init_drain_cursors()
await scheduler.restore_pending_boundaries()
await scheduler.restore_from_persisted_state()
await scheduler.restore_detector_state()

# Run (blocks until shutdown)
fired_tasks = await scheduler.run(shutdown_event)

# Results
for task in fired_tasks:
    print(f"{task.task_id}: executed={task.executed}, error={task.error}")
```

**PyO3 bindings needed:**
- `ContinuousSchedulerConfig` (#[pyclass]) — poll_interval, max_fired_tasks, task_timeout (all accept timedelta)
- `ContinuousScheduler` (#[pyclass]) — new(), with_dal(), register_task(), run(), graph_metrics(), boundary_ledger(), detector_state_store()
- `ExecutionLedger` (#[pyclass]) — new(), with_config()
- `LedgerConfig` (#[pyclass]) — max_events
- `FiredTask` (#[pyclass]) — task_id, fired_at, executed, error
- Async: `run()`, `init_drain_cursors()`, `restore_*()` must be Python async (pyo3-asyncio or spawn_blocking)

### Layer 7: Derived Data Sources (LedgerTrigger)

```python
from cloaca.continuous import LedgerTrigger, LedgerMatchMode

# Fire detect_derived when aggregate_hourly completes
trigger = LedgerTrigger(
    trigger_name="detect_derived_data",
    watch_tasks=["aggregate_hourly"],
    match_mode=LedgerMatchMode.ANY,
    ledger=ledger,  # shared ExecutionLedger
)

# For multi-dependency: fire when ALL upstream tasks complete
trigger = LedgerTrigger(
    trigger_name="detect_joined_data",
    watch_tasks=["task_a", "task_b"],
    match_mode=LedgerMatchMode.ALL,
    ledger=ledger,
)

# Register with trigger scheduler (existing cloaca trigger API)
runner.register_trigger(trigger, "detect_derived_data")
```

**PyO3 bindings needed:**
- `LedgerTrigger` (#[pyclass]) — constructor, implements Trigger trait
- `LedgerMatchMode` (#[pyclass]) — Any, All

### Layer 8: Observability and Detector State

```python
# Monitor accumulator metrics
metrics = scheduler.graph_metrics()
for m in metrics:
    print(f"{m.source} → {m.task}")
    print(f"  buffered: {m.accumulator.buffered_count}")
    print(f"  max_lag: {m.accumulator.max_lag}")
    print(f"  total_received: {m.accumulator.total_boundaries_received}")
    print(f"  drain_count: {m.accumulator.drain_count}")

# Read detector state (from Python detector on startup)
store = scheduler.detector_state_store()
last_state = store.get_committed("raw_events")
if last_state:
    last_cursor = last_state["cursor"]
    # Resume polling from last_cursor
```

**PyO3 bindings needed:**
- `AccumulatorMetrics` (#[pyclass]) — all fields as read-only properties
- `EdgeMetrics` (#[pyclass]) — source, task, accumulator
- `DetectorStateStore` (#[pyclass]) — get_committed(), get_latest()
- `ContinuousScheduler.graph_metrics()` → list of EdgeMetrics
- `ContinuousScheduler.detector_state_store()` → DetectorStateStore

### Layer 9: `@continuous_task` Decorator

```python
from cloaca import continuous_task

@continuous_task(
    id="aggregate_hourly",
    sources=["raw_events"],
    referenced=["config_table"],
)
async def aggregate_hourly(ctx, inputs):
    boundary = ctx.get("__boundary")
    conn = inputs.connection("raw_events")
    # ... process data within boundary range ...

# Decorator generates ContinuousTaskRegistration metadata
# and wraps function in PythonContinuousTaskWrapper
```

The decorator:
1. Stores `sources` and `referenced` as metadata on the function
2. Wraps the function in a `PythonContinuousTaskWrapper` (extends existing `PythonTaskWrapper`)
3. Injects `DataSourceMap` as second argument (populated by scheduler at execution time)
4. Generates `ContinuousTaskRegistration` for graph assembly

**PyO3 bindings needed:**
- `@continuous_task` decorator in Python (pure Python, calls into Rust for registration)
- `PythonContinuousTaskWrapper` — extends PythonTaskWrapper with DataSourceMap injection
- Auto-registration with WorkflowBuilder context manager

## Alternatives Considered

- **Python-native accumulators**: Rejected — accumulators run in the hot scheduling loop. Python overhead adds latency with no benefit. Users configure policies (Rust classes), not implement them.
- **Separate Python graph**: Rejected — mixed graphs are the whole point. A Python ML task reacting to a Rust ETL task's output is the key use case.
- **Python-first continuous scheduling**: Rejected — Rust-first ensures performance. Python is a consumer and configurator, not the foundation.
- **Expose everything via dict/JSON**: Rejected — proper Python classes with type hints are more ergonomic and catch errors earlier. PyO3 classes give IDE completion.
- **Skip trigger policy exposure**: Rejected — users MUST configure when their tasks fire. Hardcoding `Immediate` is only useful for demos.
- **Skip accumulator configuration**: Rejected — `WindowedAccumulator` with `WaitForWatermark` is essential for correctness in many use cases. Users need to choose.

## Implementation Plan

### Phase 1: Boundary and Detector Types (Data Plane)
- [ ] `ComputationBoundary` PyO3 class with BoundaryKind factory methods
- [ ] `DetectorOutput` PyO3 class with static factory methods
- [ ] `DETECTOR_OUTPUT_KEY` constant exported
- [ ] `register_custom_boundary()` PyO3 function
- [ ] `validate_boundary()` PyO3 function
- [ ] Serialization: Python dict ↔ serde_json::Value via pythonize
- [ ] Unit tests for boundary creation, serialization roundtrip

### Phase 2: Data Sources and Connections
- [ ] `DataSource` PyO3 class
- [ ] `DataSourceMetadata` PyO3 class
- [ ] `ConnectionDescriptor` PyO3 class
- [ ] `PostgresConnection` PyO3 class with builder methods
- [ ] `KafkaConnection` PyO3 class
- [ ] `S3Connection` PyO3 class
- [ ] Python `DataConnection` protocol support (Python objects wrapping trait)
- [ ] `DataSourceMap` PyO3 class with get(), connection(), names()
- [ ] Unit tests for connection construction and DataSourceMap access

### Phase 3: Graph Construction
- [ ] `ContinuousTaskRegistration` PyO3 class
- [ ] `assemble_graph()` PyO3 function
- [ ] `DataSourceGraph` PyO3 class with edges, tasks accessors
- [ ] `GraphEdge` PyO3 class with read/write late_arrival_policy
- [ ] `JoinMode` PyO3 enum
- [ ] `LateArrivalPolicy` PyO3 enum
- [ ] `GraphAssemblyError` → Python ValueError mapping
- [ ] Unit tests for graph assembly, cycle detection from Python

### Phase 4: Trigger Policies
- [ ] `Immediate` PyO3 class
- [ ] `BoundaryCount` PyO3 class
- [ ] `WallClockWindow` PyO3 class (accepts timedelta)
- [ ] `WallClockDebounce` PyO3 class (accepts timedelta)
- [ ] `AnyPolicy` PyO3 class (accepts list)
- [ ] `AllPolicy` PyO3 class (accepts list)
- [ ] timedelta → Duration conversion utility
- [ ] Unit tests for policy creation and composition

### Phase 5: Accumulators and Watermarks
- [ ] `SimpleAccumulator` PyO3 class with policy and max_buffer_size
- [ ] `WindowedAccumulator` PyO3 class with watermark_mode and boundary_ledger
- [ ] `WatermarkMode` PyO3 enum
- [ ] `GraphEdge.set_accumulator()` method
- [ ] `ContinuousScheduler.boundary_ledger()` accessor
- [ ] Unit tests for accumulator creation and graph edge configuration

### Phase 6: Scheduler Lifecycle
- [ ] `ContinuousSchedulerConfig` PyO3 class (timedelta inputs)
- [ ] `ExecutionLedger` PyO3 class with optional LedgerConfig
- [ ] `ContinuousScheduler` PyO3 class — new(), with_dal(), register_task()
- [ ] Async methods: run(), init_drain_cursors(), restore_pending_boundaries(), restore_from_persisted_state(), restore_detector_state()
- [ ] `FiredTask` PyO3 class — task_id, fired_at, executed, error
- [ ] Shutdown signal: Python asyncio.Event → Rust watch::channel bridge
- [ ] Integration test: construct and run scheduler from Python

### Phase 7: Derived Data Sources and Observability
- [ ] `LedgerTrigger` PyO3 class implementing Trigger
- [ ] `LedgerMatchMode` PyO3 enum
- [ ] `AccumulatorMetrics` PyO3 class (read-only properties)
- [ ] `EdgeMetrics` PyO3 class
- [ ] `ContinuousScheduler.graph_metrics()` method
- [ ] `DetectorStateStore` PyO3 class — get_committed(), get_latest()
- [ ] `ContinuousScheduler.detector_state_store()` accessor
- [ ] Unit tests for metrics reading and state store access

### Phase 8: `@continuous_task` Decorator
- [ ] `PythonContinuousTaskWrapper` — extends PythonTaskWrapper with DataSourceMap injection
- [ ] `@continuous_task` Python decorator with sources/referenced attributes
- [ ] Auto-registration with WorkflowBuilder context manager
- [ ] DataSourceMap injection into execution context
- [ ] Integration test: decorator → graph assembly → scheduler execution

### Phase 9: Integration, Examples, and Documentation
- [ ] Mixed Rust/Python continuous graph integration test
- [ ] Python detector workflow producing DetectorOutput with `__last_known_state`
- [ ] End-to-end example: Python detector + continuous task + derived data source
- [ ] Example: watermark usage with WindowedAccumulator from Python
- [ ] Example: custom boundary type with JSON Schema from Python
- [ ] Cloaca tutorial: continuous scheduling from Python
- [ ] API reference documentation for all Python continuous classes
- [ ] DefaultRunner integration (continuous config fields on DefaultRunnerConfig)
