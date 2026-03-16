---
title: "12 - Continuous Scheduling"
description: "Build a reactive pipeline that automatically re-executes tasks when data changes"
weight: 22
reviewer: "dstorey"
review_date: "2026-03-15"
---

## Overview

This tutorial walks through building a continuous reactive pipeline with Cloacina. By the end, you'll have a graph that watches a data source for changes and automatically runs an aggregation task when new data arrives.

## Prerequisites

- A Rust project with `cloacina`, `cloacina-workflow`, and `parking_lot` dependencies
- `tokio` with the `full` feature for async runtime
- Familiarity with the basic `#[task]` macro from earlier tutorials

## Concepts

In continuous scheduling, you define:

1. **Data sources** — external systems your pipeline watches
2. **Detector workflows** — regular tasks that poll data sources for changes
3. **Continuous tasks** — tasks that run when their data sources change
4. **Trigger policies** — rules for when accumulated changes should fire a task
5. **Watermarks** — assertions about data completeness for late arrival handling

## Step 1: Define a Data Source

A data source is a named handle to an external dataset. You provide a `DataConnection` implementation:

```rust
use cloacina::continuous::datasource::*;
use std::any::Any;

struct MyDbConnection {
    table: String,
}

impl DataConnection for MyDbConnection {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new(format!("postgres://localhost/{}", self.table)))
    }

    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "postgres".into(),
            location: format!("localhost/{}", self.table),
        }
    }

    fn system_metadata(&self) -> serde_json::Value {
        serde_json::json!({ "table": self.table })
    }
}
```

Register the data source:

```rust
let source = DataSource {
    name: "raw_events".into(),
    connection: Box::new(MyDbConnection { table: "events".into() }),
    detector_workflow: "detect_raw_events".into(),
    lineage: DataSourceMetadata {
        description: Some("Raw event stream".into()),
        owner: Some("data-team".into()),
        tags: vec!["events".into()],
    },
};
```

The framework provides `PostgresConnection`, `KafkaConnection`, and `S3Connection` for common systems.

## Step 2: Assemble the Graph

The graph is built from data sources and task registrations:

```rust
use cloacina::continuous::graph::*;

let graph = assemble_graph(
    vec![source],
    vec![ContinuousTaskRegistration {
        id: "aggregate_hourly".into(),
        sources: vec!["raw_events".into()],
        referenced: vec![],
    }],
).expect("graph assembly failed");
```

Each `source` in the registration creates an edge with a `SimpleAccumulator` and `Immediate` trigger policy by default. The graph is validated for cycles at assembly time.

## Step 3: Simulate Detector Output

In production, detector workflows are regular Cloacina tasks triggered by cron. They poll data sources and emit `DetectorOutput`. For this tutorial, we simulate detector completions:

```rust
use cloacina::continuous::boundary::*;
use cloacina::continuous::detector::*;
use cloacina::continuous::ledger::*;
use parking_lot::RwLock;

let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

// Simulate: detector found new data at offsets 0-100
let mut ctx = Context::new();
let output = DetectorOutput::Change {
    boundaries: vec![ComputationBoundary {
        kind: BoundaryKind::OffsetRange { start: 0, end: 100 },
        metadata: Some(serde_json::json!({"row_count": 100})),
        emitted_at: Utc::now(),
    }],
};
ctx.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output).unwrap()).unwrap();

// Detectors can also persist their state for crash recovery
ctx.insert("__last_known_state", serde_json::json!({"cursor": 100})).unwrap();

let mut l = ledger.write();
l.append(LedgerEvent::TaskCompleted {
    task: "detect_raw_events".into(),
    at: Utc::now(),
    context: ctx,
});
```

## Step 4: Run the Scheduler

The `ContinuousScheduler` runs a reactive loop, polling the ledger for detector completions:

```rust
use cloacina::continuous::scheduler::*;

let mut scheduler = ContinuousScheduler::new(
    graph,
    ledger.clone(),
    ContinuousSchedulerConfig {
        poll_interval: Duration::from_millis(100),
        max_fired_tasks: 10_000,
        task_timeout: Some(Duration::from_secs(300)), // 5 minute timeout per task
    },
);

// Register the task implementation
scheduler.register_task(Arc::new(MyAggregateTask));

// Optional: enable persistence for crash recovery
// scheduler = scheduler.with_dal(dal);
// scheduler.init_drain_cursors().await;
// scheduler.restore_pending_boundaries().await;
// scheduler.restore_from_persisted_state().await;
// scheduler.restore_detector_state().await;

let (tx, rx) = tokio::sync::watch::channel(false);
let handle = tokio::spawn(async move { scheduler.run(rx).await });

// Let it process
tokio::time::sleep(Duration::from_millis(200)).await;
tx.send(true).unwrap(); // shutdown

let fired = handle.await.unwrap();
println!("Tasks fired: {}", fired.len());
```

