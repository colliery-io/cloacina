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

## Typed inject/fire interface (opt-in)

The boundary type is also what an operator supplies when manually injecting into
an accumulator or firing a reactor with inputs (`POST .../inject`, `fire_with`).
The declared interface — `GET /v1/health/{reactors|accumulators}/{name}/interface`
— derives each slot's JSON Schema **from the boundary type, but only when that
type derives `schemars::JsonSchema`**:

```rust
// Opt in → the slot exposes a rich {best_bid, best_ask} schema and the web UI
// renders a typed inject/fire form. Without JsonSchema the slot schema is `{}`
// (permissive) and the UI falls back to a raw-JSON field.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OrderBookUpdate {
    pub best_bid: f64,
    pub best_ask: f64,
}
```

Add `schemars = "0.8"` to the package's dependencies. Schema derivation is the
only thing the derive affects — boundary serialization on the wire is unchanged.

### Python parity

Python boundaries are plain dicts, so there's no type to derive from. Declare the
shape explicitly with `@cloaca.boundary_schema(field=type, …)` on the accumulator
(CLOACI-T-0770) — the compiler parses it from source at build time into the same
typed slot; at runtime it's a no-op:

```python
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.passthrough_accumulator
def py_alpha(event):
    return event
```

Supported scalar types: `str`, `int`, `float`, `bool`, `list`, `dict`. Without
the decorator the slot stays untyped and inject/fire fall back to a raw-JSON
field — exactly like a Rust boundary that doesn't derive `JsonSchema`.

## See also

- [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) — emits boundaries.
- [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) — reacts to them.
