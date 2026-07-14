"""Python packaged STATE-ACCUMULATOR computation graph — the stateful peer of
`python-packaged-graph` (passthrough).

`tick_window` is a `@cloaca.state_accumulator(capacity=5)`: each event pushed in
is appended to a bounded window; once full the oldest is evicted, and the FULL
retained window (newest ≤5 events) is emitted as the boundary on every write.
The reactor fires the graph on each write, and the entry node receives the whole
window (a LIST), not a single event — that rolling window is the visible
difference from a passthrough accumulator.

`@cloaca.boundary_schema(bid=float, ask=float)` declares the accumulator's typed
boundary, so the server validates injected/fired events against it before they
reach the graph (a non-conforming event is rejected — the typed-inject surface).
"""
import cloaca


# Typed, bounded rolling window. The boundary schema types each injected tick;
# the state accumulator buffers them and emits the retained window.
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.state_accumulator(capacity=5)
def tick_window(event):
    # State accumulators buffer the returned value; the runtime emits the full
    # bounded window back as the boundary. Pass the tick through as-is.
    return event


@cloaca.reactor(name="tick_reactor", accumulators=["tick_window"], mode="when_any")
class _TickReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "tick_window_py",
    reactor=_TickReactor,
    graph={
        # Entry node consumes the accumulator's emitted window (a list), then a
        # linear edge to a terminal reporter — a minimal but real 2-node CG.
        "aggregate": {"inputs": ["tick_window"], "next": "report"},
        "report": {},
    },
) as builder:

    @cloaca.node
    def aggregate(tick_window):
        # `tick_window` is the bounded rolling window (list of ≤5 recent ticks),
        # not a single event — that's the state-accumulator behaviour.
        history = tick_window or []
        bids = [e.get("bid", 0.0) for e in history if isinstance(e, dict)]
        asks = [e.get("ask", 0.0) for e in history if isinstance(e, dict)]
        return {
            "window_size": len(history),
            "latest": history[-1] if history else None,
            "avg_bid": (sum(bids) / len(bids)) if bids else 0.0,
            "avg_ask": (sum(asks) / len(asks)) if asks else 0.0,
        }

    @cloaca.node
    def report(aggregate):
        # `aggregate` is the dict returned by the entry node (linear-edge payload).
        return {
            "windowed_avg_bid": aggregate.get("avg_bid", 0.0),
            "windowed_avg_ask": aggregate.get("avg_ask", 0.0),
            "samples": aggregate.get("window_size", 0),
        }