## Step 5: Reading Boundary Context

When your continuous task fires, the coalesced boundary is available in the context:

```rust
#[continuous_task(id = "aggregate_hourly", sources = ["raw_events"])]
async fn aggregate_hourly(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // The accumulator injected these keys
    let boundary = ctx.get("__boundary").unwrap();
    let signals_coalesced = ctx.get("__signals_coalesced").unwrap();

    println!("Processing boundary: {}", boundary);
    println!("Coalesced from {} signals", signals_coalesced);

    Ok(())
}
```

If multiple boundaries arrived before the trigger fired, they're coalesced:
- `OffsetRange [0,100) + [100,250)` becomes `[0,250)`
- `TimeRange [14:00,15:00) + [15:00,16:00)` becomes `[14:00,16:00)`

## Boundary Coalescing

The coalescing behavior depends on the boundary kind:

| Kind | Rule | Example |
|------|------|---------|
| TimeRange | Union: min start, max end | `[14:00,15:00) + [15:00,16:00)` → `[14:00,16:00)` |
| OffsetRange | Union: min start, max end | `[0,100) + [100,200)` → `[0,200)` |
| Cursor | Latest wins | `"abc" + "def"` → `"def"` |
| FullState | Latest wins | `"v7" + "v8"` → `"v8"` |
| Custom | Latest wins (schema-validated) | User-defined |

Mixed boundary kinds refuse to coalesce (logged as warning).

## Trigger Policies

Control when accumulated boundaries fire a task:

| Policy | Fires when | Example |
|--------|-----------|---------|
| `Immediate` | Buffer non-empty | Every boundary fires immediately |
| `BoundaryCount(n)` | N boundaries buffered | Fire every 20 boundaries |
| `WallClockWindow(d)` | Duration since last drain | Fire every 5 minutes |
| `WallClockDebounce(d)` | Silence for duration | Fire when burst is over (no new data for 30s) |
| `AnyPolicy(vec![...])` | Any sub-policy matches | "5 minutes OR 20 boundaries" |
| `AllPolicy(vec![...])` | All sub-policies match | "1000 boundaries AND 1 minute" |

```rust
use cloacina::continuous::trigger_policy::*;

// Fire every 5 minutes OR when 20 boundaries accumulate
let policy = AnyPolicy(vec![
    Box::new(WallClockWindow::new(Duration::from_secs(300))),
    Box::new(BoundaryCount::new(20)),
]);
```

## Watermarks and Late Arrival

The system uses a **two-watermark model**:

- **Source watermarks** (`BoundaryLedger`): per data source, "nothing earlier will arrive" — a user assertion from the detector
- **Consumer watermarks** (per accumulator): "I've processed up to here" — updated on each drain

The `WindowedAccumulator` checks source watermarks before firing:

```rust
let acc = WindowedAccumulator::new(
    Box::new(WallClockWindow::new(Duration::from_secs(3600))),
    WatermarkMode::WaitForWatermark,
    boundary_ledger.clone(),
    "raw_events".into(),
);
```

When a boundary arrives behind a consumer watermark, the per-edge `LateArrivalPolicy` determines behavior:
- `Discard` — drop the late boundary
- `AccumulateForward` — buffer for next cycle (default)
- `Retrigger` — re-submit for re-execution

## Crash Recovery

With a DAL configured, the scheduler persists state for crash recovery:

1. **Pending boundaries** are written to a WAL (write-ahead log) when routed to accumulators
2. **Consumer watermarks** are persisted on drain
3. **Detector state** (`__last_known_state`) is committed when all consumers drain

On restart, the startup sequence restores state in the correct order:

```rust
scheduler.init_drain_cursors().await;        // 1. Init cursor tracking
scheduler.restore_pending_boundaries().await; // 2. Re-inject un-consumed boundaries
scheduler.restore_from_persisted_state().await; // 3. Restore consumer watermarks
scheduler.restore_detector_state().await;     // 4. Load detector checkpoints
// Now run — detectors resume from committed checkpoint
```

The boundary WAL uses a Kafka-style consumer group model: one log per source, independent cursors per consuming edge. A boundary is only cleaned up when ALL consumers have drained past it.

## Derived Data Sources

`LedgerTrigger` watches the `ExecutionLedger` for task completions and fires detector workflows for derived data sources — completing the reactive feedback loop:

```rust
let trigger = LedgerTrigger::new(
    "detect_hourly_aggregates".into(),
    vec!["aggregate_hourly".into()],
    LedgerMatchMode::Any,
    ledger.clone(),
);
// When aggregate_hourly completes → trigger fires → detect_hourly_aggregates runs
```

## Next Steps

- Read the [Continuous Scheduling explanation]({{< ref "/explanation/continuous-scheduling" >}}) for architectural details
- Explore custom `TriggerPolicy` implementations for domain-specific firing rules
- See the `examples/features/continuous-scheduling` example for a runnable demo
