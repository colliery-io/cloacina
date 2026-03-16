---
id: detector-workflows-change
level: specification
title: "Detector Workflows - Change Detection Contract"
short_code: "CLOACI-S-0004"
created_at: 2026-03-10T18:18:21.696188+00:00
updated_at: 2026-03-10T18:18:21.696188+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Detector Workflows - Change Detection Contract

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

Change detection is not a special framework component — it is a regular Cloacina workflow with a specific output contract: its output context must contain `DetectorOutput` values (boundaries and/or watermark advances).

This means:
- No new scheduling infrastructure for detection
- No detection code running in the scheduler process
- Detector complexity is unlimited — a detector workflow can have multiple tasks, conditional branching, retries
- Detection is observable, retriable, and monitored through the existing execution pipeline
- The `DataConnection` on the `DataSource` (CLOACI-S-0003) is available to the detector workflow's tasks

## Triggering

Detector workflows are triggered via the existing scheduling infrastructure — Cloacina polls, external systems do not push:
- **CronTrigger (interval polling)** — existing `CronScheduler` runs the detector on an interval. "Check this table every 30 seconds."
- **LedgerTrigger** (CLOACI-S-0007) — watches the execution ledger for upstream task completions. "Run when aggregate_hourly completes." This is how derived data sources trigger detection.

Both are implementations of the existing `Trigger` trait. All paths result in the same thing: the workflow runs, polls external data via `DataConnection.connect()`, produces `DetectorOutput` in its output context, and the `ContinuousScheduler` picks it up from the execution ledger and routes boundaries to accumulators.

## Output Contract — DetectorOutput

```rust
enum DetectorOutput {
    /// Data changed — one or more new boundaries to process.
    /// Vec allows a single detector poll to emit multiple boundary ranges
    /// (e.g., multiple new partitions discovered in one check).
    Change { boundaries: Vec<ComputationBoundary> },
    /// Assertion: no boundaries earlier than this will arrive in the future
    WatermarkAdvance { boundary: ComputationBoundary },
    /// Both data changes and a watermark advance in one emission
    Both { boundaries: Vec<ComputationBoundary>, watermark: ComputationBoundary },
}
```

A detector workflow writes one or more `DetectorOutput` values into its output context. The `ContinuousScheduler` observes the completion via the execution ledger (CLOACI-S-0007), reads the output, and routes boundaries to the appropriate accumulators (CLOACI-S-0005).

## Example Detector Workflow

```rust
#[workflow(name = "detect_raw_events_changes")]
fn build() -> Workflow {
    WorkflowBuilder::new("detect_raw_events_changes")
        .add_task(check_for_new_data)
        .build()
}

#[task]
async fn check_for_new_data(ctx: &mut Context, inputs: &DataSourceMap) -> Result<()> {
    let conn = inputs.get("raw_events").connection.connect()?;
    let last_known = ctx.get("__last_known_state")?;
    let current_max = query_max_timestamp(&conn).await?;

    if current_max > last_known {
        let output = DetectorOutput::Both {
            change: ComputationBoundary {
                kind: BoundaryKind::TimeRange { start: last_known, end: current_max },
                metadata: Some(json!({ "row_count": count_new_rows(&conn, last_known, current_max).await? })),
                emitted_at: Utc::now(),
            },
            watermark: ComputationBoundary {
                kind: BoundaryKind::TimeRange { start: DateTime::MIN, end: current_max },
                metadata: None,
                emitted_at: Utc::now(),
            },
        };
        ctx.insert("__detector_output", serde_json::to_value(&output)?)?;
    }
    Ok(())
}
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Detectors are workflows, not framework components | Unlimited complexity, observable, retriable, monitored through existing pipeline |
| No detection code in scheduler process | Keeps scheduler focused on routing. Detection runs through existing execution infrastructure. |
| Cloacina polls, external systems don't push | Polling via DataConnection keeps the system simple — one direction of control. No inbound API surface for detection. |
| Single output contract (DetectorOutput) | Both trigger paths (cron, ledger trigger) produce the same output. Uniform routing. |
| Detector workflows can have multiple tasks | Complex detection logic (multi-step validation, conditional branching) is a workflow concern, not a framework concern. |
