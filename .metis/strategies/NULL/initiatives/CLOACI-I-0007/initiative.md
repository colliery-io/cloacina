---
id: fix-rwlock-poisoning-recovery-in
level: initiative
title: "Fix RwLock Poisoning Recovery in Task Registry"
short_code: "CLOACI-I-0007"
created_at: 2025-11-29T02:40:07.342548+00:00
updated_at: 2025-11-29T02:40:07.342548+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
strategy_id: NULL
initiative_id: fix-rwlock-poisoning-recovery-in
---

# Fix RwLock Poisoning Recovery in Task Registry Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The task registry in `cloacina/src/executor/thread_task_executor.rs` (lines 112-118) recovers from RwLock poisoning by extracting the inner data and continuing:

```rust
let global_tasks = match global_registry.read() {
    Ok(guard) => guard,
    Err(poisoned) => {
        tracing::warn!("Task registry RwLock was poisoned, recovering data");
        poisoned.into_inner()  // Line 116 - recovers poisoned lock
    }
};
```

**Problem:** A poisoned RwLock indicates a panic occurred while holding the lock, meaning the protected data may be in an inconsistent state. Continuing with potentially corrupted data can lead to:
- Silent data corruption
- Cascading failures
- Difficult-to-debug issues

## Goals & Non-Goals

**Goals:**
- Fail explicitly on lock poisoning rather than recovering
- Add metrics/alerting for lock poisoning events
- Consider migrating to `parking_lot::RwLock` which doesn't poison

**Non-Goals:**
- Redesigning the task registry architecture
- Adding transaction semantics to registry operations

## Detailed Design

### Option 1: Fail on Poison (Recommended)

Change recovery to explicit failure:

```rust
let global_tasks = global_registry.read()
    .map_err(|_| {
        tracing::error!("Task registry RwLock poisoned - indicates prior panic");
        metrics::counter!("task_registry.lock_poisoned", 1);
        ExecutorError::LockPoisoned("task_registry")
    })?;
```

### Option 2: Migrate to parking_lot

Replace `std::sync::RwLock` with `parking_lot::RwLock`:

```rust
// Cargo.toml
parking_lot = "0.12"

// Code change
use parking_lot::RwLock;

// parking_lot::RwLock doesn't poison on panics
// No change to lock acquisition code needed
```

**Benefits of parking_lot:**
- No poisoning semantics
- Generally faster than std RwLock
- Smaller memory footprint
- Fair locking available

### Add Metrics for Lock Contention

```rust
let start = Instant::now();
let global_tasks = global_registry.read()?;
metrics::histogram!("task_registry.read_lock.duration_us", start.elapsed().as_micros() as f64);
```

## Testing Strategy

- Test panic during write lock held
- Verify error propagation on poison
- Benchmark lock acquisition times
- Test concurrent read/write patterns

## Alternatives Considered

1. **Keep current recovery** - Risk of data corruption outweighs convenience
2. **Use Mutex instead** - Loses read parallelism benefits
3. **Lock-free data structure** - Too complex for the use case

## Implementation Plan

1. Add `LockPoisoned` variant to error types
2. Replace poison recovery with explicit error
3. Add metrics for lock operations
4. Evaluate parking_lot migration as follow-up
5. Add documentation on thread safety guarantees
