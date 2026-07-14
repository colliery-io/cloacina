"""Python packaged BATCH-ACCUMULATOR computation graph.

`event_batch` is a `@cloaca.batch_accumulator(flush_interval="1s",
max_buffer_size=5)`: each injected event is buffered; the WHOLE buffer is emitted
as one boundary when the buffer fills (5) OR the flush interval (1s) elapses,
whichever comes first. Unlike a state accumulator (a rolling window emitted on
every write), a batch emits only on flush — the entry node receives the flushed
batch (a LIST) once per flush.

`@cloaca.boundary_schema(level=float)` types each injected event, so the server
validates injected/fired events before they reach the graph (a non-conforming
event is rejected — the typed-inject surface).
"""
import cloaca


# Typed batch: buffer typed events, flush the whole buffer on size or interval.
@cloaca.boundary_schema(level=float)
@cloaca.batch_accumulator(flush_interval="1s", max_buffer_size=5)
def event_batch(event):
    # Batch accumulators buffer the returned value; the runtime emits the whole
    # buffer back as the boundary on flush. Pass the event through as-is.
    return event


@cloaca.reactor(name="batch_reactor", accumulators=["event_batch"], mode="when_any")
class _BatchReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "event_batch_py",
    reactor=_BatchReactor,
    graph={
        # Entry node consumes the flushed batch (a list), then a linear edge to a
        # terminal reporter — a minimal but real 2-node CG.
        "summarize": {"inputs": ["event_batch"], "next": "report"},
        "report": {},
    },
) as builder:

    @cloaca.node
    def summarize(event_batch):
        # `event_batch` is the flushed batch (list of buffered events), not a
        # single event — that's the batch-accumulator behaviour.
        batch = event_batch or []
        levels = [e.get("level", 0.0) for e in batch if isinstance(e, dict)]
        return {
            "batch_size": len(batch),
            "total": sum(levels),
            "peak": max(levels) if levels else 0.0,
        }

    @cloaca.node
    def report(summarize):
        # `summarize` is the dict returned by the entry node (linear-edge payload).
        return {
            "flushed": summarize.get("batch_size", 0),
            "sum_level": summarize.get("total", 0.0),
            "peak_level": summarize.get("peak", 0.0),
        }
