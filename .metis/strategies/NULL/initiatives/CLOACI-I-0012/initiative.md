---
id: fix-async-runtime-shutdown-error
level: initiative
title: "Fix Async Runtime Shutdown Error Handling in Python Bindings"
short_code: "CLOACI-I-0012"
created_at: 2025-11-29T02:40:20.488270+00:00
updated_at: 2025-11-29T02:40:20.488270+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
strategy_id: NULL
initiative_id: fix-async-runtime-shutdown-error
---

# Fix Async Runtime Shutdown Error Handling in Python Bindings Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The async runtime shutdown in `cloaca-backend/src/runner.rs` (lines 98-121) silently ignores errors:

```rust
impl AsyncRuntimeHandle {
    fn shutdown(&mut self) {
        let _ = self.tx.send(RuntimeMessage::Shutdown);  // Ignores send errors
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();  // Ignores join panics
        }
    }
}
```

**Problems:**
- Channel send failures silently ignored (runtime may not receive shutdown)
- Thread panics during shutdown are ignored (no logging/alerting)
- No timeout for thread join - could hang forever
- Drop impl calls shutdown but can't handle errors

**Risk:** Resource leaks or hangs on shutdown if async runtime fails.

## Goals & Non-Goals

**Goals:**
- Log all shutdown errors with context
- Add timeout for thread join to prevent hangs
- Surface shutdown failures to callers where possible
- Add metrics for shutdown success/failure

**Non-Goals:**
- Changing the overall threading model
- Adding retry logic for shutdown

## Detailed Design

### Improved Shutdown Implementation

```rust
use std::time::Duration;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

impl AsyncRuntimeHandle {
    pub fn shutdown(&mut self) -> Result<(), ShutdownError> {
        // Send shutdown signal
        if let Err(e) = self.tx.send(RuntimeMessage::Shutdown) {
            tracing::error!("Failed to send shutdown signal: {:?}", e);
            // Continue with join attempt anyway
        }
        
        if let Some(handle) = self.thread_handle.take() {
            // Use a timeout for joining
            let start = std::time::Instant::now();
            
            // Spawn a watchdog thread
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let join_handle = std::thread::spawn(move || {
                let result = handle.join();
                let _ = done_tx.send(result);
            });
            
            match done_rx.recv_timeout(SHUTDOWN_TIMEOUT) {
                Ok(Ok(())) => {
                    tracing::debug!(
                        duration_ms = start.elapsed().as_millis(),
                        "Async runtime shutdown completed"
                    );
                    Ok(())
                }
                Ok(Err(panic_payload)) => {
                    tracing::error!("Async runtime thread panicked during shutdown");
                    Err(ShutdownError::ThreadPanic)
                }
                Err(_timeout) => {
                    tracing::error!(
                        timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
                        "Async runtime shutdown timed out"
                    );
                    Err(ShutdownError::Timeout)
                }
            }
        } else {
            Ok(()) // Already shut down
        }
    }
}
```

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    #[error("Runtime thread panicked during shutdown")]
    ThreadPanic,
    
    #[error("Shutdown timed out after {0} seconds")]
    Timeout(u64),
    
    #[error("Failed to send shutdown signal")]
    ChannelClosed,
}
```

### Drop Implementation

```rust
impl Drop for AsyncRuntimeHandle {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            tracing::warn!("Error during AsyncRuntimeHandle drop: {}", e);
        }
    }
}
```

## Testing Strategy

- Test normal shutdown path
- Test shutdown when runtime thread has panicked
- Test shutdown timeout (mock slow thread)
- Test double-shutdown (idempotent)

## Alternatives Considered

1. **Ignore errors (current)** - Risk of silent resource leaks
2. **Panic on shutdown failure** - Too aggressive for cleanup code
3. **Async shutdown** - Adds complexity, Drop can't be async

## Implementation Plan

1. Create `ShutdownError` type
2. Implement timeout-based join
3. Add logging throughout shutdown path
4. Update Drop implementation
5. Add shutdown metrics
6. Update Python bindings to expose shutdown errors