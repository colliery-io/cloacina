---
title: "Performance Characteristics"
weight: 50
---

# Computation Graph Performance

Baseline performance numbers for the embedded computation graph pipeline. The benchmark exercises the full pipeline from event injection to graph execution completion, using a minimal single-accumulator graph to isolate the framework overhead from the application logic.

## The Pipeline

```
injector
   │
   ▼ (socket channel, capacity=64)
accumulator socket task
   │
   ▼ (merge channel, capacity=1024)
accumulator processor task
   │
   ▼ (boundary channel, capacity=256)
reactor
   │
   ▼
graph_fn(cache snapshot)
   │
   ▼
GraphResult
```

The benchmark measures wall-clock time from the moment `socket_tx.send(bytes)` returns to the moment `graph_fn` completes. This includes all channel hops, task wakeups, cache updates, dirty flag evaluation, and the graph execution itself.

## Benchmark Setup

- **Graph**: single accumulator (`source`) → `process` → `output` (minimal overhead)
- **Binary**: `cg-bench` at `examples/performance/computation-graph/`
- **Strategy**: `when_any` reaction criteria, `Latest` input strategy
- **Channels**: socket=64, boundary=256, merge=1024

## Baseline Results

### Latency (event push to graph execution complete)

| Metric | Debug Build | Release Build |
|--------|------------|---------------|
| p50    | ~0 us      | 2,745 us      |
| p95    | 7,638 us   | 9,196 us      |
| p99    | 9,076 us   | 10,480 us     |
| mean   | 1,874 us   | 3,355 us      |

Measured at 1ms injection interval over 10 seconds (~10,000 events pushed, ~7,600-7,900 graph fires).

### Throughput (max sustained events/sec before channel backup)

| Metric | Debug Build | Release Build |
|--------|------------|---------------|
| Max sustained | ~733 events/sec | ~763 events/sec |

Measured by ramping injection rate from 500us down to 10us interval until `TrySendError::Full` is detected.

### Kafka Stream Accumulator Throughput (soak test)

These numbers measure sustained throughput from a live Kafka broker through the stream accumulator into graph execution. The soak ran for 5 minutes to surface any backpressure or offset-commit saturation.

| Accumulator type | Sustained throughput |
|-----------------|---------------------|
| Stream (latest value) | ~70 events/sec |
| Batch (flush after graph) | ~45 graph firings/sec |

Stream throughput is lower than the passthrough baseline because each message involves a Kafka `recv()` call (network round trip) plus an offset `commit()` call after graph execution. Batch throughput is lower still because the batch size determines firing frequency — smaller batches fire more often, larger batches fire less often but process more events per fire. These numbers reflect default Kafka consumer configuration; `acks`, `fetch.wait.max.ms`, and consumer group partition count all affect real-world throughput.

### Reference Machine

- Apple M3 (macOS)
- Rust 1.85+
- tokio 1.x multi-threaded runtime

## Understanding the Numbers

### Why debug and release are similar

The benchmark's most counterintuitive result is that debug and release builds have nearly identical throughput (~733 vs ~763 events/sec). This is because **the bottleneck is async channel hops and tokio task scheduling, not computation or serialization**.

Each event traverses two mpsc channels before reaching the reactor:

```
injector
   │  try_send() — if full, backpressure detected
   ▼
[socket channel, cap=64]
   │  recv() + deserialize
   ▼
accumulator processor
   │  process() — user code, ~negligible
   │  serialize + send()
   ▼
[boundary channel, cap=256]
   │  recv()
   ▼
reactor
   │  cache.update() + dirty.set() + criteria check
   │  graph_fn(snapshot).await
   ▼
GraphResult
```

Each channel hop involves a tokio task wakeup: a sleeping task is woken, scheduled onto a tokio thread, and begins executing. On the Apple M3, each wakeup cycle takes roughly 3-4ms under load. With two hops, the latency floor is approximately 6-8ms — matching the observed p95/p99 numbers.

Rust's release optimization eliminates dead code and speeds up serialization, but it cannot eliminate tokio task scheduling overhead. The computation itself (the `process()` call and the graph function) is genuinely negligible compared to the scheduling cost. This means:

- Adding more complex user logic in `process()` or graph nodes will not significantly affect throughput until it exceeds the scheduling cost
- Profile-switching from debug to release will not dramatically change latency for typical workloads
- The latency floor is set by the number of channel hops, not by the amount of work done in each hop

### Channel buffer sizes

Increasing channel buffer sizes **increases latency** (more queuing delay) without improving throughput. The current sizes (socket=64, boundary=256) are tuned for the latency/throughput tradeoff. A larger socket channel means more events can queue up before backpressure, which allows bursts but also allows a slow reactor to fall further behind before the injector notices.

The `merge_channel_capacity` (1024 by default) is larger because it is the internal merge point for both the socket task and the event source task. It needs headroom to avoid deadlocking when both paths produce events simultaneously.

### The `Latest` input strategy

The benchmark uses `Latest` input strategy: if 10 events arrive while the reactor is executing a graph, only the 10th value is in the cache when the next execution starts. This is why ~7,600-7,900 graph fires are observed for ~10,000 events pushed at 1ms intervals — some boundaries collapse because the reactor is busy. This is the correct behavior for reactive workloads. If every event must produce exactly one graph execution, use `Sequential` strategy (at the cost of higher per-event latency as the queue builds up).

## Running Benchmarks

```bash
# Full benchmark (default: 15s latency + 10s throughput)
angreal performance computation-graph-bench

# Quick run
angreal performance computation-graph-bench --latency-duration 5 --throughput-duration 3

# Release build for production-representative numbers
cd examples/performance/computation-graph
cargo run --release --bin cg-bench -- --latency-duration 15 --throughput-duration 10
```

The `cg-bench` binary is at `examples/performance/computation-graph/src/main.rs`. It creates an in-process graph with a single passthrough accumulator, injects events at a configurable rate, and records timestamps at injection and graph completion to compute latency histograms.

## Further Reading

- [Architecture]({{< ref "architecture" >}}) — the reactor loop and input strategy semantics
- [Accumulator Design]({{< ref "accumulator-design" >}}) — how channel sizes and accumulator types affect throughput
