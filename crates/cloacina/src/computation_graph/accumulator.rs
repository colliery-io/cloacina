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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{mpsc, watch};

use super::types::{self, SourceName};

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
/// - Event source (optional): an [`EventSource`] actively pulls from a source
/// - Socket receiver: events pushed in from outside (always active)
///
/// Both paths feed through `process()` which is called sequentially.
///
/// To add an active event loop, implement [`EventSource`] and pass it to
/// [`accumulator_runtime_with_source`]. The processor owns `&mut self` for
/// `process()` while the event source runs independently on its own task.
#[async_trait::async_trait]
pub trait Accumulator: Send + 'static {
    /// The typed boundary produced for the reactor.
    type Output: Serialize + Send + 'static;

    /// Process raw event bytes and optionally produce a boundary.
    /// The implementor owns deserialization — the runtime is format-agnostic.
    /// Called sequentially by the processor task — no concurrent `&mut self`.
    fn process(&mut self, event: Vec<u8>) -> Option<Self::Output>;

    /// Called on startup before first receive.
    /// Use to restore state from last checkpoint.
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}

/// An event source actively pulls events from an external source and pushes
/// them into the accumulator's merge channel. Runs on its own tokio task,
/// independently of the processor that calls [`Accumulator::process()`].
///
/// This is the correct way to add an active event loop to an accumulator.
/// The event source takes ownership (`self`, not `&mut self`) so it can
/// run concurrently with the processor without borrowing conflicts.
///
/// For stream-backed sources, see also [`StreamBackend`](super::stream_backend::StreamBackend).
#[async_trait::async_trait]
pub trait EventSource: Send + 'static {
    /// Run the event loop. Push raw event bytes into `events` until shutdown
    /// fires or the source is exhausted. The runtime shuts down if this returns.
    async fn run(
        self,
        events: mpsc::Sender<Vec<u8>>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<(), AccumulatorError>;
}

/// Handle for persisting accumulator state via the DAL.
///
/// Wraps simple key-value checkpoint storage keyed by (graph_name, accumulator_name).
/// Serialization uses bincode wire format.
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
/// Wire format: bincode.
/// Tracks a monotonically increasing sequence number per accumulator
/// for deduplication and ordering guarantees.
#[derive(Clone)]
pub struct BoundarySender {
    inner: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: SourceName,
    /// Monotonically increasing sequence counter (shared across clones).
    sequence: Arc<AtomicU64>,
}

