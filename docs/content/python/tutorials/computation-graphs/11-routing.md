---
title: "11 - Routing and Conditional Paths"
description: "Route graph execution down conditional branches using tuple returns and the routes topology key"
weight: 30
---

In this tutorial you'll add routing to your computation graph. A decision node examines market data and returns a tuple — `("Trade", data)` or `("NoAction", data)` — and the runtime dispatches each case to its dedicated handler. This mirrors [Rust Tutorial 10]({{< ref "/computation-graphs/tutorials/library/10-routing/" >}}) but uses Python's tuple-based dispatch instead of Rust enums.

## What you'll learn

- The `"routes"` key in the topology dict — declaring conditional paths
- Tuple returns for dispatch: `("VariantName", payload_dict)`
- Multiple terminal nodes — one per branch
- How input conditions determine which path executes
- Reading branch-specific output from `builder.execute()`

## Prerequisites

- Completion of [Tutorial 10 — Accumulators]({{< ref "/python/tutorials/computation-graphs/10-accumulators/" >}})

## The complete example

The full source lives at [`examples/tutorials/python/computation-graphs/11_routing.py`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/python/computation-graphs/11_routing.py).

To run it:

```bash
python examples/tutorials/python/computation-graphs/11_routing.py
```

---

## Step 1: Declare a routing topology

Instead of `"next"`, a routing node uses `"routes"` to map variant names to downstream handlers.

```python
import cloaca

with cloaca.ComputationGraphBuilder(
    "market_maker",
    react={"mode": "when_any", "accumulators": ["orderbook", "pricing"]},
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

Comparing with the linear topology from Tutorial 09:

| Linear | Routing |
|---|---|
| `"next": "next_node"` | `"routes": {"Variant": "handler_node"}` |
| One path always taken | One of N paths chosen at runtime |
| Return type: dict | Return type: `("VariantName", dict)` tuple |

---

## Step 2: The decision node returns a tuple

A routing node returns a two-element tuple: the variant name (a string) and the payload dict for the chosen branch.

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

The tuple `("Trade", {...})` tells the runtime to send the payload dict to `signal_handler`. The tuple `("NoAction", {...})` sends its payload to `audit_logger`. The variant string must exactly match a key in the `"routes"` dict.

---

## Step 3: The branch handler nodes

Each handler receives the payload dict from the decision node as its sole argument.

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

`signal_handler` receives the `{"direction", "price", "confidence"}` dict from the `Trade` branch. `audit_logger` receives the `{"reason"}` dict from the `NoAction` branch. Only one handler runs per `execute()` call.

---

## Step 4: Five scenarios

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

---

## Expected output

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

---

## Comparing Python and Rust routing

| Concept | Rust | Python |
|---|---|---|
| Routing syntax | `=>` in topology | `"routes": {...}` in topology dict |
| Dispatch type | `enum DecisionOutcome { Trade(T), NoAction(U) }` | `("Trade", dict)` / `("NoAction", dict)` |
| Branch node receives | `&TradeSignal` / `&NoActionReason` | the payload dict directly |
| Terminal result | `output.downcast_ref::<TradeConfirmation>()` | return dict from the handler |
| Variant name | Rust enum variant name | string key in `"routes"` dict |

---

## Summary

You've added conditional routing to your Python computation graph:

- `"routes"` in the topology dict replaces `"next"` for routing nodes
- A routing node returns `("VariantName", payload_dict)` — the variant selects the branch, the payload is the handler's input
- Only one branch executes per `execute()` call
- The result dict comes from whichever terminal handler ran

This completes the Python computation graph tutorial series. You've gone from a simple single-path graph all the way to a multi-source, conditionally routed pipeline.

## Related resources

- [Rust Tutorial 10 — Routing]({{< ref "/computation-graphs/tutorials/library/10-routing/" >}}): the same pattern in Rust using enum dispatch
- [Python Tutorial 09 — Your First Computation Graph]({{< ref "/python/tutorials/computation-graphs/09-computation-graph/" >}})
- [Python Tutorial 10 — Accumulators]({{< ref "/python/tutorials/computation-graphs/10-accumulators/" >}})
