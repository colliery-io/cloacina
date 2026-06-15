---
title: "Accumulator"
description: "Turns an external source or stream into the boundary events a reactor reacts to."
weight: 24
---

# Accumulator

An **Accumulator** sits between an external data source and a
[Reactor]({{< ref "/engine/computation-graphs/reactor" >}}). It receives raw
events, optionally aggregates or filters them, and emits typed
[Boundary events]({{< ref "/engine/computation-graphs/boundary" >}}) under a named
source. Its core operation is `process(event) -> Option<output>` — returning
nothing drops the event (filtering/dedup).

## Variants

| Variant | What it does |
|---------|--------------|
| **passthrough** | No buffering; converts each event to an output (or drops it). |
| **stream** | Consumes from a stream backend (e.g. Kafka `topic`/`group`). |
| **polling** | Pulls from a source on a timer (`interval`). |
| **batch** | Buffers events, flushes on `flush_interval` / `max_buffer_size`. |
| **state** | A bounded, DAL-persisted history buffer (`capacity`). **Rust-only.** |

## Interfaces

{{< tabs "accumulator-variants" >}}
{{< tab "Rust" >}}
```rust
#[passthrough_accumulator]
#[stream_accumulator(type = "...", topic = "...", group = "...")]
#[polling_accumulator(interval = "...")]
#[batch_accumulator(flush_interval = "...", max_buffer_size = 100)]
#[state_accumulator(capacity = 64)]   // Rust-only
```
{{< /tab >}}
{{< tab "Python" >}}
```python
@cloaca.passthrough_accumulator
@cloaca.stream_accumulator(type="...", topic="...", group="...")
@cloaca.polling_accumulator(interval="5s")
@cloaca.batch_accumulator(flush_interval="1s", max_buffer_size=100)
```

{{< hint type=warning title="Parity gap" >}}
Python exposes **four** accumulator types. The Rust-only `#[state_accumulator]`
(a bounded DAL-persisted history buffer) has **no Python decorator** yet — tracked
in [CLOACI-T-0688]. Until it lands, author state accumulators in Rust.
{{< /hint >}}
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **`process()` returns an `Option`** — `None` silently drops the event.
- **Named source:** an accumulator emits under a `SourceName` that must match the
  reactor's `accumulators` list and the graph's entry-node source name.
- **Choosing one:** see [Choosing Accumulator Types]({{< ref "/computation-graphs/how-to-guides/accumulator-types" >}}).

## See also

- [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) · [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}}) · [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}})
