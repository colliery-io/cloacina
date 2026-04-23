---
title: "Computation Graph Architecture"
weight: 10
---

# Computation Graph Architecture

Cloacina's computation graph system is a graph scheduling engine built alongside — but architecturally independent from — the existing cron/trigger workflow scheduler. Where workflows are database-backed, horizontally scalable, and claim-based, computation graphs are event-driven, in-process, and compiled. This document explains the model, why it exists, and how the pieces fit together.

## Why a Separate System?

The unified scheduler (cron + triggers) is designed for workloads where tasks are discrete, retryable units that can be distributed across many worker processes. It uses database claims, heartbeats, and polling as its coordination primitives. These characteristics are exactly what make it unsuitable for reactive workloads.

Consider a market-making system with three real-time data feeds: an order book, a pricing model, and a running exposure tracker. Each feed produces events continuously. A decision engine needs the latest value from all three every time any one changes. The decision does not fit the cron/trigger model:

- There is no "claim" to make — the work is triggered by data, not a schedule
- There is no discrete "job" — the reactor runs forever
- Latency matters — a 10ms database polling floor is unacceptable
- All state needs to be correlated at decision time — not spread across separate task executions

The computation graph system was designed specifically for this pattern: **multiple independent event streams, correlated into a single decision snapshot, executed as one compiled function**.

## The Reactive Model

The core model is: accumulators feed a cached input snapshot, reaction criteria decide when to fire, and a compiled graph function runs to completion.

```
external source
      │
      ▼
  accumulator              accumulator              accumulator
  (long-lived task)        (long-lived task)        (long-lived task)
      │                         │                         │
      └────────────[boundary channel]───────────────────┘
                                │
                                ▼
                            reactor
                         (long-lived task)
                         ┌──────────────┐
                         │ input cache  │
                         │ dirty flags  │
                         │ criteria     │
                         └──────┬───────┘
                                │ (criteria met)
                                ▼
                      graph_fn(cache snapshot)
                      ─────────────────────────
                      compiled async function
                      runs to completion
```

Everything in this diagram except the graph function is a long-lived tokio task. Nothing polls a database. The only coordination state is the cache (last-seen value per source) and a dirty flag per source.

## Process Model

The graph scheduler manages three kinds of long-lived processes:

**Accumulators** — one per data source. Each runs its own event loop, consuming from its backend (Kafka topic, socket, Postgres, push channel). When an event arrives, the accumulator's `process()` function is called. If it returns `Some(boundary)`, the boundary is serialized and sent to the reactor over a tokio mpsc channel. Accumulators are independent — they do not know about each other, and they do not know about the graph.

**Reactors** — one per computation graph. The reactor owns the input cache and the dirty flags. It receives boundaries from all its accumulators, updates the cache, marks the corresponding source dirty, and checks whether reaction criteria are met. If they are, it calls the graph function with a snapshot of the current cache, then clears all dirty flags.

**Compiled graph functions** — not processes. The `#[computation_graph]` macro resolves the node dependency graph at compile time and emits a single async function with nested match arms. The reactor calls it as a normal async function call. When it returns, the graph is done. There is no runtime graph walker.

This separation is intentional. The reactor's job is trivial — maintain a cache, check flags, call a function. The complexity of routing lives in the macro-generated code, not in any runtime component.

## Event Flow

Following a single event through the full pipeline:

1. An external source emits data (Kafka produces a message, a WebSocket push arrives, a timer fires)
2. The accumulator's event loop receives it and calls `process()` — a user-defined function that transforms raw events into typed boundary values
3. `process()` returns `Some(boundary)` if the event produces output (or `None` to suppress it)
4. The accumulator serializes the boundary and sends it to the reactor over the boundary channel
5. The reactor's receive loop takes the `(SourceName, Vec<u8>)` pair from the channel, updates the cache for that source, and marks the source dirty
6. The reactor checks its reaction criteria (`when_any` = any dirty flag set, `when_all` = all dirty flags set)
7. If criteria are met: the reactor snapshots the cache, clears all dirty flags, and calls the compiled graph function
8. The graph function executes — potentially firing multiple nodes in sequence — and returns `GraphResult`
9. The reactor persists the updated cache to the DAL and signals any batch accumulators to flush

