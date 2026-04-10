---
title: "Using Sequential Input Strategy"
description: "How to configure a reactor to process every boundary in order rather than collapsing to the latest"
weight: 40
---

# Using Sequential Input Strategy

This guide explains when to use `InputStrategy::Sequential` instead of the default `InputStrategy::Latest`, and how to configure it.

## Prerequisites

- Familiarity with the reactor model (see [tutorial 09 — full pipeline]({{< ref "/tutorials/computation-graphs/library/09-full-pipeline" >}}))
- A working accumulator-reactor pipeline

## Latest vs Sequential

The reactor stores incoming boundaries in a cache keyed by source name. How it handles new boundaries before the graph has fired depends on the input strategy:

| Strategy | Cache behaviour | Guarantee |
|----------|----------------|-----------|
| `Latest` | Overwrites the previous boundary for that source | Graph always sees the freshest data; intermediate values may be skipped |
| `Sequential` | Queues every boundary in arrival order | Graph executes once per boundary; no event is ever skipped |

### When to use Latest (the default)

Latest is the correct choice for most reactive pipelines:

- Price feeds, sensor readings, configuration — you want the most current value
- High-throughput sources where intermediate values have no meaning on their own
- When the graph is idempotent over the same data and skipping stale values is fine

### When to use Sequential

Use Sequential when every individual boundary must be processed:

- **Audit trails**: each event has independent business meaning and must appear in results
- **Financial ledgers**: every transaction must be accounted for, not collapsed
- **Event sourcing**: you are replaying history and every step matters
- **Workflow triggers**: each boundary triggers a distinct side effect

If a source emits 10 boundaries before the graph has a chance to run, Latest processes them as one execution with the 10th value. Sequential processes all 10 as 10 separate executions in arrival order.

## Configuring Sequential

Pass `InputStrategy::Sequential` when constructing the reactor:

```rust
use cloacina::computation_graph::reactor::{Reactor, ReactionCriteria, InputStrategy};

let reactor = Reactor::new(
    graph_fn,
    ReactionCriteria::WhenAny,
    InputStrategy::Sequential,   // <-- process every boundary
    boundary_rx,
    manual_rx,
    shutdown_rx,
);
```

Sequential works with both `WhenAny` and `WhenAll` reaction criteria.

## How the queue works

With `InputStrategy::Sequential` the receiver task does not update the shared cache immediately. Instead it pushes each `(source, bytes)` pair onto an in-order queue.

When the executor wakes up, it drains the queue one item at a time:

1. Pop the front item from the queue.
2. Update the cache with that boundary.
3. Execute the graph with the current cache snapshot.
4. Persist the queue state to the DAL for crash resilience.
5. Repeat until the queue is empty.

This means that if five boundaries arrive before the executor runs, the graph fires five times sequentially, each time with the cache reflecting exactly one additional boundary.

## Crash resilience

The queue is persisted to the DAL before each drain cycle. If the process crashes mid-drain, the remaining unprocessed items survive and replay on restart. This gives Sequential strategy at-least-once delivery semantics.

## Example — audit trail pipeline

```rust
use std::sync::Arc;

use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAction {
    pub user_id: String,
    pub action: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub user_id: String,
    pub action: String,
    pub timestamp_ms: u64,
    pub processed_at_ms: u64,
}

#[cloacina_macros::computation_graph(
    react = when_any(actions),
    graph = {
        record(actions),
    }
)]
pub mod audit_pipeline {
    use super::*;

    pub async fn record(actions: Option<&UserAction>) -> AuditRecord {
        let action = actions.expect("when_any guarantees actions is Some");
        AuditRecord {
            user_id: action.user_id.clone(),
            action: action.action.clone(),
            timestamp_ms: action.timestamp_ms,
            processed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

// Passthrough accumulator — events arrive via socket
struct ActionAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for ActionAccumulator {
    type Event = UserAction;
    type Output = UserAction;

    fn process(&mut self, event: UserAction) -> Option<UserAction> {
        Some(event)
    }
}

#[tokio::main]
async fn main() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(256);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(256);
    let sender = BoundarySender::new(boundary_tx, SourceName::new("actions"));
    let ctx = AccumulatorContext {
        output: sender,
        name: "actions".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };
    tokio::spawn(accumulator_runtime(
        ActionAccumulator,
        ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let processed = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let processed_inner = processed.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let p = processed_inner.clone();
        Box::pin(async move {
            let result = audit_pipeline_compiled(&cache).await;
            if let GraphResult::Completed { ref outputs } = result {
                p.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                for output in outputs {
                    if let Some(record) = output.downcast_ref::<AuditRecord>() {
                        println!("Audit: {:?}", record);
                    }
                }
            }
            result
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAny,
        InputStrategy::Sequential,   // every action gets its own execution
        boundary_rx,
        manual_rx,
        shutdown_rx,
    );
    tokio::spawn(reactor.run());

    // Send three actions in rapid succession
    let actions = vec![
        UserAction { user_id: "alice".into(), action: "login".into(), timestamp_ms: 1000 },
        UserAction { user_id: "alice".into(), action: "view_report".into(), timestamp_ms: 1001 },
        UserAction { user_id: "alice".into(), action: "export_csv".into(), timestamp_ms: 1002 },
    ];

    for action in actions {
        socket_tx.send(serialize(&action).unwrap()).await.unwrap();
    }

    // With Latest, the graph would fire once and only see "export_csv".
    // With Sequential, the graph fires three times — once per action.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!("Total executions: {}", processed.load(std::sync::atomic::Ordering::SeqCst)); // 3

    shutdown_tx.send(true).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}
```

## Performance considerations

Sequential has higher per-event overhead than Latest: each boundary causes one full graph execution and one DAL persistence cycle. For high-throughput sources this can become a bottleneck. If you need guaranteed delivery at high volume, consider pairing Sequential with a [batch accumulator]({{< ref "how-to-guides/library/accumulator-types" >}}#batch) to amortise cost: buffer many events into one boundary, then let Sequential ensure every batch is processed in order.

## Related

- [Choosing and using accumulator types]({{< ref "how-to-guides/library/accumulator-types" >}})
- [How to use when_all reaction criteria]({{< ref "how-to-guides/library/when-all-criteria" >}})
