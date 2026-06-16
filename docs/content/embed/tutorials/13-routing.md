---
title: "13 — Routing and Enum Dispatch"
description: "Route computation graph execution down conditional paths using enum variants and the => topology syntax"
weight: 23
aliases:
  - "/computation-graphs/tutorials/library/10-routing/"
  - "/python/computation-graphs/tutorials/11-routing/"

---

In this tutorial you'll build a market maker that routes each market event to one of two outcomes: execute a trade or log a no-action audit record. A decision node examines market data and the runtime dispatches each case to its dedicated handler. Only one branch runs per graph invocation.

Cloacina has full Rust/Python parity here. The difference is how the decision is expressed: Rust uses a routing **enum** (`DecisionOutcome::Trade(...)`) matched against the `=>` topology syntax, while Python uses a **tuple** (`("Trade", data)`) matched against the `"routes"` topology key. Both select exactly one downstream handler.

## What you'll learn

- The routing syntax in the topology declaration (`=>` in Rust, `"routes"` in Python)
- Defining the dispatch type whose variants carry data for downstream nodes
- Multiple terminal nodes — one per branch
- How the runtime matches each variant and routes data accordingly
- How input conditions determine which path executes

## Prerequisites

- Completion of [Tutorial 12 — Full Multi-Source Pipeline]({{< ref "/embed/tutorials/12-full-pipeline/" >}})

## The complete example

The full source lives at [`examples/tutorials/computation-graphs/library/10-routing`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/10-routing).

To run it:

```bash
angreal demos tutorials rust 10
```

> The on-disk example keeps its original number (`10`); these tutorials were renumbered, so the command number won't match the tutorial number — that's expected.

---

## Step 1: Define boundary types

You need a type for each branch of the routing decision. In Rust these are explicit structs; in Python the equivalent values are plain dicts, so there are no type declarations — the dict shapes are introduced inline with the nodes that produce and consume them.

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

In Rust, each variant of the routing enum carries a different payload type. That payload is what the downstream branch node receives.

---

## Step 2: Declare the routing graph

A routing node declares its branches instead of a single linear successor. In Rust the `=>` syntax replaces the `->` arrow and maps each enum variant name to its downstream handler. In Python the node uses a `"routes"` dict instead of `"next"`, mapping variant names to handler nodes.

{{< tabs "t13-topology" >}}
{{< tab "Rust" >}}
```rust
#[cloacina_macros::reactor(
    name = "market_maker_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct MarketMakerReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("market_maker_reactor"),
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
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca


# Declare the reactor that fires the graph (CLOACI-I-0101 split — the
# bundled `react={...}` kwarg was removed in favour of first-class
# `@cloaca.reactor` classes).
@cloaca.reactor(
    name="market_maker_reactor",
    accumulators=["orderbook", "pricing"],
    mode="when_any",
)
class MarketMakerReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "market_maker",
    reactor=MarketMakerReactor,
    graph={
        "decision": {
            "inputs": ["orderbook", "pricing"],
            "routes": {
                "Trade": "signal_handler",    # when decision returns ("Trade", ...)
                "NoAction": "audit_logger",   # when decision returns ("NoAction", ...)
            },
        },
        "signal_handler": {},   # terminal node on Trade branch
        "audit_logger": {},     # terminal node on NoAction branch
    },
) as builder:
```
{{< /tab >}}
{{< /tabs >}}

**How routing works:**

When the decision returns the `Trade` case, the runtime extracts its payload (a `TradeSignal` struct in Rust, the payload dict in Python) and passes it to `signal_handler`. When it returns the `NoAction` case, the payload goes to `audit_logger`. The variant name must exactly match a route key. Only one branch executes per graph invocation.

In Rust the routing enum lives inside the module and does **not** need `Serialize/Deserialize` — it is created, matched, and discarded within the graph execution without ever being stored in the cache.

---

## Step 3: The decision node returns a routed value

The decision node selects a branch by returning a tagged value: in Rust a `DecisionOutcome` enum variant carrying a struct payload, in Python a two-element tuple of the variant name (a string) and a payload dict.

{{< tabs "t13-decision" >}}
{{< tab "Rust" >}}
```rust
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
```
{{< /tab >}}
{{< tab "Python" >}}
```python
    @cloaca.node
    def decision(orderbook, pricing):
        """Decision engine: evaluate market data and decide whether to trade."""
        if orderbook is None:
            return ("NoAction", {"reason": "no order book data"})

        bid = orderbook["best_bid"]
        ask = orderbook["best_ask"]
        spread = ask - bid
        mid = (ask + bid) / 2.0
        pricing_mid = pricing["mid_price"] if pricing else mid

        price_diff = abs(mid - pricing_mid)

        if spread < 0.20 and price_diff < 0.50:
            direction = "BUY" if pricing_mid > mid else "SELL"
            return ("Trade", {
                "direction": direction,
                "price": mid,
                "confidence": 1.0 - (price_diff / mid),
            })
        else:
            reason = (
                f"spread too wide: {spread:.2f}"
                if spread >= 0.20
                else f"price divergence: {price_diff:.2f}"
            )
            return ("NoAction", {"reason": reason})
```
{{< /tab >}}
{{< /tabs >}}

The `Trade` case tells the runtime to send its payload to `signal_handler`; the `NoAction` case sends its payload to `audit_logger`. The variant name must exactly match a key in the routing declaration.

---

## Step 4: The branch handler nodes

Each handler receives the payload from the decision node as its sole argument — a borrowed struct in Rust (`&T`), the payload dict directly in Python. Only one handler runs per graph invocation.

