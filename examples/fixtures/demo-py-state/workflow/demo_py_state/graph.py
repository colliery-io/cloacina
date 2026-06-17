"""Demo Python state-accumulator computation graph (CLOACI-T-0688).

Showcases the Python `@cloaca.state_accumulator(capacity=N)` authoring surface
closed in T-0688 (Python previously had only passthrough/stream/polling/batch
accumulators — no state accumulator, which is the bounded-history primitive).

`py_window` is a *state* accumulator with `capacity=5`: every event pushed over
the socket is appended to a bounded `VecDeque`; once full, the oldest entry is
evicted, and the **full retained window** (newest ≤5 events) is emitted back as
the boundary on every write. The reactor fires the graph on each write; the
entry node receives the whole window (a list), not a single event — that rolling
window is the visible difference from `demo_py_graph`'s passthrough accumulator.

Fed by the demo `producer` over WS (HARNESS_WS_ACCUMULATORS includes `py_window`),
so the Accumulators + Graphs views show live activity.
"""
import cloaca


@cloaca.state_accumulator(capacity=5)
def py_window(event):
    # State accumulators buffer the returned value; the runtime emits the full
    # bounded window back as the boundary. Pass the market event through as-is.
    return event


@cloaca.reactor(name="demo_py_state_rx", accumulators=["py_window"], mode="when_any")
class _DemoStateReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "demo_py_state",
    reactor=_DemoStateReactor,
    graph={
        # Entry node consumes the accumulator's emitted window (a list), then a
        # linear edge to a terminal reporter — a minimal but real 2-node CG.
        "window": {"inputs": ["py_window"], "next": "report"},
        "report": {},
    },
) as builder:

    @cloaca.node
    def window(py_window):
        # `py_window` is the bounded rolling window (list of ≤5 recent events),
        # not a single event — that's the state-accumulator behaviour.
        history = py_window or []
        bids = [e.get("bid", 0.0) for e in history if isinstance(e, dict)]
        asks = [e.get("ask", 0.0) for e in history if isinstance(e, dict)]
        return {
            "window_size": len(history),
            "latest": history[-1] if history else None,
            "avg_bid": (sum(bids) / len(bids)) if bids else 0.0,
            "avg_ask": (sum(asks) / len(asks)) if asks else 0.0,
        }

    @cloaca.node
    def report(window):
        # `window` is the dict returned by the entry node (linear edge payload).
        return {
            "windowed_avg_bid": window.get("avg_bid", 0.0),
            "windowed_avg_ask": window.get("avg_ask", 0.0),
            "samples": window.get("window_size", 0),
        }
