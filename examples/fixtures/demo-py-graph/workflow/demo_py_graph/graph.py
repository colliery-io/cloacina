"""Demo Python computation graph (CLOACI-I-0117 / T-0664).

A reactor + passthrough accumulator + a small market-maker computation graph,
defined with the cloaca decorators. The reconciler imports this module at load
time; the decorators register the reactor/accumulator/graph. Content mirrors the
proven soak-server Python CG (.angreal/test/soak/server.py).
"""
import cloaca


@cloaca.passthrough_accumulator
def py_alpha(event):
    return event


@cloaca.reactor(name="demo_py_graph_rx", accumulators=["py_alpha"], mode="when_any")
class _DemoReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "demo_py_graph",
    reactor=_DemoReactor,
    graph={
        "decision": {
            "inputs": ["py_alpha"],
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
    def decision(py_alpha):
        if py_alpha is None:
            return ("NoAction", {"reason": "no data"})
        bid = py_alpha.get("bid", 0.0)
        ask = py_alpha.get("ask", 0.0)
        spread = ask - bid
        if spread < 1.0:
            mid = (bid + ask) / 2.0
            return ("Trade", {"direction": "BUY", "price": mid})
        return ("NoAction", {"reason": f"spread too wide: {spread:.2f}"})

    @cloaca.node
    def signal_handler(signal):
        return {
            "executed": True,
            "direction": signal.get("direction", "?"),
            "price": signal.get("price", 0.0),
        }

    @cloaca.node
    def audit_logger(reason):
        return {"logged": True, "reason": reason.get("reason", "unknown")}
