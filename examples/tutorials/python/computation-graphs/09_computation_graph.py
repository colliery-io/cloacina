#!/usr/bin/env python3
"""
Cloacina Python Tutorial 09: Your First Computation Graph

This tutorial mirrors Rust Tutorial 07. Define a computation graph using
the ComputationGraphBuilder context manager and @cloaca.node decorator,
then execute it.

Learning objectives:
- ComputationGraphBuilder context manager
- @cloaca.node decorator for defining graph nodes
- Graph topology declaration via Python dict
- Executing a computation graph and inspecting results

Prerequisites:
    pip install cloaca
"""

import cloaca


# Define the computation graph
with cloaca.ComputationGraphBuilder(
    "pricing_pipeline",
    react={"mode": "when_any", "accumulators": ["orderbook"]},
    graph={
        "ingest": {
            "inputs": ["orderbook"],
            "target": "compute_spread",
        },
        "compute_spread": {
            "target": "format_output",
        },
        "format_output": {},
    },
) as builder:

    @cloaca.node
    def ingest(orderbook):
        """Entry node: extract key fields from order book."""
        if orderbook is None:
            return {"spread": 0.0, "mid_price": 0.0}
        spread = orderbook["best_ask"] - orderbook["best_bid"]
        mid_price = (orderbook["best_ask"] + orderbook["best_bid"]) / 2.0
        return {"spread": spread, "mid_price": mid_price}

    @cloaca.node
    def compute_spread(input_data):
        """Processing node: compute spread in basis points."""
        mid = input_data["mid_price"]
        if mid == 0:
            return input_data
        spread_bps = (input_data["spread"] / mid) * 10_000
        return {"spread_bps": spread_bps, "mid_price": mid}

    @cloaca.node
    def format_output(input_data):
        """Terminal node: format for display."""
        return {
            "message": f"Mid: {input_data['mid_price']:.2f}, Spread: {input_data['spread_bps']:.1f} bps",
            "mid_price": input_data["mid_price"],
            "spread_bps": input_data["spread_bps"],
        }


print("=== Python Tutorial 09: Your First Computation Graph ===\n")

# Execute the graph with input data
orderbook = {"best_bid": 100.50, "best_ask": 100.55}
print(f"Input: {orderbook}\n")

result = builder.execute({"orderbook": orderbook})
print(f"Result: {result}")
print(f"  Message: {result.get('message', 'N/A')}")
print(f"  Mid price: {result.get('mid_price', 'N/A')}")
print(f"  Spread: {result.get('spread_bps', 'N/A')} bps")

print("\n=== Tutorial 09 Complete ===")
