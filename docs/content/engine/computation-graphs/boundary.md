---
title: "Boundary event"
description: "The typed event an accumulator emits and a reactor reacts to."
weight: 25
---

# Boundary event

A **Boundary event** is the typed value an [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}})
emits and a [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) reacts to.
It is **coupled** to both — a boundary only exists *between* an accumulator and a
reactor; it has no standalone lifecycle.

## Mental model

- An accumulator's `process()` produces a boundary **output**, tagged with a
  **`SourceName`** and sent to the reactor.
- The reactor slots it into its **input cache** under that source name; the cache
  is what the [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}})
  reads when it fires.
- The boundary value is **serialized** (bytes) as it crosses the channel.

## Key facts

- **Names must line up.** The boundary's `SourceName` must match the reactor's
  `accumulators = [...]` / `mode` sources *and* the graph's entry-node source name.
  This is the most common wiring mistake.
- **One per emit.** Each `process()` that returns a value produces one boundary;
  returning `None` produces none.

## See also

- [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) — emits boundaries.
- [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) — reacts to them.
