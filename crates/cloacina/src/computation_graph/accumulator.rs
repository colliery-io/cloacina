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

// =============================================================================
// Accumulator Health
// =============================================================================

/// Health state of an accumulator, reported via watch channel.
///
/// The reactor watches these to gate its own startup (Warming → Live)
/// and detect degradation (Live → Degraded).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccumulatorHealth {
    /// Loading checkpoint from DAL.
    Starting,
    /// Checkpoint loaded, connecting to source. Socket already active.
    Connecting,
    /// Connected, processing events, pushing boundaries.
    Live,
    /// Was live, lost source connection. Socket still active. Retrying.
    Disconnected,
    /// Passthrough — no source to connect to. Healthy by definition.
    SocketOnly,
}

impl std::fmt::Display for AccumulatorHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Connecting => write!(f, "connecting"),
            Self::Live => write!(f, "live"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::SocketOnly => write!(f, "socket_only"),
        }
    }
}

/// Create a health reporting channel for an accumulator.
pub fn health_channel() -> (
    watch::Sender<AccumulatorHealth>,
    watch::Receiver<AccumulatorHealth>,
) {
    watch::channel(AccumulatorHealth::Starting)
}

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

/// Handle for persisting accumulator state via the DAL.
///
/// Wraps simple key-value checkpoint storage keyed by (graph_name, accumulator_name).
/// Serialization uses the same debug-JSON/release-bincode pattern as boundary wire format.
#[derive(Clone)]
pub struct CheckpointHandle {
    dal: crate::dal::unified::DAL,
    graph_name: String,
    accumulator_name: String,
}

impl CheckpointHandle {
    /// Create a new checkpoint handle for the given graph and accumulator.
    pub fn new(
        dal: crate::dal::unified::DAL,
        graph_name: String,
        accumulator_name: String,
    ) -> Self {
        Self {
            dal,
            graph_name,
            accumulator_name,
        }
    }

    /// Persist accumulator state.
    pub async fn save<T: Serialize>(&self, state: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(state)
            .map_err(|e| AccumulatorError::Checkpoint(format!("serialization failed: {}", e)))?;
        self.dal
            .checkpoint()
            .save_checkpoint(&self.graph_name, &self.accumulator_name, bytes)
            .await
            .map_err(|e| AccumulatorError::Checkpoint(e.to_string()))
    }

