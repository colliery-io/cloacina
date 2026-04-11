---
title: "10 - Routing and Enum Dispatch"
description: "Route computation graph execution down conditional paths using enum variants and the => topology syntax"
weight: 40
---

In this tutorial you'll build a market maker that routes each market event to one of two outcomes: execute a trade or log a no-action audit record. The graph uses the `=>` routing syntax to dispatch based on a Rust enum returned by the decision node.

## What you'll learn

- The `=>` routing syntax in the topology declaration
- Defining a routing enum whose variants carry data for downstream nodes
- Multiple terminal nodes — one per branch
- How the runtime matches enum variants and routes data accordingly
- How input conditions determine which path executes

## Prerequisites

- Completion of [Tutorial 09 — Full Reactive Pipeline]({{< ref "/computation-graphs/tutorials/library/09-full-pipeline/" >}})

## The complete example

The full source lives at [`examples/tutorials/computation-graphs/library/10-routing`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/10-routing).

To run it:

```bash
angreal demos tutorial-10
```

---

## Step 1: Define boundary types

You need a type for each branch of the routing decision.

```rust
/// Input types from the two data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub mid_price: f64,
}

/// Data carried when the decision is to trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub direction: String,
    pub price: f64,
    pub confidence: f64,
}

/// Data carried when the decision is no action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoActionReason {
    pub reason: String,
}

/// Terminal outputs — one per branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeConfirmation {
    pub executed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub logged: bool,
    pub reason: String,
}
```

Each variant of the routing enum carries a different payload type. That payload is what the downstream branch node receives.

---

## Step 2: Declare the routing graph

The `=>` syntax replaces the `->` arrow on a routing node. Inside `=>`, you map each enum variant name to its downstream handler.

```rust
#[cloacina_macros::computation_graph(
    react = when_any(orderbook, pricing),
    graph = {
        decision(orderbook, pricing) => {
            Trade -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod market_maker {
    use super::*;

    /// The routing enum. Each variant carries the data for its branch.
    /// Note: does NOT need Serialize/Deserialize — it's never stored in the cache.
    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Trade(TradeSignal),
        NoAction(NoActionReason),
    }

    /// Decision engine: evaluates market data and decides whether to trade.
    pub async fn decision(
        orderbook: Option<&OrderBookData>,
        pricing: Option<&PricingData>,
    ) -> DecisionOutcome {
        let (bid, ask) = match orderbook {
            Some(ob) => (ob.best_bid, ob.best_ask),
            None => {
                return DecisionOutcome::NoAction(NoActionReason {
                    reason: "no order book data".to_string(),
                });
            }
        };

        let mid = (bid + ask) / 2.0;
        let spread = ask - bid;
        let pricing_mid = pricing.map(|p| p.mid_price).unwrap_or(mid);
        let price_diff = (mid - pricing_mid).abs();

        if spread < 0.20 && price_diff < 0.50 {
            DecisionOutcome::Trade(TradeSignal {
                direction: if pricing_mid > mid { "BUY".to_string() } else { "SELL".to_string() },
                price: mid,
                confidence: 1.0 - (price_diff / mid),
            })
        } else {
            let reason = if spread >= 0.20 {
                format!("spread too wide: {:.2}", spread)
            } else {
                format!("price divergence too high: {:.2}", price_diff)
            };
            DecisionOutcome::NoAction(NoActionReason { reason })
        }
    }

    /// Signal handler: executes the trade (terminal node on Trade path).
    pub async fn signal_handler(signal: &TradeSignal) -> TradeConfirmation {
        TradeConfirmation {
            executed: true,
            message: format!(
                "{} @ {:.2} (confidence: {:.4})",
                signal.direction, signal.price, signal.confidence
            ),
        }
    }

    /// Audit logger: records why no action was taken (terminal on NoAction path).
    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: true,
            reason: reason.reason.clone(),
        }
    }
}
```

**How routing works:**

When `decision` returns `DecisionOutcome::Trade(signal)`, the macro extracts the inner `TradeSignal` and passes it to `signal_handler`. When it returns `DecisionOutcome::NoAction(reason)`, the inner `NoActionReason` goes to `audit_logger`. Only one branch executes per graph invocation.

The routing enum lives inside the module and does **not** need `Serialize/Deserialize` — it is created, matched, and discarded within the graph execution without ever being stored in the cache.

---

## Step 3: Inspect both branches in the reactor

Because only one branch runs per execution, `GraphResult::Completed { outputs }` will contain either a `TradeConfirmation` or an `AuditRecord` — never both. You can `downcast_ref` for each and handle whichever is present.

