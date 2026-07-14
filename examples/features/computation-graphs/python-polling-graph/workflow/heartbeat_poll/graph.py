"""Python packaged POLLING-ACCUMULATOR computation graph.

`heartbeat` is a `@cloaca.polling_accumulator(interval="2s")`: unlike the
event-driven accumulators (passthrough/state/batch), a polling accumulator is
SELF-FIRING — the runtime calls its poll function on the interval and emits
whatever it returns (return `None` to skip a tick). No inject/socket is needed;
the reactor fires on its own each interval, and the entry node receives the
polled value.

On the packaged path this works because the reconciler imports this module in
the server process, so the poll function is invoked in-process on the interval
(spawn_blocking + GIL, exactly like a Python poll trigger) — no FFI (T-0896).
"""
import cloaca

# Module-level counter so each poll returns a distinct value — makes the
# self-firing visible (every tick emits a fresh boundary).
_ticks = {"n": 0}


@cloaca.polling_accumulator(interval="2s")
def heartbeat():
    """Poll function: called on each interval. Returns the event to emit as the
    boundary, or None to skip this tick."""
    _ticks["n"] += 1
    return {"tick": _ticks["n"], "source": "poller"}


@cloaca.reactor(name="poll_reactor", accumulators=["heartbeat"], mode="when_any")
class _PollReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "heartbeat_poll_py",
    reactor=_PollReactor,
    graph={
        # Entry node consumes the polled value, then a linear edge to a terminal
        # reporter — a minimal but real 2-node CG driven purely by the poller.
        "observe": {"inputs": ["heartbeat"], "next": "report"},
        "report": {},
    },
) as builder:

    @cloaca.node
    def observe(heartbeat):
        # `heartbeat` is the value returned by the poll fn this tick.
        beat = heartbeat or {}
        return {"tick": beat.get("tick", 0), "source": beat.get("source")}

    @cloaca.node
    def report(observe):
        # `observe` is the dict returned by the entry node (linear-edge payload).
        return {"observed_tick": observe.get("tick", 0)}
