#!/usr/bin/env python3
"""
Cloacina Python Tutorial 11: Routing and Conditional Paths

This tutorial mirrors Rust Tutorial 10. Define a computation graph with
enum-style routing using tuple returns: ("VariantName", data).

Learning objectives:
- Routing topology declaration in Python
- Tuple returns for enum dispatch: ("Signal", data) / ("NoAction", data)
- Multiple terminal paths from one decision node
- How input values determine which path executes

Prerequisites:
    pip install cloaca
"""

import cloaca


# Define the computation graph with routing
with cloaca.ComputationGraphBuilder(
    "market_maker",
    react={"mode": "when_any", "accumulators": ["orderbook", "pricing"]},
    graph={
        "decision": {
            "inputs": ["orderbook", "pricing"],
            "routes": {
                "Trade": "signal_handler",
                "NoAction": "audit_logger",
            },
        },
        "signal_handler": {},
        "audit_logger": {},
    },
) as builder:

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
            reason = f"spread too wide: {spread:.2f}" if spread >= 0.20 else f"price divergence: {price_diff:.2f}"
            return ("NoAction", {"reason": reason})

    @cloaca.node
    def signal_handler(signal):
        """Execute the trade (terminal node on Trade path)."""
        return {
            "executed": True,
            "message": f"{signal['direction']} @ {signal['price']:.2f} (confidence: {signal['confidence']:.4f})",
        }

    @cloaca.node
    def audit_logger(reason):
        """Log why no action was taken (terminal node on NoAction path)."""
        return {
            "logged": True,
            "reason": reason["reason"],
        }


print("=== Python Tutorial 11: Routing and Conditional Paths ===\n")

# Scenario 1: No order book → NoAction
print("1. Pricing only (no order book):")
result = builder.execute({"pricing": {"mid_price": 100.05}})
print(f"   Result: {result}\n")

# Scenario 2: Tight spread + confirmed → Trade
print("2. Tight spread (0.10) + confirmed pricing:")
result = builder.execute({
    "orderbook": {"best_bid": 100.00, "best_ask": 100.10},
    "pricing": {"mid_price": 100.05},
})
print(f"   Result: {result}\n")

# Scenario 3: Wide spread → NoAction
print("3. Wide spread (1.00):")
result = builder.execute({
    "orderbook": {"best_bid": 99.50, "best_ask": 100.50},
    "pricing": {"mid_price": 100.00},
})
print(f"   Result: {result}\n")

# Scenario 4: Tight spread, divergent pricing → NoAction
print("4. Tight spread but divergent pricing:")
result = builder.execute({
    "orderbook": {"best_bid": 100.00, "best_ask": 100.10},
    "pricing": {"mid_price": 105.00},
})
print(f"   Result: {result}\n")

# Scenario 5: Everything aligned → Trade
print("5. Aligned data (tight spread + confirmed):")
result = builder.execute({
    "orderbook": {"best_bid": 102.00, "best_ask": 102.08},
    "pricing": {"mid_price": 102.05},
})
print(f"   Result: {result}\n")

print("=== Tutorial 11 Complete ===")
