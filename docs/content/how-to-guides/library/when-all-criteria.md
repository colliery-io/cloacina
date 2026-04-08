---
title: "Using when_all Reaction Criteria"
description: "How to configure a computation graph to fire only after all input sources have emitted"
weight: 30
---

# Using when_all Reaction Criteria

This guide explains how to use `when_all` reaction criteria to hold a computation graph until every input source has emitted at least once.

## Prerequisites

- Familiarity with the reactor model (see [tutorial 09 — full pipeline]({{< ref "/tutorials/computation-graphs/library/09-full-pipeline" >}}))
- A computation graph with multiple named inputs

## when_any vs when_all

The reactor evaluates dirty flags every time a boundary arrives. The two criteria differ in what they require before firing:

| Criteria | Fires when |
|----------|-----------|
| `when_any` | At least one source has new data since the last execution |
| `when_all` | Every declared source has emitted at least once since the last execution |

Use `when_any` when each source is self-sufficient — the graph can compute a meaningful result from a single input. This is the default in most streaming pipelines where you want low latency.

Use `when_all` when your graph function requires data from every source to produce a valid result. A common example is a graph that joins two independent feeds: executing before both have arrived would produce a meaningless result with all `None` inputs.

## Declaring when_all in the graph macro

Change `react = when_any(...)` to `react = when_all(...)` in the `#[computation_graph]` attribute:

```rust
#[cloacina_macros::computation_graph(
    react = when_all(orderbook, pricing),
    graph = {
        combine(orderbook, pricing) -> evaluate,
        evaluate -> signal,
    }
)]
pub mod market_pipeline {
    // ...
}
```

The source names inside `when_all(...)` must match the parameter names of your entry node function.

## How dirty flags work with when_all

The reactor maintains one dirty flag per source. On each boundary arrival the corresponding flag is set. The executor checks:

- `when_any`: fires if any flag is set
- `when_all`: fires only when every flag is set

After each execution all flags are cleared, so the next execution again requires all sources to emit.

**Critical**: For `when_all` to work correctly, the reactor must be told the full set of expected source names at startup. This seeds the dirty flags to `false` for all sources. Without this seeding, `all_set()` would incorrectly return `true` the first time any single source emits (because it would only see the one flag that exists, which is set).

Provide the expected sources when building the reactor:

```rust
use cloacina::computation_graph::reactor::{Reactor, ReactionCriteria, InputStrategy};

let reactor = Reactor::new(
    graph_fn,
    ReactionCriteria::WhenAll,
    InputStrategy::Latest,
    boundary_rx,
    manual_rx,
    shutdown_rx,
)
.with_expected_sources(vec![
    SourceName::new("orderbook"),
    SourceName::new("pricing"),
]);
```

## Complete example

The following example converts tutorial 09's `when_any` pipeline to `when_all`. The graph does not fire on the first order book push; it waits until the pricing source has also emitted.

