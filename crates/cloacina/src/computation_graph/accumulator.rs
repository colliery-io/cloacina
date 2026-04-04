/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Accumulator trait, runtime, and supporting types.
//!
//! An accumulator is a long-lived process that consumes events from a source,
//! optionally aggregates them, and pushes typed boundaries to a reactor.
//! See CLOACI-S-0004 for the full specification.

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{mpsc, watch};

use super::types::{self, GraphError, SourceName};

/// Errors from accumulator operations.
#[derive(Debug, thiserror::Error)]
pub enum AccumulatorError {
    #[error("accumulator init failed: {0}")]
    Init(String),
    #[error("accumulator run failed: {0}")]
    Run(String),
    #[error("send failed: {0}")]
    Send(String),
    #[error("checkpoint error: {0}")]
    Checkpoint(String),
}

/// An accumulator consumes events from a source and pushes boundaries to a reactor.
///
/// Two input paths:
/// - Event loop (optional): `run()` actively pulls from a source
/// - Socket receiver: events pushed in from outside (always active)
///
/// Both paths feed through `process()` which is called sequentially.
#[async_trait::async_trait]
pub trait Accumulator: Send + 'static {
    /// The raw event type consumed from the source.
    type Event: DeserializeOwned + Send + 'static;

    /// The typed boundary produced for the reactor.
    type Output: Serialize + Send + 'static;

    /// Process a received event and optionally produce a boundary.
    /// Called sequentially by the processor task — no concurrent `&mut self`.
    fn process(&mut self, event: Self::Event) -> Option<Self::Output>;

    /// Optional: active event loop that pulls from a source and pushes
    /// raw events into the merge channel. Should NOT call `process()` directly.
    /// Default: no event loop (socket-only / passthrough mode).
    async fn run(
        &mut self,
        _ctx: &AccumulatorContext,
        _events: mpsc::Sender<Self::Event>,
    ) -> Result<(), AccumulatorError> {
        // Default: no active event loop. Accumulator is socket-only.
        std::future::pending().await
    }

    /// Called on startup before `run()` or first receive.
    /// Use to restore state from last checkpoint.
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}

/// Context provided to the accumulator by the runtime.
pub struct AccumulatorContext {
    /// Send a boundary to the reactor.
    pub output: BoundarySender,
    /// Accumulator's name (used for registration and logging).
    pub name: String,
    /// Shutdown signal — accumulator should exit run() when this fires.
    pub shutdown: watch::Receiver<bool>,
}

/// Sends serialized boundaries to the reactor.
///
/// Wire format: bincode in release, JSON in debug.
#[derive(Clone)]
pub struct BoundarySender {
    inner: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: SourceName,
}

impl BoundarySender {
    pub fn new(sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName) -> Self {
        Self {
            inner: sender,
            source_name,
        }
    }

    /// Serialize and send a boundary to the reactor.
    pub async fn send<T: Serialize>(&self, boundary: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(boundary)
            .map_err(|e| AccumulatorError::Send(format!("serialization failed: {}", e)))?;
        self.inner
            .send((self.source_name.clone(), bytes))
            .await
            .map_err(|e| AccumulatorError::Send(format!("channel send failed: {}", e)))?;
        Ok(())
    }

    /// Get the source name this sender is associated with.
    pub fn source_name(&self) -> &SourceName {
        &self.source_name
    }
}

/// Configuration for the accumulator runtime.
pub struct AccumulatorRuntimeConfig {
    /// Merge channel capacity (backpressure).
    pub merge_channel_capacity: usize,
}

impl Default for AccumulatorRuntimeConfig {
    fn default() -> Self {
        Self {
            merge_channel_capacity: 1024,
        }
    }
}

/// Run an accumulator as 3 tokio tasks connected by a merge channel.
///
/// ```text
/// ┌─────────────────┐
/// │  Event loop task │──→ mpsc<Event> ──┐
/// │  (optional)      │                  │     ┌─────────────────┐
/// └─────────────────┘                  ├────→│  Processor task  │──→ BoundarySender ──→ Reactor
/// ┌─────────────────┐                  │     │  (calls process) │
/// │  Socket task     │──→ mpsc<Event> ──┘     └─────────────────┘
/// │  (always active) │
/// └─────────────────┘
/// ```
pub async fn accumulator_runtime<A: Accumulator>(
    mut acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
) {
    // Initialize
    if let Err(e) = acc.init(&ctx).await {
        tracing::error!(name = %ctx.name, "accumulator init failed: {}", e);
        return;
    }

    // Create merge channel
    let (event_tx, mut event_rx) = mpsc::channel::<A::Event>(config.merge_channel_capacity);

    // Spawn event loop task
    let event_tx_loop = event_tx.clone();
    let mut shutdown_loop = ctx.shutdown.clone();
    let name_loop = ctx.name.clone();
    let loop_handle = tokio::spawn(async move {
        // We need a mutable reference to acc for run(), but acc is moved into the processor.
        // Instead, run() is a no-op for passthrough accumulators. For stream accumulators,
        // the event loop is handled externally (StreamBackend feeds into event_tx).
        // So this task just waits for shutdown.
        let _ = shutdown_loop.changed().await;
        tracing::debug!(name = %name_loop, "event loop task shutting down");
    });

    // Spawn socket receiver task
    let event_tx_socket = event_tx.clone();
    let mut shutdown_socket = ctx.shutdown.clone();
    let name_socket = ctx.name.clone();
    let socket_handle = tokio::spawn(async move {
        let mut socket_rx = socket_rx;
        loop {
            tokio::select! {
                Some(bytes) = socket_rx.recv() => {
                    match types::deserialize::<A::Event>(&bytes) {
                        Ok(event) => {
                            if event_tx_socket.send(event).await.is_err() {
                                break; // merge channel closed
                            }
                        }
                        Err(e) => {
                            tracing::warn!(name = %name_socket, "socket deserialize error: {}", e);
                        }
                    }
                }
                _ = shutdown_socket.changed() => {
                    tracing::debug!(name = %name_socket, "socket task shutting down");
                    break;
                }
            }
        }
    });

    // Processor task (runs on current task — owns &mut acc)
    let mut shutdown_proc = ctx.shutdown.clone();
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                if let Some(boundary) = acc.process(event) {
                    if let Err(e) = ctx.output.send(&boundary).await {
                        tracing::error!(name = %ctx.name, "boundary send failed: {}", e);
                    }
                }
            }
            _ = shutdown_proc.changed() => {
                tracing::debug!(name = %ctx.name, "processor task shutting down");
                break;
            }
        }
    }

    // Wait for spawned tasks to finish
    let _ = loop_handle.await;
    let _ = socket_handle.await;
}