{{< tabs "t13-handlers" >}}
{{< tab "Rust" >}}
```rust
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
```
{{< /tab >}}
{{< tab "Python" >}}
```python
    @cloaca.node
    def signal_handler(signal):
        """Execute the trade — terminal node on Trade path."""
        return {
            "executed": True,
            "message": f"{signal['direction']} @ {signal['price']:.2f} "
                       f"(confidence: {signal['confidence']:.4f})",
        }

    @cloaca.node
    def audit_logger(reason):
        """Log why no action was taken — terminal node on NoAction path."""
        return {
            "logged": True,
            "reason": reason["reason"],
        }
```
{{< /tab >}}
{{< /tabs >}}

`signal_handler` receives the trade payload (`direction`, `price`, `confidence`) from the `Trade` branch. `audit_logger` receives the `reason` payload from the `NoAction` branch.

---

## Step 5: Inspect the branch output

Because only one branch runs per execution, the result will contain either a trade confirmation or an audit record — never both.

In Rust, `GraphResult::Completed { outputs }` carries type-erased outputs, so you `downcast_ref` for each possible terminal type and handle whichever is present:

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

In Python the terminal handler's return dict is the result of `builder.execute()` directly — you simply read the dict, as shown in the scenarios below.

---

## Step 6: The accumulator and reactor wiring (Rust)

The wiring follows the same pattern as Tutorial 12 — two accumulators, one shared boundary channel. In Python this plumbing is handled by the reactor declaration and `builder.execute()`, so there is no separate wiring step.

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

## Step 7: Five scenarios, two branches

The same five scenarios exercise both branches. In Rust each event is pushed onto an accumulator channel and the graph fires reactively; in Python each call to `builder.execute()` supplies the inputs directly and returns the chosen handler's dict.

{{< tabs "t13-scenarios" >}}
{{< tab "Rust" >}}
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
{{< /tab >}}
{{< tab "Python" >}}
```python
# 1. Pricing only, no order book → NoAction
result = builder.execute({"pricing": {"mid_price": 100.05}})
# → {"logged": True, "reason": "no order book data"}

# 2. Tight spread (0.10) + confirmed pricing → Trade
result = builder.execute({
    "orderbook": {"best_bid": 100.00, "best_ask": 100.10},
    "pricing": {"mid_price": 100.05},
})
# → {"executed": True, "message": "BUY @ 100.05 (confidence: 0.9995)"}

# 3. Wide spread (1.00) → NoAction
result = builder.execute({
    "orderbook": {"best_bid": 99.50, "best_ask": 100.50},
    "pricing": {"mid_price": 100.00},
})
# → {"logged": True, "reason": "spread too wide: 1.00"}

# 4. Tight spread, divergent pricing → NoAction
result = builder.execute({
    "orderbook": {"best_bid": 100.00, "best_ask": 100.10},
    "pricing": {"mid_price": 105.00},
})
# → {"logged": True, "reason": "price divergence: 4.95"}

# 5. Everything aligned → Trade
result = builder.execute({
    "orderbook": {"best_bid": 102.00, "best_ask": 102.08},
    "pricing": {"mid_price": 102.05},
})
# → {"executed": True, "message": "BUY @ 102.04 (confidence: 0.9995)"}
```
{{< /tab >}}
{{< /tabs >}}

---

## Expected output

{{< tabs "t13-output" >}}
{{< tab "Rust" >}}
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
{{< /tab >}}
{{< tab "Python" >}}
```
=== Python Tutorial 11: Routing and Conditional Paths ===

1. Pricing only (no order book):
   Result: {'logged': True, 'reason': 'no order book data'}

2. Tight spread (0.10) + confirmed pricing:
   Result: {'executed': True, 'message': 'BUY @ 100.05 (confidence: 0.9995)'}

3. Wide spread (1.00):
   Result: {'logged': True, 'reason': 'spread too wide: 1.00'}

4. Tight spread but divergent pricing:
   Result: {'logged': True, 'reason': 'price divergence: 4.95'}

5. Aligned data (tight spread + confirmed):
   Result: {'executed': True, 'message': 'BUY @ 102.04 (confidence: 0.9995)'}

=== Tutorial 11 Complete ===
```
{{< /tab >}}
{{< /tabs >}}

---

## Routing at a glance: Rust vs Python

| Concept | Rust | Python |
|---|---|---|
| Routing syntax | `=>` in topology | `"routes": {...}` in topology dict |
| Dispatch type | `enum DecisionOutcome { Trade(T), NoAction(U) }` | `("Trade", dict)` / `("NoAction", dict)` |
| Branch node receives | `&TradeSignal` / `&NoActionReason` | the payload dict directly |
| Terminal result | `output.downcast_ref::<TradeConfirmation>()` | return dict from the handler |
| Variant name | Rust enum variant name | string key in `"routes"` dict |

---

## Summary

You've implemented a routed computation graph:

- The routing syntax (`=>` in Rust, `"routes"` in Python) declares conditional dispatch
- The dispatch value's variant names must match the route keys
- Each variant carries the payload that its downstream node receives
- Only one branch executes per graph invocation
- In Rust you `downcast_ref` for each possible terminal type; in Python the terminal handler's return dict is the result of `execute()`

This completes the computation graph tutorial track. You've gone from a single-path graph all the way to a fully event-driven, multi-source, routed pipeline in both languages.

## Next

- Reference: [Computation Graphs]({{< ref "/engine/computation-graphs" >}})
- How-to guides: [Embedded how-to]({{< ref "/embed/how-to" >}})
