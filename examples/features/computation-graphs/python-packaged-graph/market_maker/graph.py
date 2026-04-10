"""
Market maker computation graph — packaged Python example.

This module defines a computation graph using the cloaca decorator pattern.
When imported by the reconciler, the decorators fire and register the graph
in the global Python executor registry.
"""

import cloaca


# Define accumulators
@cloaca.passthrough_accumulator
def orderbook(event):
    """Pass through order book events."""
    return event


@cloaca.passthrough_accumulator
def pricing(event):
    """Pass through pricing events."""
    return event


# Define the computation graph
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
        """Decision engine: evaluate market data."""
        if orderbook is None:
            return ("NoAction", {"reason": "no order book data"})

        bid = orderbook.get("best_bid", 0)
        ask = orderbook.get("best_ask", 0)
        spread = ask - bid
        mid = (ask + bid) / 2.0
        pricing_mid = pricing.get("mid_price", mid) if pricing else mid

        price_diff = abs(mid - pricing_mid)

        if spread < 0.20 and price_diff < 0.50:
            direction = "BUY" if pricing_mid > mid else "SELL"
            return ("Trade", {
                "direction": direction,
                "price": mid,
                "confidence": 1.0 - (price_diff / mid) if mid > 0 else 0,
            })
        else:
            reason = f"spread too wide: {spread:.2f}" if spread >= 0.20 else f"price divergence: {price_diff:.2f}"
            return ("NoAction", {"reason": reason})

    @cloaca.node
    def signal_handler(signal):
        """Execute the trade."""
        return {
            "executed": True,
            "message": f"{signal['direction']} @ {signal['price']:.2f} (confidence: {signal['confidence']:.4f})",
        }

    @cloaca.node
    def audit_logger(reason):
        """Log why no action was taken."""
        return {
            "logged": True,
            "reason": reason["reason"],
        }
