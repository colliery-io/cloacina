---
title: "Performance Characteristics"
weight: 50
---

# Computation Graph Performance

Baseline performance numbers for the embedded computation graph pipeline, measured with `angreal performance computation-graph-bench`.

## Benchmark Setup

- **Graph**: single accumulator (`source`) -> `process` -> `output` (minimal overhead)
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

### Reference Machine

- Apple M3 (macOS)
- Rust 1.85+
- tokio 1.x multi-threaded runtime

## Understanding the Numbers

### Why debug and release are similar

The latency is dominated by **async channel hops and tokio task scheduling**, not by computation or serialization. Each event traverses two mpsc channels:

```
sender -> [socket channel] -> accumulator runtime -> [boundary channel] -> reactor
```

Each hop involves a tokio task wakeup cycle (~3-4ms). The graph computation itself (process + output) is negligible.

### Channel buffer sizes

Increasing channel buffer sizes **increases latency** (more queuing delay) without improving throughput. The current sizes (socket=64, boundary=256) are tuned for the latency/throughput tradeoff.

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