impl BoundarySender {
    pub fn new(sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a sender with a specific starting sequence number (for restart recovery).
    pub fn with_sequence(
        sender: mpsc::Sender<(SourceName, Vec<u8>)>,
        source_name: SourceName,
        start_sequence: u64,
    ) -> Self {
        Self {
            inner: sender,
            source_name,
            sequence: Arc::new(AtomicU64::new(start_sequence)),
        }
    }

    /// Serialize and send a boundary to the reactor.
    /// Increments the sequence counter atomically after successful send.
    pub async fn send<T: Serialize>(&self, boundary: &T) -> Result<(), AccumulatorError> {
        let bytes = types::serialize(boundary)
            .map_err(|e| AccumulatorError::Send(format!("serialization failed: {}", e)))?;
        self.inner
            .send((self.source_name.clone(), bytes))
            .await
            .map_err(|e| AccumulatorError::Send(format!("channel send failed: {}", e)))?;
        self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Get the source name this sender is associated with.
    pub fn source_name(&self) -> &SourceName {
        &self.source_name
    }

    /// Get the current sequence number (last emitted).
    pub fn sequence_number(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
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

/// Run an accumulator as 2-3 tokio tasks connected by a merge channel.
///
/// Socket-only mode (no event source):
/// ```text
/// ┌─────────────────┐     ┌─────────────────┐
/// │  Socket task     │──→  │  Processor task  │──→ BoundarySender ──→ Reactor
/// │  (always active) │     │  (calls process) │
/// └─────────────────┘     └─────────────────┘
/// ```
///
/// With event source (use [`accumulator_runtime_with_source`]):
/// ```text
/// ┌─────────────────┐
/// │  Event source    │──→ mpsc<Event> ──┐
/// │  (pulls events)  │                  │     ┌─────────────────┐
/// └─────────────────┘                  ├────→│  Processor task  │──→ BoundarySender ──→ Reactor
/// ┌─────────────────┐                  │     │  (calls process) │
/// │  Socket task     │──→ mpsc<Event> ──┘     └─────────────────┘
/// │  (always active) │
/// └─────────────────┘
/// ```
pub async fn accumulator_runtime<A: Accumulator>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
) {
    accumulator_runtime_inner::<A, NoEventSource>(acc, ctx, socket_rx, config, None).await
}

/// Run an accumulator with an active event source that pulls events from
/// an external system. The event source runs on its own task and pushes
/// raw bytes into the merge channel concurrently with the socket receiver.
pub async fn accumulator_runtime_with_source<A, S>(
    acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
    source: S,
) where
    A: Accumulator,
    S: EventSource,
{
    accumulator_runtime_inner(acc, ctx, socket_rx, config, Some(source)).await
}

/// Placeholder type for when no event source is provided.
struct NoEventSource;

#[async_trait::async_trait]
impl EventSource for NoEventSource {
    async fn run(
        self,
        _events: mpsc::Sender<Vec<u8>>,
        _shutdown: watch::Receiver<bool>,
    ) -> Result<(), AccumulatorError> {
        std::future::pending().await
    }
}

/// Inner runtime shared by both `accumulator_runtime` and `accumulator_runtime_with_source`.
async fn accumulator_runtime_inner<A: Accumulator, S: EventSource>(
    mut acc: A,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
    config: AccumulatorRuntimeConfig,
    event_source: Option<S>,
) {
    // Report starting health
    set_health(&ctx, AccumulatorHealth::Starting);

    // Initialize — may restore state from checkpoint
    if let Err(e) = acc.init(&ctx).await {
        tracing::error!(name = %ctx.name, "accumulator init failed: {}", e);
        return;
    }

    // Create merge channel — carries raw bytes from all sources
    let (event_tx, mut event_rx) = mpsc::channel::<Vec<u8>>(config.merge_channel_capacity);

    // Spawn event source task (or no-op wait if none provided)
    let name_loop = ctx.name.clone();
    let loop_handle = if let Some(source) = event_source {
        set_health(&ctx, AccumulatorHealth::Connecting);
        let shutdown_source = ctx.shutdown.clone();
        let event_tx_source = event_tx.clone();
        let name_source = name_loop.clone();
        tokio::spawn(async move {
            match source.run(event_tx_source, shutdown_source).await {
                Ok(()) => tracing::debug!(name = %name_source, "event source completed"),
                Err(e) => tracing::error!(name = %name_source, "event source failed: {}", e),
            }
        })
    } else {
        set_health(&ctx, AccumulatorHealth::SocketOnly);
        let mut shutdown_loop = ctx.shutdown.clone();
        tokio::spawn(async move {
            let _ = shutdown_loop.changed().await;
            tracing::debug!(name = %name_loop, "event loop task shutting down");
        })
    };

    // Spawn socket receiver task — forwards raw bytes without deserialization
    let event_tx_socket = event_tx.clone();
    let mut shutdown_socket = ctx.shutdown.clone();
    let name_socket = ctx.name.clone();
    let socket_handle = tokio::spawn(async move {
        let mut socket_rx = socket_rx;
        loop {
            tokio::select! {
                Some(bytes) = socket_rx.recv() => {
                    if event_tx_socket.send(bytes).await.is_err() {
                        break; // merge channel closed
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

    // Restore last poll output from checkpoint and emit to reactor
    if let Some(ref handle) = ctx.checkpoint {
        match handle.load::<P::Output>().await {
            Ok(Some(output)) => {
                tracing::info!(name = %ctx.name, "polling accumulator restored last output from checkpoint");
                if let Err(e) = ctx.output.send(&output).await {
                    tracing::warn!(name = %ctx.name, "failed to emit restored poll output: {}", e);
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load polling checkpoint: {}", e);
            }
        }
    }

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
                        // Checkpoint the last successful poll output
                        if let Some(ref handle) = ctx.checkpoint {
                            if let Err(e) = handle.save(&output).await {
                                tracing::warn!(name = %ctx.name, "polling checkpoint save failed: {}", e);
                            }
                        }
                    }
                }
            }
            Some(bytes) = socket_rx.recv() => {
                // Socket receives JSON from external sources
                match serde_json::from_slice::<P::Output>(&bytes) {
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
    /// The typed boundary produced from the batch.
    type Output: Serialize + Send + 'static;

    /// Process a batch of raw event bytes and optionally produce a boundary.
    /// The implementor owns deserialization — the runtime is format-agnostic.
    /// Called when the buffer is flushed. Empty batches are never passed.
    fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<Self::Output>;
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

    // Restore buffered events from checkpoint if available
    let mut buffer: Vec<Vec<u8>> = Vec::new();
    if let Some(ref handle) = ctx.checkpoint {
        match handle.load::<Vec<Vec<u8>>>().await {
            Ok(Some(raw_events)) => {
                buffer = raw_events;
                if !buffer.is_empty() {
                    tracing::info!(name = %ctx.name, events = buffer.len(), "batch buffer restored from checkpoint");
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load batch checkpoint: {}", e);
            }
        }
    }

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

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                buffer.push(bytes);
                // Persist buffer snapshot for crash resilience
                persist_batch_buffer(&ctx, &buffer).await;
                // Check size threshold
                if let Some(max) = config.max_buffer_size {
                    if buffer.len() >= max {
                        flush_batch(&mut acc, &mut buffer, &ctx).await;
                    }
                }
            }
            Some(()) = flush_rx.recv() => {
                flush_batch(&mut acc, &mut buffer, &ctx).await;
                // Clear checkpoint after flush (buffer is empty)
                persist_batch_buffer(&ctx, &[]).await;
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

/// Persist batch buffer snapshot to DAL for crash resilience (best-effort).
async fn persist_batch_buffer(ctx: &AccumulatorContext, buffer: &[Vec<u8>]) {
    if let Some(ref handle) = ctx.checkpoint {
        if let Err(e) = handle.save(&buffer.to_vec()).await {
            tracing::warn!(name = %ctx.name, "batch buffer checkpoint failed: {}", e);
        }
    }
}

/// Flush the buffer through the batch accumulator and send boundary if produced.
async fn flush_batch<B: BatchAccumulator>(
    acc: &mut B,
    buffer: &mut Vec<Vec<u8>>,
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

/// Persist last-emitted boundary with sequence number to DAL (best-effort, logs on failure).
async fn persist_boundary<T: Serialize>(ctx: &AccumulatorContext, boundary: &T) {
    if let Some(ref handle) = ctx.checkpoint {
        let bytes = match types::serialize(boundary) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(name = %ctx.name, "boundary persistence serialization failed: {}", e);
                return;
            }
        };
        let seq = ctx.output.sequence_number() as i64;
        if let Err(e) = handle
            .dal()
            .checkpoint()
            .save_boundary(handle.graph_name(), handle.accumulator_name(), bytes, seq)
            .await
        {
            tracing::warn!(name = %ctx.name, "boundary persistence failed: {}", e);
        }
    }
}

// =============================================================================
// State Accumulator
// =============================================================================

/// A state accumulator holds a bounded VecDeque<T> that receives values from
/// the computation graph (collector or mid-graph writes), persists to DAL on
/// every write, and loads from DAL on startup. Enables cyclic state patterns
/// where the graph's output feeds back as input on the next execution.
///
/// Capacity modes:
/// - `capacity > 0`: bounded — evicts oldest when at capacity
/// - `capacity < 0` (e.g., -1): unbounded — grows without limit
/// - `capacity == 0`: write-only sink — no history emitted back
pub struct StateAccumulator<T: Serialize + DeserializeOwned + Send + Clone + 'static> {
    buffer: std::collections::VecDeque<T>,
    capacity: i32,
}

impl<T: Serialize + DeserializeOwned + Send + Clone + 'static> StateAccumulator<T> {
    pub fn new(capacity: i32) -> Self {
        Self {
            buffer: std::collections::VecDeque::new(),
            capacity,
        }
    }
}

/// Run a state accumulator. Receives values via socket, appends to VecDeque,
/// evicts if over capacity, persists to DAL, and emits the full list as boundary.
///
/// On startup: loads from DAL and emits current list to reactor.
pub async fn state_accumulator_runtime<T: Serialize + DeserializeOwned + Send + Clone + 'static>(
    mut acc: StateAccumulator<T>,
    ctx: AccumulatorContext,
    socket_rx: mpsc::Receiver<Vec<u8>>,
) {
    set_health(&ctx, AccumulatorHealth::Starting);

    // Load from DAL on startup
    if let Some(ref handle) = ctx.checkpoint {
        match handle
            .dal()
            .checkpoint()
            .load_state_buffer(handle.graph_name(), handle.accumulator_name())
            .await
        {
            Ok(Some((data, _cap))) => {
                if let Ok(buffer) = types::deserialize::<std::collections::VecDeque<T>>(&data) {
                    acc.buffer = buffer;
                    tracing::info!(name = %ctx.name, entries = acc.buffer.len(), "state accumulator restored from DAL");
                }
            }
            Ok(None) => {
                tracing::debug!(name = %ctx.name, "no persisted state accumulator buffer found");
            }
            Err(e) => {
                tracing::warn!(name = %ctx.name, "failed to load state buffer: {}", e);
            }
        }

        // Emit current list to reactor immediately (so reactor has state on startup)
        if !acc.buffer.is_empty() && acc.capacity != 0 {
            let list: Vec<T> = acc.buffer.iter().cloned().collect();
            if let Err(e) = ctx.output.send(&list).await {
                tracing::error!(name = %ctx.name, "state accumulator initial emit failed: {}", e);
            }
        }
    }

    set_health(&ctx, AccumulatorHealth::SocketOnly);

    let mut shutdown = ctx.shutdown.clone();
    let mut socket_rx = socket_rx;

    loop {
        tokio::select! {
            Some(bytes) = socket_rx.recv() => {
                // Socket receives JSON from external sources
                match serde_json::from_slice::<T>(&bytes) {
                    Ok(value) => {
                        // Append to buffer
                        acc.buffer.push_back(value);

                        // Evict if over capacity (bounded mode)
                        if acc.capacity > 0 {
                            while acc.buffer.len() > acc.capacity as usize {
                                acc.buffer.pop_front();
                            }
                        }

                        // Persist to DAL
                        if let Some(ref handle) = ctx.checkpoint {
                            let data = match types::serialize(&acc.buffer) {
                                Ok(d) => d,
                                Err(e) => {
                                    tracing::warn!(name = %ctx.name, "state buffer serialization failed: {}", e);
                                    continue;
                                }
                            };
                            if let Err(e) = handle
                                .dal()
                                .checkpoint()
                                .save_state_buffer(
                                    handle.graph_name(),
                                    handle.accumulator_name(),
                                    data,
                                    acc.capacity,
                                )
                                .await
                            {
                                tracing::warn!(name = %ctx.name, "state buffer persistence failed: {}", e);
                            }
                        }

                        // Emit full list as boundary (unless write-only mode)
                        if acc.capacity != 0 {
                            let list: Vec<T> = acc.buffer.iter().cloned().collect();
                            if let Err(e) = ctx.output.send(&list).await {
                                tracing::error!(name = %ctx.name, "state accumulator emit failed: {}", e);
                            } else {
                                persist_boundary(&ctx, &list).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(name = %ctx.name, "state accumulator deserialize error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                tracing::debug!(name = %ctx.name, "state accumulator shutting down");
                break;
            }
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
        type Output = TestBoundary;

        fn process(&mut self, event: Vec<u8>) -> Option<TestBoundary> {
            let parsed: TestEvent = serde_json::from_slice(&event).ok()?;
            Some(TestBoundary {
                result: parsed.value * 2.0,
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
        let event_bytes = serde_json::to_vec(&event).unwrap();
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
            let bytes = serde_json::to_vec(&TestEvent { value: v }).unwrap();
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
        type Output = TestBoundary;

        fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<TestBoundary> {
            let sum: f64 = events
                .iter()
                .filter_map(|raw| serde_json::from_slice::<TestEvent>(raw).ok())
                .map(|e| e.value)
                .sum();
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
                .send(serde_json::to_vec(&TestEvent { value: v }).unwrap())
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
                .send(serde_json::to_vec(&TestEvent { value: v }).unwrap())
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
                .send(serde_json::to_vec(&TestEvent { value: v }).unwrap())
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
                .send(serde_json::to_vec(&TestEvent { value: v }).unwrap())
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
        type Output = TestBoundary;

        fn process(&mut self, event: Vec<u8>) -> Option<TestBoundary> {
            let parsed: TestEvent = serde_json::from_slice(&event).ok()?;
            // Only produce boundary for values > 5
            if parsed.value > 5.0 {
                Some(TestBoundary {
                    result: parsed.value,
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
            .send(serde_json::to_vec(&TestEvent { value: 3.0 }).unwrap())
            .await
            .unwrap();

        // Push event above threshold (should produce boundary)
        socket_tx
            .send(serde_json::to_vec(&TestEvent { value: 10.0 }).unwrap())
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