    /// Load previously persisted accumulator state.
    pub async fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, AccumulatorError> {
        let bytes = self
            .dal
            .checkpoint()
            .load_checkpoint(&self.graph_name, &self.accumulator_name)
            .await
            .map_err(|e| AccumulatorError::Checkpoint(e.to_string()))?;
        match bytes {
            Some(data) => {
                let state = types::deserialize(&data).map_err(|e| {
                    AccumulatorError::Checkpoint(format!("deserialization failed: {}", e))
                })?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Access the underlying DAL for direct checkpoint operations.
    pub fn dal(&self) -> &crate::dal::unified::DAL {
        &self.dal
    }

    /// Get the graph name this handle is scoped to.
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }

    /// Get the accumulator name this handle is scoped to.
    pub fn accumulator_name(&self) -> &str {
        &self.accumulator_name
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
    /// Handle to persist accumulator state. None when DAL is not available
    /// (e.g., embedded mode without database).
    pub checkpoint: Option<CheckpointHandle>,
    /// Health state reporter. None when health tracking is not needed
    /// (e.g., tests, embedded mode).
    pub health: Option<watch::Sender<AccumulatorHealth>>,
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
    // Report starting health
    set_health(&ctx, AccumulatorHealth::Starting);

    // Initialize — may restore state from checkpoint
    if let Err(e) = acc.init(&ctx).await {
        tracing::error!(name = %ctx.name, "accumulator init failed: {}", e);
        return;
    }

    // Passthrough/standard accumulators are socket-only (no external source)
    set_health(&ctx, AccumulatorHealth::SocketOnly);

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
                    } else {
                        persist_boundary(&ctx, &boundary).await;
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

// =============================================================================
// Polling Accumulator
// =============================================================================

/// A polling accumulator periodically calls an async poll function to query
/// pull-based data sources (databases, APIs, config files).
///
/// Returns `Option<Output>` — `Some` emits a boundary, `None` means "no change".
#[async_trait::async_trait]
pub trait PollingAccumulator: Send + 'static {
    /// The typed boundary produced for the reactor.
    type Output: Serialize + DeserializeOwned + Send + 'static;

    /// Poll the data source. Called on each timer tick.
    /// Return `Some(output)` to emit a boundary, `None` to skip.
    async fn poll(&mut self) -> Option<Self::Output>;

    /// Polling interval.
    fn interval(&self) -> std::time::Duration;
}

/// Run a polling accumulator as a timer-based loop.
///
/// On each tick: calls `poll()`, if Some → serializes and sends boundary.
/// Also accepts socket events (same as passthrough — external pushes still work).
pub async fn polling_accumulator_runtime<P: PollingAccumulator>(
    mut poller: P,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
) {
    set_health(&ctx, AccumulatorHealth::Starting);

    let interval = poller.interval();
    let mut timer = tokio::time::interval(interval);
    // Skip the first immediate tick — we want to wait one interval before first poll
    timer.tick().await;

    // Polling accumulators are Live once the timer starts
    set_health(&ctx, AccumulatorHealth::Live);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            _ = timer.tick() => {
                if let Some(output) = poller.poll().await {
                    if let Err(e) = ctx.output.send(&output).await {
                        tracing::error!(name = %ctx.name, "polling boundary send failed: {}", e);
                    } else {
                        persist_boundary(&ctx, &output).await;
                    }
                }
            }
            Some(bytes) = socket_rx.recv() => {
                // Socket events are deserialized as Output and sent directly
                match types::deserialize::<P::Output>(&bytes) {
                    Ok(output) => {
                        if let Err(e) = ctx.output.send(&output).await {
                            tracing::error!(name = %ctx.name, "socket boundary send failed: {}", e);
                        } else {
                            persist_boundary(&ctx, &output).await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "socket deserialize error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "polling accumulator shutting down");
                break;
            }
        }
    }
}

// =============================================================================
// Batch Accumulator
// =============================================================================

/// A batch accumulator buffers incoming events and processes them all at once
/// on a flush signal. Emits a single boundary containing the batch result.
///
/// Flush triggers:
/// - Timer-based flush interval
/// - Buffer size threshold (optional)
/// - Shutdown (drains remaining buffer)
#[async_trait::async_trait]
pub trait BatchAccumulator: Send + 'static {
    /// The raw event type buffered from the source.
    type Event: DeserializeOwned + Send + 'static;

    /// The typed boundary produced from the batch.
    type Output: Serialize + Send + 'static;

    /// Process a batch of events and optionally produce a boundary.
    /// Called when the buffer is flushed. Empty batches are never passed.
    fn process_batch(&mut self, events: Vec<Self::Event>) -> Option<Self::Output>;
}

/// Configuration for the batch accumulator runtime.
pub struct BatchAccumulatorConfig {
    /// Optional timer-based flush interval. If None, only flushes on signal or size threshold.
    pub flush_interval: Option<std::time::Duration>,
    /// Optional: flush when buffer reaches this size.
    pub max_buffer_size: Option<usize>,
}

impl Default for BatchAccumulatorConfig {
    fn default() -> Self {
        Self {
            flush_interval: None,
            max_buffer_size: None,
        }
    }
}

/// Create a flush signal pair for batch accumulators.
///
/// The sender is held by the reactor (or external code) and used to trigger
/// a flush. The receiver is passed to `batch_accumulator_runtime`.
pub fn flush_signal() -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    mpsc::channel(16)
}

/// Run a batch accumulator that buffers events and flushes on signal, timer, or size threshold.
///
/// Primary flush trigger is the `flush_rx` channel — typically sent by the reactor
/// after each graph execution ("give me everything since last run").
/// Timer and size threshold are secondary/fallback triggers.
pub async fn batch_accumulator_runtime<B: BatchAccumulator>(
    mut acc: B,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    mut flush_rx: mpsc::Receiver<()>,
    config: BatchAccumulatorConfig,
) {
    set_health(&ctx, AccumulatorHealth::Starting);

    // Create timer only if interval is configured
    let mut timer = config.flush_interval.map(tokio::time::interval);
    if let Some(ref mut t) = timer {
        // Skip the first immediate tick
        t.tick().await;
    }

    // Batch accumulators are Live once ready to receive events
    set_health(&ctx, AccumulatorHealth::Live);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;
    let mut buffer: Vec<B::Event> = Vec::new();

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                match types::deserialize::<B::Event>(&bytes) {
                    Ok(event) => {
                        buffer.push(event);
                        // Check size threshold
                        if let Some(max) = config.max_buffer_size {
                            if buffer.len() >= max {
                                flush_batch(&mut acc, &mut buffer, &ctx).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "batch deserialize error: {}", e);
                    }
                }
            }
            Some(()) = flush_rx.recv() => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
            }
            _ = async {
                match timer.as_mut() {
                    Some(t) => t.tick().await,
                    None => std::future::pending().await,
                }
            } => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "batch accumulator shutting down, draining buffer");
                // Drain remaining buffer on shutdown
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                break;
            }
        }
    }
}