/// Create a shutdown signal pair.
pub fn shutdown_signal() -> (watch::Sender<bool>, watch::Receiver<bool>) {
    watch::channel(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEvent {
        value: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestBoundary {
        result: f64,
    }

    struct DoubleAccumulator;

    #[async_trait::async_trait]
    impl Accumulator for DoubleAccumulator {
        type Event = TestEvent;
        type Output = TestBoundary;

        fn process(&mut self, event: TestEvent) -> Option<TestBoundary> {
            Some(TestBoundary {
                result: event.value * 2.0,
            })
        }
    }

    #[tokio::test]
    async fn test_boundary_sender_round_trip() {
        let (tx, mut rx) = mpsc::channel(10);
        let sender = BoundarySender::new(tx, SourceName::new("test"));

        let boundary = TestBoundary { result: 42.0 };
        sender.send(&boundary).await.unwrap();

        let (name, bytes) = rx.recv().await.unwrap();
        assert_eq!(name, SourceName::new("test"));

        let decoded: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(decoded, boundary);
    }

    #[tokio::test]
    async fn test_accumulator_runtime_processes_socket_events() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("test_acc"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "test_acc".to_string(),
            shutdown: shutdown_rx,
        };

        let acc = DoubleAccumulator;

        // Spawn the runtime
        let handle = tokio::spawn(accumulator_runtime(
            acc,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        // Push an event via socket
        let event = TestEvent { value: 5.0 };
        let event_bytes = types::serialize(&event).unwrap();
        socket_tx.send(event_bytes).await.unwrap();

        // Read the boundary
        let (name, bytes) = boundary_rx.recv().await.unwrap();
        assert_eq!(name, SourceName::new("test_acc"));
        let boundary: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(boundary.result, 10.0);

        // Shutdown
        shutdown_tx.send(true).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_accumulator_runtime_multiple_events() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("multi"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "multi".to_string(),
            shutdown: shutdown_rx,
        };

        let handle = tokio::spawn(accumulator_runtime(
            DoubleAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        // Push 3 events
        for v in [1.0, 2.0, 3.0] {
            let bytes = types::serialize(&TestEvent { value: v }).unwrap();
            socket_tx.send(bytes).await.unwrap();
        }

        // Read 3 boundaries in order
        for expected in [2.0, 4.0, 6.0] {
            let (_, bytes) = boundary_rx.recv().await.unwrap();
            let boundary: TestBoundary = types::deserialize(&bytes).unwrap();
            assert_eq!(boundary.result, expected);
        }

        shutdown_tx.send(true).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_accumulator_shutdown() {
        let (boundary_tx, _boundary_rx) = mpsc::channel(10);
        let (_socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("shutdown_test"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "shutdown_test".to_string(),
            shutdown: shutdown_rx,
        };

        let handle = tokio::spawn(accumulator_runtime(
            DoubleAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        // Shutdown immediately
        shutdown_tx.send(true).unwrap();

        // Should complete without hanging
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("runtime should shut down within 2 seconds")
            .unwrap();
    }

    struct FilterAccumulator;

    #[async_trait::async_trait]
    impl Accumulator for FilterAccumulator {
        type Event = TestEvent;
        type Output = TestBoundary;

        fn process(&mut self, event: TestEvent) -> Option<TestBoundary> {
            // Only produce boundary for values > 5
            if event.value > 5.0 {
                Some(TestBoundary {
                    result: event.value,
                })
            } else {
                None
            }
        }
    }

    #[tokio::test]
    async fn test_accumulator_process_returns_none() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("filter"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "filter".to_string(),
            shutdown: shutdown_rx,
        };

        let handle = tokio::spawn(accumulator_runtime(
            FilterAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        // Push event below threshold (should produce no boundary)
        socket_tx
            .send(types::serialize(&TestEvent { value: 3.0 }).unwrap())
            .await
            .unwrap();

        // Push event above threshold (should produce boundary)
        socket_tx
            .send(types::serialize(&TestEvent { value: 10.0 }).unwrap())
            .await
            .unwrap();

        // Only one boundary should come through
        let (_, bytes) = boundary_rx.recv().await.unwrap();
        let boundary: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(boundary.result, 10.0);

        shutdown_tx.send(true).unwrap();
        handle.await.unwrap();
    }
}
