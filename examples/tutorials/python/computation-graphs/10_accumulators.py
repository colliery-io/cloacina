#!/usr/bin/env python3
"""
Cloacina Python Tutorial 10: Accumulators

This tutorial mirrors Rust Tutorial 08. Define a passthrough accumulator
using the @cloaca.passthrough_accumulator decorator and wire it into a
computation graph.

Learning objectives:
- @cloaca.passthrough_accumulator decorator
- Accumulator registration alongside graph nodes
- How accumulators transform events before feeding the graph

Prerequisites:
    pip install cloaca
"""

import cloaca


# Define a passthrough accumulator
@cloaca.passthrough_accumulator
def pricing(event):
    """Transform a raw pricing event into a pricing signal."""
    return {"price": event["mid_price"], "change_pct": 0.0}


# Define the computation graph
with cloaca.ComputationGraphBuilder(
    "pricing_graph",
    react={"mode": "when_any", "accumulators": ["pricing"]},
    graph={
        "ingest": {
            "inputs": ["pricing"],
            "target": "analyze",
        },
        "analyze": {
            "target": "format_signal",
        },
        "format_signal": {},
    },
) as builder:

    @cloaca.node
    def ingest(pricing):
        """Entry node: receive pricing data from accumulator."""
        if pricing is None:
            return {"price": 0.0, "change_pct": 0.0}
        return pricing

    @cloaca.node
    def analyze(input_data):
        """Analyze pricing for large moves."""
        price = input_data["price"]
        change_pct = ((price - 100.0) / 100.0) * 100.0 if price > 100.0 else 0.0
        return {"price": price, "change_pct": change_pct}

    @cloaca.node
    def format_signal(input_data):
        """Terminal node: format the signal."""
        return {
            "message": f"Price: {input_data['price']:.2f}, Change: {input_data['change_pct']:.2f}%",
        }


print("=== Python Tutorial 10: Accumulators ===\n")

# Simulate pushing events through the accumulator → graph pipeline
events = [
    {"mid_price": 99.50},
    {"mid_price": 101.25},
    {"mid_price": 103.75},
]

for i, event in enumerate(events, 1):
    print(f"Event {i}: {event}")
    # Simulate accumulator processing
    processed = pricing(event)
    print(f"  Accumulator output: {processed}")
    # Execute graph with processed data
    result = builder.execute({"pricing": processed})
    print(f"  Graph result: {result.get('message', 'N/A')}\n")

print("=== Tutorial 10 Complete ===")