/// Flush the buffer through the batch accumulator and send boundary if produced.
async fn flush_batch<B: BatchAccumulator>(
    acc: &mut B,
    buffer: &mut Vec<B::Event>,
    ctx: &AccumulatorContext,
) {
    if buffer.is_empty() {
        return;
    }
    let batch = std::mem::take(buffer);
    let count = batch.len();
    if let Some(output) = acc.process_batch(batch) {
        if let Err(e) = ctx.output.send(&output).await {
            tracing::error!(name = %ctx.name, "batch boundary send failed: {}", e);
        } else {
            tracing::debug!(name = %ctx.name, events = count, "batch flushed");
            persist_boundary(ctx, &output).await;
        }
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Set health state (best-effort, no-op if health channel not configured).
fn set_health(ctx: &AccumulatorContext, health: AccumulatorHealth) {
    if let Some(ref sender) = ctx.health {
        let _ = sender.send(health);
    }
}

/// Persist last-emitted boundary to DAL (best-effort, logs on failure).
async fn persist_boundary<T: Serialize>(ctx: &AccumulatorContext, boundary: &T) {
    if let Some(ref handle) = ctx.checkpoint {
        let bytes = match types::serialize(boundary) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(name = %ctx.name, "boundary persistence serialization failed: {}", e);
                return;
            }
        };
        if let Err(e) = handle
            .dal()
            .checkpoint()
            .save_boundary(handle.graph_name(), handle.accumulator_name(), bytes, 0)
            .await
        {
            tracing::warn!(name = %ctx.name, "boundary persistence failed: {}", e);
        }
    }
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
            checkpoint: None,
            health: None,
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
            checkpoint: None,
            health: None,
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
            checkpoint: None,
            health: None,
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

    // --- Polling accumulator tests ---

    struct CountingPoller {
        count: u32,
        max: u32,
    }

    #[async_trait::async_trait]
    impl PollingAccumulator for CountingPoller {
        type Output = TestBoundary;

        async fn poll(&mut self) -> Option<TestBoundary> {
            self.count += 1;
            if self.count <= self.max {
                Some(TestBoundary {
                    result: self.count as f64,
                })
            } else {
                None // "no change" after max polls
            }
        }

        fn interval(&self) -> std::time::Duration {
            std::time::Duration::from_millis(50)
        }
    }

    #[tokio::test]
    async fn test_polling_accumulator_emits_on_some() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (_socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("poller"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "poller".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let poller = CountingPoller { count: 0, max: 3 };
        let handle = tokio::spawn(polling_accumulator_runtime(poller, ctx, socket_rx));

        // Wait for 3 polls (50ms each + first tick skipped = ~200ms)
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        // Should have received 3 boundaries
        let mut received = vec![];
        while let Ok((_name, bytes)) = boundary_rx.try_recv() {
            let b: TestBoundary = types::deserialize(&bytes).unwrap();
            received.push(b.result);
        }
        assert!(
            received.len() >= 3,
            "expected at least 3 polls, got {}",
            received.len()
        );
        assert_eq!(received[0], 1.0);
        assert_eq!(received[1], 2.0);
        assert_eq!(received[2], 3.0);

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_polling_accumulator_skips_on_none() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (_socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("skip_poller"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "skip_poller".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        // max=0 means poll always returns None
        let poller = CountingPoller { count: 0, max: 0 };
        let handle = tokio::spawn(polling_accumulator_runtime(poller, ctx, socket_rx));

        // Wait for a few poll cycles
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should have received zero boundaries
        assert!(
            boundary_rx.try_recv().is_err(),
            "should not have received any boundary"
        );

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_polling_accumulator_shutdown() {
        let (boundary_tx, _boundary_rx) = mpsc::channel(10);
        let (_socket_tx, socket_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("shutdown_poller"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "shutdown_poller".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let poller = CountingPoller { count: 0, max: 100 };
        let handle = tokio::spawn(polling_accumulator_runtime(poller, ctx, socket_rx));

        // Shutdown immediately
        shutdown_tx.send(true).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("polling runtime should shut down within 2 seconds")
            .unwrap();
    }

    // --- Batch accumulator tests ---

    struct SumBatchAccumulator;

    #[async_trait::async_trait]
    impl BatchAccumulator for SumBatchAccumulator {
        type Event = TestEvent;
        type Output = TestBoundary;

        fn process_batch(&mut self, events: Vec<TestEvent>) -> Option<TestBoundary> {
            let sum: f64 = events.iter().map(|e| e.value).sum();
            Some(TestBoundary { result: sum })
        }
    }

    #[tokio::test]
    async fn test_batch_accumulator_flush_on_signal() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (flush_tx, flush_rx) = flush_signal();
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("batch_signal"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "batch_signal".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let config = BatchAccumulatorConfig::default(); // no timer, no size threshold

        let handle = tokio::spawn(batch_accumulator_runtime(
            SumBatchAccumulator,
            ctx,
            socket_rx,
            flush_rx,
            config,
        ));

        // Push 3 events
        for v in [10.0, 20.0, 30.0] {
            socket_tx
                .send(types::serialize(&TestEvent { value: v }).unwrap())
                .await
                .unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // No boundary yet — no flush signal sent
        assert!(boundary_rx.try_recv().is_err());

        // Send flush signal
        flush_tx.send(()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Should get one boundary with sum = 60.0
        let (_name, bytes) = boundary_rx.recv().await.unwrap();
        let b: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(b.result, 60.0);

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_batch_accumulator_flush_on_timer() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (_flush_tx, flush_rx) = flush_signal();
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("batch"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "batch".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let config = BatchAccumulatorConfig {
            flush_interval: Some(std::time::Duration::from_millis(100)),
            max_buffer_size: None,
        };

        let handle = tokio::spawn(batch_accumulator_runtime(
            SumBatchAccumulator,
            ctx,
            socket_rx,
            flush_rx,
            config,
        ));

        // Push 5 events quickly (before timer fires)
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            socket_tx
                .send(types::serialize(&TestEvent { value: v }).unwrap())
                .await
                .unwrap();
        }

        // Wait for timer flush
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should get ONE boundary with sum = 15.0
        let (_name, bytes) = boundary_rx.recv().await.unwrap();
        let b: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(b.result, 15.0);

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_batch_accumulator_empty_flush_skips() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (_socket_tx, socket_rx) = mpsc::channel(10);
        let (flush_tx, flush_rx) = flush_signal();
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("empty_batch"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "empty_batch".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let config = BatchAccumulatorConfig::default();

        let handle = tokio::spawn(batch_accumulator_runtime(
            SumBatchAccumulator,
            ctx,
            socket_rx,
            flush_rx,
            config,
        ));

        // Send flush with empty buffer
        flush_tx.send(()).await.unwrap();

        // Wait for a few flush cycles with no events
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should have no boundaries
        assert!(boundary_rx.try_recv().is_err());

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_batch_accumulator_max_buffer_size() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (_flush_tx, flush_rx) = flush_signal();
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("size_batch"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "size_batch".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let config = BatchAccumulatorConfig {
            flush_interval: None,     // no timer
            max_buffer_size: Some(3), // flush at 3 events
        };

        let handle = tokio::spawn(batch_accumulator_runtime(
            SumBatchAccumulator,
            ctx,
            socket_rx,
            flush_rx,
            config,
        ));

        // Push exactly 3 events — should trigger size-based flush
        for v in [10.0, 20.0, 30.0] {
            socket_tx
                .send(types::serialize(&TestEvent { value: v }).unwrap())
                .await
                .unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Should get boundary with sum = 60.0
        let (_name, bytes) = boundary_rx.recv().await.unwrap();
        let b: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(b.result, 60.0);

        shutdown_tx.send(true).unwrap();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_batch_accumulator_shutdown_drains() {
        let (boundary_tx, mut boundary_rx) = mpsc::channel(10);
        let (socket_tx, socket_rx) = mpsc::channel(10);
        let (_flush_tx, flush_rx) = flush_signal();
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let sender = BoundarySender::new(boundary_tx, SourceName::new("drain_batch"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "drain_batch".to_string(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: None,
        };

        let config = BatchAccumulatorConfig::default(); // no timer, no size

        let handle = tokio::spawn(batch_accumulator_runtime(
            SumBatchAccumulator,
            ctx,
            socket_rx,
            flush_rx,
            config,
        ));

        // Push events without triggering flush
        for v in [1.0, 2.0] {
            socket_tx
                .send(types::serialize(&TestEvent { value: v }).unwrap())
                .await
                .unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Shutdown — should drain remaining buffer
        shutdown_tx.send(true).unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;

        // Should get one boundary from the drain
        let (_name, bytes) = boundary_rx.recv().await.unwrap();
        let b: TestBoundary = types::deserialize(&bytes).unwrap();
        assert_eq!(b.result, 3.0); // 1.0 + 2.0
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
            checkpoint: None,
            health: None,
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
