---
title: "09 - Full Reactive Pipeline"
description: "Connect two accumulators to a single reactor and handle optional multi-source inputs in a computation graph"
weight: 30
---

In this tutorial you'll build a full reactive pipeline with two independent data sources — an order book feed and a pricing feed — both flowing into a single reactor. The graph fires whenever either source delivers new data, combining both into a trading signal.

## What you'll learn

- How multiple accumulators share a single boundary channel
- `when_any` with multiple sources: the graph fires on any update
- Entry nodes with multiple `Option<&T>` inputs — handling missing data gracefully
- How `InputStrategy::Latest` keeps the cache current across independent sources
- Observing that earlier sources remain cached when later ones arrive

## Prerequisites

- Completion of [Tutorial 08 — Accumulators]({{< ref "/tutorials/computation-graphs/library/08-accumulators/" >}})

## The complete example

The full source lives at [`examples/tutorials/computation-graphs/library/09-full-pipeline`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/09-full-pipeline).

To run it:

```bash
angreal demos tutorial-09
```

---

## Step 1: Define boundary types for two sources

This pipeline combines order book data with a separate pricing feed.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingUpdate {
    pub mid_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketView {
    pub spread: f64,
    pub mid_price: f64,
    pub pricing_mid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub action: String,
    pub confidence: f64,
}
```

---

## Step 2: Declare the multi-source graph

The topology lists both `orderbook` and `pricing` in the `react` criterion and in the entry node's input list.

```rust
#[cloacina_macros::computation_graph(
    react = when_any(orderbook, pricing),
    graph = {
        combine(orderbook, pricing) -> evaluate,
        evaluate -> signal,
    }
)]
pub mod market_pipeline {
    use super::*;

    /// Entry node: combines data from both sources.
    /// Both inputs are Option<&T> — either may be None if that source
    /// hasn't emitted yet.
    pub async fn combine(
        orderbook: Option<&OrderBookUpdate>,
        pricing: Option<&PricingUpdate>,
    ) -> MarketView {
        let (spread, mid) = match orderbook {
            Some(ob) => (ob.best_ask - ob.best_bid, (ob.best_ask + ob.best_bid) / 2.0),
            None => (0.0, 0.0),
        };
        let pricing_mid = pricing.map(|p| p.mid_price).unwrap_or(0.0);
        MarketView { spread, mid_price: mid, pricing_mid }
    }

    pub async fn evaluate(view: &MarketView) -> TradingSignal {
        let confidence = if view.spread > 0.0 && view.pricing_mid > 0.0 {
            let diff = (view.mid_price - view.pricing_mid).abs();
            1.0 - (diff / view.mid_price).min(1.0)
        } else {
            0.0
        };

        let action = if confidence > 0.8 {
            "TRADE".to_string()
        } else if confidence > 0.5 {
            "MONITOR".to_string()
        } else {
            "WAIT".to_string()
        };

        TradingSignal { action, confidence }
    }

    pub async fn signal(input: &TradingSignal) -> TradingSignal {
        input.clone()
    }
}
```

The key difference from Tutorial 07: `combine` takes two `Option<&T>` parameters. The graph fires when _either_ source updates, so one of them may not have a value yet. Your code must handle `None` gracefully.

---

## Step 3: Two passthrough accumulators

Each accumulator is a simple passthrough — the real logic lives in the graph.

```rust
struct OrderBookAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for OrderBookAccumulator {
    type Event = OrderBookUpdate;
    type Output = OrderBookUpdate;

    fn process(&mut self, event: OrderBookUpdate) -> Option<OrderBookUpdate> {
        Some(event)
    }
}

struct PricingAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for PricingAccumulator {
    type Event = PricingUpdate;
    type Output = PricingUpdate;

    fn process(&mut self, event: PricingUpdate) -> Option<PricingUpdate> {
        Some(event)
    }
}
```

---

## Step 4: One boundary channel, two accumulators

The critical insight here: both accumulators send to the **same** `boundary_tx`. Each uses its own `BoundarySender` with a different `SourceName`. The reactor receives all messages on one channel and dispatches by name.

```rust
// Shared boundary channel — both accumulators send to the same reactor
let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);
let (shutdown_tx, shutdown_rx) = shutdown_signal();

// --- Accumulator 1: Order Book ---
let (ob_socket_tx, ob_socket_rx) = tokio::sync::mpsc::channel(10);
let ob_sender = BoundarySender::new(boundary_tx.clone(), SourceName::new("orderbook"));
let ob_ctx = AccumulatorContext {
    output: ob_sender,
    name: "orderbook".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};
let _ob_handle = tokio::spawn(accumulator_runtime(
    OrderBookAccumulator,
    ob_ctx,
    ob_socket_rx,
    AccumulatorRuntimeConfig::default(),
));