The boundary channel is the only inter-task communication. Accumulators push; the reactor receives. This is a one-directional message-passing architecture with no shared mutable state between producers and consumer.

## The Reactor Loop

The reactor runs a `tokio::select!` loop with three arms:

```
loop {
    select! {
        boundary = accumulator_channel.recv() => {
            cache.update(source, boundary);
            dirty.set(source, true);
            if criteria.met(&dirty) && !paused {
                let snapshot = cache.snapshot();
                dirty.clear_all();
                graph_fn(snapshot).await;
                persist_cache();
                flush_batch_accumulators();
            }
        }

        command = manual_channel.recv() => {
            // ForceFire: execute with current cache
            // FireWith(injected): replace cache, execute
        }

        _ = shutdown.recv() => break,
    }
}
```

The reactor is single-threaded within its loop — it does not execute the graph concurrently with receiving boundaries. This is deliberate: the `latest` input strategy collapses concurrent updates into one slot per source, so intermediate values that arrive during graph execution are captured for the next fire. Boundaries are not lost; they update the cache. This is the correct behavior for reactive workloads where stale intermediate states have no value.

For workloads where every boundary must produce exactly one graph execution, the `sequential` input strategy preserves order.

## How Graphs Differ from Workflows

| Dimension | Computation Graph | Workflow |
|-----------|------------------|----------|
| Execution unit | Compiled async function | Runtime-walked DAG |
| Trigger | Boundary arrival + reaction criteria | Cron schedule or trigger rules |
| State | Cache + dirty flags (in-memory, persisted) | Context object (database-backed) |
| Routing | Rust enums, compiled to match arms | Task dependencies, resolved at runtime |
| Lifecycle | Long-lived reactor + accumulators | Per-execution job |
| Scaling | Per-graph process, not distributed | Horizontally scalable via claim semantics |
| Latency floor | Channel hops (~1-10ms) | Database polling (varies) |
| Recovery unit | Cache snapshot (restore and continue) | Task retry (re-execute failed step) |

The ontology is deliberately distinct. A "node" in a computation graph is not a "task" in a workflow. A "reactor" is not a "trigger". These are different concepts serving different workloads, and the naming reflects that.

## Where the Graph Scheduler Lives

The graph scheduler runs inside the API server process (Postgres-only). It is loaded by the reconciler when packages containing computation graphs are registered. The same API server hosts:

- The unified scheduler (cron + triggers, database-backed, horizontally scalable)
- The graph scheduler (computation graphs, event-driven, per-graph processes)
- The WebSocket layer (auth, accumulator and reactor endpoints)
- The shared DAL (Postgres)

The two schedulers share the same tokio runtime and DAL but have no other coupling. A detector running on the unified scheduler can write to an accumulator via the API server WebSocket — the same path as any external producer, with the same auth — but the schedulers themselves are architecturally separate.

## Recovery

On process crash, the reactor restores its cache from the DAL and is immediately operational with the last known state. Accumulators restart independently: each loads its checkpoint (last committed offset for Kafka-backed accumulators, persisted state for stateful ones) and progressively freshen the cache with new data. The reactor does not wait for accumulators to "catch up" before processing new boundaries — it uses the restored cache as the starting point and updates it as boundaries arrive.

The graph execution is the unit of recovery, not individual nodes. If the process crashes during a graph execution, the reactor will re-execute from the cached state when it restarts. Nodes that write to external systems must be idempotent to handle this correctly.

For more details on the recovery sequence and what each accumulator type checkpoints, see the [Accumulator Design]({{< ref "accumulator-design" >}}) explanation.

## Further Reading

- [Accumulator Design]({{< ref "accumulator-design" >}}) — how accumulators work, the four types, and state management
- [Packaging & FFI]({{< ref "packaging" >}}) — how packaged computation graphs are compiled and loaded
- [Performance Characteristics]({{< ref "performance" >}}) — throughput and latency baseline numbers