```rust
use std::sync::atomic::{AtomicU32, Ordering};
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
pub struct OrderBookUpdate { pub best_bid: f64, pub best_ask: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingUpdate { pub mid_price: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketView { pub spread: f64, pub mid_price: f64, pub pricing_mid: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal { pub action: String, pub confidence: f64 }

// Same graph declaration as tutorial 09, but with when_all
#[cloacina_macros::computation_graph(
    react = when_all(orderbook, pricing),   // <-- changed from when_any
    graph = {
        combine(orderbook, pricing) -> evaluate,
        evaluate -> signal,
    }
)]
pub mod market_pipeline {
    use super::*;

    pub async fn combine(
        orderbook: Option<&OrderBookUpdate>,
        pricing: Option<&PricingUpdate>,
    ) -> MarketView {
        let (spread, mid) = match orderbook {
            Some(ob) => (ob.best_ask - ob.best_bid, (ob.best_ask + ob.best_bid) / 2.0),
            None => (0.0, 0.0),
        };
        MarketView {
            spread,
            mid_price: mid,
            pricing_mid: pricing.map(|p| p.mid_price).unwrap_or(0.0),
        }
    }

    pub async fn evaluate(view: &MarketView) -> TradingSignal {
        TradingSignal {
            action: if view.spread < 0.5 { "TRADE".into() } else { "WAIT".into() },
            confidence: 1.0 - view.spread.min(1.0),
        }
    }

    pub async fn signal(input: &TradingSignal) -> TradingSignal { input.clone() }
}

struct OrderBookAccumulator;
#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for OrderBookAccumulator {
    type Event = OrderBookUpdate;
    type Output = OrderBookUpdate;
    fn process(&mut self, event: OrderBookUpdate) -> Option<OrderBookUpdate> { Some(event) }
}

struct PricingAccumulator;
#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for PricingAccumulator {
    type Event = PricingUpdate;
    type Output = PricingUpdate;
    fn process(&mut self, event: PricingUpdate) -> Option<PricingUpdate> { Some(event) }
}

#[tokio::main]
async fn main() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // Accumulator: order book
    let (ob_socket_tx, ob_socket_rx) = tokio::sync::mpsc::channel(10);
    let ob_sender = BoundarySender::new(boundary_tx.clone(), SourceName::new("orderbook"));
    let ob_ctx = AccumulatorContext {
        output: ob_sender,
        name: "orderbook".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };
    tokio::spawn(accumulator_runtime(
        OrderBookAccumulator, ob_ctx, ob_socket_rx, AccumulatorRuntimeConfig::default(),
    ));

    // Accumulator: pricing
    let (pr_socket_tx, pr_socket_rx) = tokio::sync::mpsc::channel(10);
    let pr_sender = BoundarySender::new(boundary_tx, SourceName::new("pricing"));
    let pr_ctx = AccumulatorContext {
        output: pr_sender,
        name: "pricing".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };
    tokio::spawn(accumulator_runtime(
        PricingAccumulator, pr_ctx, pr_socket_rx, AccumulatorRuntimeConfig::default(),
    ));

    // Reactor with WhenAll and seeded expected sources
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            market_pipeline_compiled(&cache).await
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAll,       // require all sources
        InputStrategy::Latest,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    )
    .with_expected_sources(vec![         // seed dirty flags
        SourceName::new("orderbook"),
        SourceName::new("pricing"),
    ]);
    tokio::spawn(reactor.run());

    // Push only order book — reactor does NOT fire (pricing not yet received)
    ob_socket_tx.send(
        serialize(&OrderBookUpdate { best_bid: 100.0, best_ask: 100.10 }).unwrap()
    ).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("After orderbook only — fires: {}", fire_count.load(Ordering::SeqCst)); // 0

    // Push pricing — now both sources have emitted, reactor fires once
    pr_socket_tx.send(
        serialize(&PricingUpdate { mid_price: 100.05 }).unwrap()
    ).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("After pricing too — fires: {}", fire_count.load(Ordering::SeqCst)); // 1

    // After firing, flags are cleared. Next push from either source alone does NOT fire.
    // Both sources must emit again before the next execution.
    ob_socket_tx.send(
        serialize(&OrderBookUpdate { best_bid: 99.5, best_ask: 100.5 }).unwrap()
    ).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("After second orderbook alone — fires: {}", fire_count.load(Ordering::SeqCst)); // still 1

    shutdown_tx.send(true).unwrap();
}
```

## Behaviour summary

1. On startup, dirty flags are seeded to `false` for each expected source.
2. Each boundary received from a source sets that source's flag to `true`.
3. The executor fires only when every flag is `true`.
4. After firing, all flags are reset to `false`.
5. The next execution requires all sources to emit again.

This means `when_all` is strictly "at least one new boundary from each source per cycle", not "simultaneous". If source A emits 10 times before source B emits once, the reactor fires once (using the latest boundary from A).

## Related

- [Choosing and using accumulator types]({{< ref "how-to-guides/library/accumulator-types" >}})
- [How to use sequential input strategy]({{< ref "how-to-guides/library/sequential-strategy" >}})