// --- Accumulator 2: Pricing ---
// Note: boundary_tx (not .clone()) — the last sender, no clone needed
let (pr_socket_tx, pr_socket_rx) = tokio::sync::mpsc::channel(10);
let pr_sender = BoundarySender::new(boundary_tx, SourceName::new("pricing"));
let pr_ctx = AccumulatorContext {
    output: pr_sender,
    name: "pricing".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};
let _pr_handle = tokio::spawn(accumulator_runtime(
    PricingAccumulator,
    pr_ctx,
    pr_socket_rx,
    AccumulatorRuntimeConfig::default(),
));
```

You clone `boundary_tx` for the first accumulator and move the original into the second. The channel stays open as long as either sender is alive.

---

## Step 5: Reactor that prints signals

```rust
let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
let fire_count = Arc::new(AtomicU32::new(0));
let fire_count_inner = fire_count.clone();

let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
    let fc = fire_count_inner.clone();
    Box::pin(async move {
        fc.fetch_add(1, Ordering::SeqCst);
        let result = market_pipeline_compiled(&cache).await;
        if let GraphResult::Completed { outputs } = &result {
            for output in outputs {
                if let Some(signal) = output.downcast_ref::<TradingSignal>() {
                    println!(
                        "    -> Signal: {} (confidence: {:.2})",
                        signal.action, signal.confidence
                    );
                }
            }
        }
        result
    })
});

let reactor = Reactor::new(
    graph_fn,
    ReactionCriteria::WhenAny,
    InputStrategy::Latest,
    boundary_rx,
    manual_rx,
    shutdown_rx,
);
let _reactor_handle = tokio::spawn(reactor.run());
```

---

## Step 6: Push events and observe caching behaviour

```rust
// Push order book first — pricing is still None in the graph
println!("1. Push order book (pricing not yet available):");
ob_socket_tx
    .send(serialize(&OrderBookUpdate { best_bid: 100.0, best_ask: 100.10 }).unwrap())
    .await.unwrap();
tokio::time::sleep(Duration::from_millis(150)).await;
// → graph fires, orderbook=Some(...), pricing=None → WAIT

// Push pricing — now both sources have data
println!("2. Push pricing (both sources now available):");
pr_socket_tx
    .send(serialize(&PricingUpdate { mid_price: 100.05 }).unwrap())
    .await.unwrap();
tokio::time::sleep(Duration::from_millis(150)).await;
// → graph fires, orderbook=Some(cached), pricing=Some(new) → TRADE

// Push updated order book — reactor fires again with cached pricing + new orderbook
println!("3. Push updated order book (wider spread):");
ob_socket_tx
    .send(serialize(&OrderBookUpdate { best_bid: 99.50, best_ask: 100.50 }).unwrap())
    .await.unwrap();
tokio::time::sleep(Duration::from_millis(150)).await;
// → graph fires, orderbook=Some(new), pricing=Some(cached) → WAIT or MONITOR

// Push pricing update that diverges from order book
println!("4. Push pricing update (divergent from order book):");
pr_socket_tx
    .send(serialize(&PricingUpdate { mid_price: 105.00 }).unwrap())
    .await.unwrap();
tokio::time::sleep(Duration::from_millis(150)).await;
// → graph fires, orderbook=Some(cached), pricing=Some(new divergent) → WAIT
```

Notice what happens at step 2: you only push `pricing`, but `orderbook` is still in the cache from step 1. The reactor retains all sources between firings — `InputStrategy::Latest` overwrites on update but never clears.

---

## Expected output

```
Pipeline running: 2 accumulators → 1 reactor → compiled graph

1. Push order book (pricing not yet available):
    -> Signal: WAIT (confidence: 0.00)
   Fires: 1

2. Push pricing (both sources now available):
    -> Signal: TRADE (confidence: 1.00)
   Fires: 2

3. Push updated order book (wider spread):
    -> Signal: WAIT (confidence: 0.00)
   Fires: 3

4. Push pricing update (divergent from order book):
    -> Signal: WAIT (confidence: 0.00)
   Fires: 4

Shutting down...
Total fires: 4
```

---

## Summary

You've built a full multi-source reactive pipeline:

- Two accumulators share one boundary channel using `boundary_tx.clone()` for all but the last
- Each accumulator uses its own `BoundarySender` with a distinct `SourceName`
- The reactor's `InputCache` retains each source's latest value between firings
- Entry nodes handle `Option<&T>` inputs so the graph can fire before all sources have data
- `ReactionCriteria::WhenAny` fires on any source update; older cached values fill in the rest

## What's next?

- [Tutorial 10 — Routing]({{< ref "/tutorials/computation-graphs/library/10-routing/" >}}): add conditional branching with enum dispatch — route the graph down different paths based on the decision node's output