```rust
let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
    let fc = fc.clone();
    Box::pin(async move {
        fc.fetch_add(1, Ordering::SeqCst);
        let result = market_maker_compiled(&cache).await;
        if let GraphResult::Completed { outputs } = &result {
            for output in outputs {
                if let Some(confirm) = output.downcast_ref::<TradeConfirmation>() {
                    println!("    [TRADE] {}", confirm.message);
                }
                if let Some(audit) = output.downcast_ref::<AuditRecord>() {
                    println!("    [NO ACTION] {}", audit.reason);
                }
            }
        }
        result
    })
});
```

---

## Step 4: The accumulator and reactor wiring

The wiring follows the same pattern as Tutorial 09 — two accumulators, one shared boundary channel.

```rust
let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);
let (shutdown_tx, shutdown_rx) = shutdown_signal();

// Order book accumulator
let (ob_tx, ob_rx) = tokio::sync::mpsc::channel(10);
let _ob = tokio::spawn(accumulator_runtime(
    OrderBookAccumulator,
    AccumulatorContext {
        output: BoundarySender::new(boundary_tx.clone(), SourceName::new("orderbook")),
        name: "orderbook".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    },
    ob_rx,
    AccumulatorRuntimeConfig::default(),
));

// Pricing accumulator
let (pr_tx, pr_rx) = tokio::sync::mpsc::channel(10);
let _pr = tokio::spawn(accumulator_runtime(
    PricingAccumulator,
    AccumulatorContext {
        output: BoundarySender::new(boundary_tx, SourceName::new("pricing")),
        name: "pricing".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    },
    pr_rx,
    AccumulatorRuntimeConfig::default(),
));
```

---

## Step 5: Five scenarios, two branches

```rust
// 1. Pricing only — no order book → NoAction
pr_tx.send(serialize(&PricingData { mid_price: 100.05 }).unwrap()).await.unwrap();
// → [NO ACTION] no order book data

// 2. Tight spread (0.10) + pricing confirms → Trade
ob_tx.send(serialize(&OrderBookData { best_bid: 100.00, best_ask: 100.10 }).unwrap()).await.unwrap();
// → [TRADE] BUY @ 100.05 (confidence: 0.9995)

// 3. Wide spread (1.00) → NoAction
ob_tx.send(serialize(&OrderBookData { best_bid: 99.50, best_ask: 100.50 }).unwrap()).await.unwrap();
// → [NO ACTION] spread too wide: 1.00

// 4. Tight spread + divergent pricing → NoAction
ob_tx.send(serialize(&OrderBookData { best_bid: 100.00, best_ask: 100.10 }).unwrap()).await.unwrap();
// brief pause so it fires before pricing update...
pr_tx.send(serialize(&PricingData { mid_price: 105.00 }).unwrap()).await.unwrap();
// → [NO ACTION] price divergence too high: 4.95

// 5. Everything aligned → Trade
ob_tx.send(serialize(&OrderBookData { best_bid: 102.00, best_ask: 102.08 }).unwrap()).await.unwrap();
pr_tx.send(serialize(&PricingData { mid_price: 102.05 }).unwrap()).await.unwrap();
// → [TRADE] BUY @ 102.04 (confidence: 0.9995)
```

---

## Expected output

```
Market maker running: decision engine with Trade/NoAction routing

1. Push pricing only (no order book yet):
    [NO ACTION] no order book data

2. Push tight order book (spread=0.10) + pricing confirms:
    [TRADE] BUY @ 100.05 (confidence: 0.9995)

3. Push wide order book (spread=1.00):
    [NO ACTION] spread too wide: 1.00

4. Push tight order book but divergent pricing:
    [NO ACTION] price divergence too high: 4.95

5. Push aligned data (tight spread + confirmed):
    [TRADE] BUY @ 102.04 (confidence: 0.9995)

Shutting down...
Total fires: 6
```

(Six fires because scenarios 4 and 5 each push two events — the graph fires after each one.)

---

## Summary

You've implemented a routed computation graph:

- The `=>` syntax in the topology declares enum-dispatch routing
- The routing enum is defined inside the module and its variant names must match the route keys
- Each variant carries the payload type that its downstream node receives as `&T`
- Only one branch executes per graph invocation — `GraphResult::Completed` holds the output from whichever branch ran
- `downcast_ref` for each possible terminal type handles either case

This completes the Rust library tutorial series for computation graphs. You've gone from a hand-called compiled function all the way to a fully reactive, multi-source, routed pipeline.

## Related resources

- [Python Tutorial 09 — Your First Computation Graph]({{< ref "/python/tutorials/computation-graphs/09-computation-graph/" >}}): the same concepts in Python using `ComputationGraphBuilder` and `@cloaca.node`
- [Python Tutorial 11 — Routing]({{< ref "/python/tutorials/computation-graphs/11-routing/" >}}): tuple-based routing in Python
