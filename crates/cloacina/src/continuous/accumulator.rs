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

//! Signal accumulator for continuous scheduling.
//!
//! Per-edge stateful component that buffers boundaries, coalesces them,
//! and decides when to fire the downstream task.
//!
//! See CLOACI-S-0005 for the full specification.

use super::boundary::{coalesce, validate_boundary, BufferedBoundary, ComputationBoundary};
use super::trigger_policy::TriggerPolicy;
use super::watermark::BoundaryLedger;
use chrono::{DateTime, Duration, Utc};
use cloacina_workflow::Context;
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

/// Observable state for monitoring and backpressure detection.
#[derive(Debug, Clone)]
pub struct AccumulatorMetrics {
    /// Number of boundaries currently buffered.
    pub buffered_count: usize,
    /// Emitted_at of the oldest boundary in the buffer.
    pub oldest_boundary_emitted_at: Option<DateTime<Utc>>,
    /// Emitted_at of the newest boundary in the buffer.
    pub newest_boundary_emitted_at: Option<DateTime<Utc>>,
    /// Maximum ingestion lag across all buffered boundaries.
    pub max_lag: Option<Duration>,
    /// Total number of boundaries received since creation.
    pub total_boundaries_received: u64,
    /// Total number of drains since creation.
    pub drain_count: u64,
}

/// Per-edge metrics snapshot for the scheduler.
#[derive(Debug, Clone)]
pub struct EdgeMetrics {
    /// Data source name for this edge.
    pub source: String,
    /// Task name for this edge.
    pub task: String,
    /// Accumulator metrics snapshot.
    pub accumulator: AccumulatorMetrics,
}

/// Result of receiving a boundary into an accumulator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReceiveResult {
    /// Boundary accepted into the buffer.
    Accepted,
    /// Boundary accepted, but an older boundary was dropped (buffer was full).
    AcceptedWithDrop,
}

/// Per-edge stateful component that buffers boundaries and decides when to fire.
pub trait SignalAccumulator: Send + Sync {
    /// Buffer a boundary event. Returns whether the boundary was accepted
    /// cleanly or if backpressure occurred (oldest boundary dropped).
    fn receive(&mut self, boundary: ComputationBoundary) -> ReceiveResult;

    /// Should the downstream task run now?
    fn is_ready(&self) -> bool;

    /// Coalesce buffered boundaries and produce a partial context fragment.
    /// Clears the buffer.
    fn drain(&mut self) -> Context<serde_json::Value>;

    /// Observable state for monitoring.
    fn metrics(&self) -> AccumulatorMetrics;

    /// What boundary has this accumulator processed up to?
    /// Updated on each drain().
    fn consumer_watermark(&self) -> Option<&ComputationBoundary>;

    /// Set the consumer watermark from persisted state on restart.
    fn set_consumer_watermark(&mut self, watermark: ComputationBoundary);

    /// Atomically check readiness and drain if ready.
    /// Returns `Some(context)` if the accumulator was ready and drained,
    /// `None` if not ready. This avoids the double-lock race where readiness
    /// could change between a separate `is_ready()` check and `drain()` call.
    fn try_drain(&mut self) -> Option<Context<serde_json::Value>> {
        if self.is_ready() {
            Some(self.drain())
        } else {
            None
        }
    }
}

/// Default maximum buffer size for accumulators.
const DEFAULT_MAX_BUFFER_SIZE: usize = 10_000;

/// Simple accumulator with no watermark awareness.
///
/// Fires based on the injected `TriggerPolicy` alone. Suitable for
/// config changes, full-state data, and non-temporal data sources.
pub struct SimpleAccumulator {
    buffer: Vec<BufferedBoundary>,
    policy: Box<dyn TriggerPolicy>,
    watermark: Option<ComputationBoundary>,
    total_received: u64,
    drain_count: u64,
    max_buffer_size: usize,
    dropped_count: u64,
    // Cached metrics — updated incrementally on receive, cleared on drain
    cached_oldest: Option<DateTime<Utc>>,
    cached_newest: Option<DateTime<Utc>>,
    cached_max_lag: Option<Duration>,
}

impl SimpleAccumulator {
    /// Create a new SimpleAccumulator with the given trigger policy.
    pub fn new(policy: Box<dyn TriggerPolicy>) -> Self {
        Self {
            buffer: Vec::new(),
            policy,
            watermark: None,
            total_received: 0,
            drain_count: 0,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            dropped_count: 0,
            cached_oldest: None,
            cached_newest: None,
            cached_max_lag: None,
        }
    }

    /// Create a new SimpleAccumulator with a custom buffer size limit.
    pub fn with_max_buffer(policy: Box<dyn TriggerPolicy>, max_buffer_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            policy,
            watermark: None,
            total_received: 0,
            drain_count: 0,
            max_buffer_size,
            dropped_count: 0,
            cached_oldest: None,
            cached_newest: None,
            cached_max_lag: None,
        }
    }
}

impl SignalAccumulator for SimpleAccumulator {
    fn receive(&mut self, boundary: ComputationBoundary) -> ReceiveResult {
        let result = if self.buffer.len() >= self.max_buffer_size {
            self.buffer.remove(0);
            self.dropped_count += 1;
            // Invalidate caches that may have been affected by the drop
            self.cached_oldest = self.buffer.first().map(|b| b.boundary.emitted_at);
            // Max lag must be recomputed — the dropped boundary may have been the max
            self.cached_max_lag = self.buffer.iter().map(|b| b.lag()).max();
            warn!(
                "SimpleAccumulator buffer full (max={}), dropped oldest boundary (total dropped: {})",
                self.max_buffer_size, self.dropped_count
            );
            ReceiveResult::AcceptedWithDrop
        } else {
            ReceiveResult::Accepted
        };
        let buffered = BufferedBoundary::new(boundary);
        let emitted = buffered.boundary.emitted_at;
        let lag = buffered.lag();
        self.cached_newest = Some(emitted);
        if self.cached_oldest.is_none() || emitted < self.cached_oldest.unwrap() {
            self.cached_oldest = Some(emitted);
        }
        if self.cached_max_lag.is_none() || lag > self.cached_max_lag.unwrap() {
            self.cached_max_lag = Some(lag);
        }
        self.buffer.push(buffered);
        self.total_received += 1;
        result
    }

    fn is_ready(&self) -> bool {
        self.policy.should_fire(&self.buffer)
    }

    fn drain(&mut self) -> Context<serde_json::Value> {
        let mut ctx = Context::new();

        if self.buffer.is_empty() {
            return ctx;
        }

        let boundaries: Vec<ComputationBoundary> =
            self.buffer.iter().map(|b| b.boundary.clone()).collect();
        let signals_coalesced = boundaries.len();

        // Calculate max lag before draining
        let max_lag_ms = self
            .buffer
            .iter()
            .map(|b| b.lag().num_milliseconds())
            .max()
            .unwrap_or(0);

        // Coalesce boundaries
        if let Some(coalesced) = coalesce(&boundaries) {
            // Re-validate Custom boundaries after coalescing
            if let Err(e) = validate_boundary(&coalesced) {
                let _ = ctx.insert("__validation_error", json!(e));
            }

            let boundary_value = serde_json::to_value(&coalesced).unwrap_or(json!(null));
            let _ = ctx.insert("__boundary", boundary_value);

            // Update consumer watermark
            self.watermark = Some(coalesced);
        }

        let _ = ctx.insert("__signals_coalesced", json!(signals_coalesced));
        let _ = ctx.insert("__accumulator_lag_ms", json!(max_lag_ms));

        self.buffer.clear();
        self.drain_count += 1;
        self.policy.mark_drained();
        // Clear cached metrics
        self.cached_oldest = None;
        self.cached_newest = None;
        self.cached_max_lag = None;

        ctx
    }

    fn metrics(&self) -> AccumulatorMetrics {
        AccumulatorMetrics {
            buffered_count: self.buffer.len(),
            oldest_boundary_emitted_at: self.cached_oldest,
            newest_boundary_emitted_at: self.cached_newest,
            max_lag: self.cached_max_lag,
            total_boundaries_received: self.total_received,
            drain_count: self.drain_count,
        }
    }

    fn consumer_watermark(&self) -> Option<&ComputationBoundary> {
        self.watermark.as_ref()
    }

    fn set_consumer_watermark(&mut self, watermark: ComputationBoundary) {
        self.watermark = Some(watermark);
    }
}

/// How the accumulator uses source watermarks for readiness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatermarkMode {
    /// Wait for source watermark to cover the pending boundary before firing.
    WaitForWatermark,
    /// Fire when trigger policy says so, regardless of watermark.
    BestEffort,
}

/// Windowed accumulator with source watermark awareness.
///
/// Extends `SimpleAccumulator` behavior with an additional watermark check:
/// in `WaitForWatermark` mode, `is_ready()` returns false until the source
/// watermark confirms data completeness for the pending boundary.
pub struct WindowedAccumulator {
    buffer: Vec<BufferedBoundary>,
    policy: Box<dyn TriggerPolicy>,
    watermark: Option<ComputationBoundary>,
    watermark_mode: WatermarkMode,
    boundary_ledger: Arc<RwLock<BoundaryLedger>>,
    source_name: String,
    total_received: u64,
    drain_count: u64,
    max_buffer_size: usize,
    dropped_count: u64,
    cached_oldest: Option<DateTime<Utc>>,
    cached_newest: Option<DateTime<Utc>>,
    cached_max_lag: Option<Duration>,
}

impl WindowedAccumulator {
    /// Create a new WindowedAccumulator.
    pub fn new(
        policy: Box<dyn TriggerPolicy>,
        watermark_mode: WatermarkMode,
        boundary_ledger: Arc<RwLock<BoundaryLedger>>,
        source_name: String,
    ) -> Self {
        Self {
            buffer: Vec::new(),
            policy,
            watermark: None,
            watermark_mode,
            boundary_ledger,
            source_name,
            total_received: 0,
            drain_count: 0,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            dropped_count: 0,
            cached_oldest: None,
            cached_newest: None,
            cached_max_lag: None,
        }
    }

    /// Create a new WindowedAccumulator with a custom buffer size limit.
    pub fn with_max_buffer(
        policy: Box<dyn TriggerPolicy>,
        watermark_mode: WatermarkMode,
        boundary_ledger: Arc<RwLock<BoundaryLedger>>,
        source_name: String,
        max_buffer_size: usize,
    ) -> Self {
        Self {
            buffer: Vec::new(),
            policy,
            watermark: None,
            watermark_mode,
            boundary_ledger,
            source_name,
            total_received: 0,
            drain_count: 0,
            max_buffer_size,
            dropped_count: 0,
            cached_oldest: None,
            cached_newest: None,
            cached_max_lag: None,
        }
    }

    /// Get the coalesced pending boundary without draining.
    pub fn pending_boundary(&self) -> Option<ComputationBoundary> {
        let boundaries: Vec<ComputationBoundary> =
            self.buffer.iter().map(|b| b.boundary.clone()).collect();
        coalesce(&boundaries)
    }
}

impl SignalAccumulator for WindowedAccumulator {
    fn receive(&mut self, boundary: ComputationBoundary) -> ReceiveResult {
        let result = if self.buffer.len() >= self.max_buffer_size {
            self.buffer.remove(0);
            self.dropped_count += 1;
            self.cached_oldest = self.buffer.first().map(|b| b.boundary.emitted_at);
            self.cached_max_lag = self.buffer.iter().map(|b| b.lag()).max();
            warn!(
                "WindowedAccumulator[{}] buffer full (max={}), dropped oldest boundary (total dropped: {})",
                self.source_name, self.max_buffer_size, self.dropped_count
            );
            ReceiveResult::AcceptedWithDrop
        } else {
            ReceiveResult::Accepted
        };
        let buffered = BufferedBoundary::new(boundary);
        let emitted = buffered.boundary.emitted_at;
        let lag = buffered.lag();
        self.cached_newest = Some(emitted);
        if self.cached_oldest.is_none() || emitted < self.cached_oldest.unwrap() {
            self.cached_oldest = Some(emitted);
        }
        if self.cached_max_lag.is_none() || lag > self.cached_max_lag.unwrap() {
            self.cached_max_lag = Some(lag);
        }
        self.buffer.push(buffered);
        self.total_received += 1;
        result
    }

    fn is_ready(&self) -> bool {
        if !self.policy.should_fire(&self.buffer) {
            return false;
        }
        match self.watermark_mode {
            WatermarkMode::BestEffort => true,
            WatermarkMode::WaitForWatermark => {
                if let Some(pending) = self.pending_boundary() {
                    let bl = self.boundary_ledger.read();
                    bl.covers(&self.source_name, &pending)
                } else {
                    false
                }
            }
        }
    }

    fn drain(&mut self) -> Context<serde_json::Value> {
        let mut ctx = Context::new();
        if self.buffer.is_empty() {
            return ctx;
        }

        let boundaries: Vec<ComputationBoundary> =
            self.buffer.iter().map(|b| b.boundary.clone()).collect();
        let signals_coalesced = boundaries.len();
        let max_lag_ms = self
            .buffer
            .iter()
            .map(|b| b.lag().num_milliseconds())
            .max()
            .unwrap_or(0);

        if let Some(coalesced) = coalesce(&boundaries) {
            // Re-validate Custom boundaries after coalescing
            if let Err(e) = validate_boundary(&coalesced) {
                let _ = ctx.insert("__validation_error", json!(e));
            }

            let boundary_value = serde_json::to_value(&coalesced).unwrap_or(json!(null));
            let _ = ctx.insert("__boundary", boundary_value);
            self.watermark = Some(coalesced);
        }

        let _ = ctx.insert("__signals_coalesced", json!(signals_coalesced));
        let _ = ctx.insert("__accumulator_lag_ms", json!(max_lag_ms));

        self.buffer.clear();
        self.drain_count += 1;
        self.policy.mark_drained();
        self.cached_oldest = None;
        self.cached_newest = None;
        self.cached_max_lag = None;
        ctx
    }

    fn metrics(&self) -> AccumulatorMetrics {
        AccumulatorMetrics {
            buffered_count: self.buffer.len(),
            oldest_boundary_emitted_at: self.cached_oldest,
            newest_boundary_emitted_at: self.cached_newest,
            max_lag: self.cached_max_lag,
            total_boundaries_received: self.total_received,
            drain_count: self.drain_count,
        }
    }

    fn consumer_watermark(&self) -> Option<&ComputationBoundary> {
        self.watermark.as_ref()
    }

    fn set_consumer_watermark(&mut self, watermark: ComputationBoundary) {
        self.watermark = Some(watermark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::boundary::BoundaryKind;
    use crate::continuous::trigger_policy::{BoundaryCount, Immediate};

    fn make_offset_boundary(start: i64, end: i64) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::OffsetRange { start, end },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    fn make_cursor_boundary(value: &str) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: value.into(),
            },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    #[test]
    fn test_simple_accumulator_receive_and_drain() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        acc.receive(make_offset_boundary(0, 100));
        acc.receive(make_offset_boundary(100, 200));

        assert!(acc.is_ready());

        let ctx = acc.drain();
        assert_eq!(ctx.get("__signals_coalesced"), Some(&json!(2)));

        // After drain, buffer is empty
        assert!(!acc.is_ready());
        assert_eq!(acc.metrics().buffered_count, 0);
    }

    #[test]
    fn test_simple_accumulator_coalesces_on_drain() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        acc.receive(make_offset_boundary(0, 100));
        acc.receive(make_offset_boundary(100, 200));
        acc.receive(make_offset_boundary(50, 150));

        let ctx = acc.drain();
        let boundary_value = ctx.get("__boundary").unwrap();
        let kind = &boundary_value["kind"];
        assert_eq!(kind["start"], 0);
        assert_eq!(kind["end"], 200);
    }

    #[test]
    fn test_simple_accumulator_updates_consumer_watermark() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        assert!(acc.consumer_watermark().is_none());

        acc.receive(make_offset_boundary(0, 100));
        acc.drain();

        assert!(acc.consumer_watermark().is_some());
        if let Some(wm) = acc.consumer_watermark() {
            if let BoundaryKind::OffsetRange { start, end } = &wm.kind {
                assert_eq!(*start, 0);
                assert_eq!(*end, 100);
            } else {
                panic!("expected OffsetRange watermark");
            }
        }
    }

    #[test]
    fn test_simple_accumulator_empty_drain() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));
        let ctx = acc.drain();
        // Empty drain produces empty context
        assert!(ctx.get("__boundary").is_none());
        assert!(ctx.get("__signals_coalesced").is_none());
    }

    #[test]
    fn test_simple_accumulator_metrics() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        assert_eq!(acc.metrics().buffered_count, 0);
        assert!(acc.metrics().oldest_boundary_emitted_at.is_none());

        acc.receive(make_cursor_boundary("a"));
        acc.receive(make_cursor_boundary("b"));

        let metrics = acc.metrics();
        assert_eq!(metrics.buffered_count, 2);
        assert!(metrics.oldest_boundary_emitted_at.is_some());
        assert!(metrics.newest_boundary_emitted_at.is_some());
        assert!(metrics.max_lag.is_some());
    }

    #[test]
    fn test_simple_accumulator_lag_tracking() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        // Create a boundary emitted 500ms ago
        let boundary = ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: "test".into(),
            },
            metadata: None,
            emitted_at: Utc::now() - chrono::Duration::milliseconds(500),
        };
        acc.receive(boundary);

        let ctx = acc.drain();
        let lag = ctx.get("__accumulator_lag_ms").unwrap().as_i64().unwrap();
        assert!(lag >= 400, "lag should be at least 400ms, got {}", lag);
    }

    #[test]
    fn test_simple_accumulator_multiple_drain_cycles() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        // First cycle
        acc.receive(make_offset_boundary(0, 100));
        let ctx1 = acc.drain();
        assert_eq!(ctx1.get("__signals_coalesced"), Some(&json!(1)));

        // Second cycle
        acc.receive(make_offset_boundary(100, 200));
        acc.receive(make_offset_boundary(200, 300));
        let ctx2 = acc.drain();
        assert_eq!(ctx2.get("__signals_coalesced"), Some(&json!(2)));

        // Watermark should reflect second drain
        if let Some(wm) = acc.consumer_watermark() {
            if let BoundaryKind::OffsetRange { start, end } = &wm.kind {
                assert_eq!(*start, 100);
                assert_eq!(*end, 300);
            }
        }
    }

    // --- WindowedAccumulator tests ---

    #[test]
    fn test_windowed_best_effort_fires_immediately() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));
        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::BestEffort,
            bl,
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        assert!(acc.is_ready());
    }

    #[test]
    fn test_windowed_wait_for_watermark_blocks_without_watermark() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));
        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::WaitForWatermark,
            bl,
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        // No watermark set — should NOT be ready
        assert!(!acc.is_ready());
    }

    #[test]
    fn test_windowed_wait_for_watermark_fires_when_covered() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));

        // Set watermark covering [0, 200)
        {
            let mut ledger = bl.write();
            ledger.advance("src", make_offset_boundary(0, 200)).unwrap();
        }

        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::WaitForWatermark,
            bl,
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        // Watermark [0,200) covers boundary [0,100) — should be ready
        assert!(acc.is_ready());
    }

    #[test]
    fn test_windowed_wait_for_watermark_blocks_when_not_covered() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));

        // Set watermark only covering [0, 50)
        {
            let mut ledger = bl.write();
            ledger.advance("src", make_offset_boundary(0, 50)).unwrap();
        }

        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::WaitForWatermark,
            bl,
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        // Watermark [0,50) does NOT cover boundary [0,100) — should NOT be ready
        assert!(!acc.is_ready());
    }

    #[test]
    fn test_windowed_watermark_advance_unblocks() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));

        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::WaitForWatermark,
            bl.clone(),
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        assert!(!acc.is_ready()); // No watermark yet

        // Advance watermark
        {
            let mut ledger = bl.write();
            ledger.advance("src", make_offset_boundary(0, 200)).unwrap();
        }

        assert!(acc.is_ready()); // Now covered
    }

    #[test]
    fn test_windowed_drain_produces_context() {
        let bl = Arc::new(RwLock::new(BoundaryLedger::new()));
        {
            let mut ledger = bl.write();
            ledger.advance("src", make_offset_boundary(0, 500)).unwrap();
        }

        let mut acc = WindowedAccumulator::new(
            Box::new(Immediate),
            WatermarkMode::WaitForWatermark,
            bl,
            "src".into(),
        );

        acc.receive(make_offset_boundary(0, 100));
        acc.receive(make_offset_boundary(100, 200));

        let ctx = acc.drain();
        assert_eq!(ctx.get("__signals_coalesced"), Some(&json!(2)));
        assert!(ctx.get("__boundary").is_some());
        assert!(acc.consumer_watermark().is_some());
    }

    // --- Buffer overflow / backpressure tests ---

    #[test]
    fn test_simple_accumulator_buffer_overflow_drops_oldest() {
        let mut acc = SimpleAccumulator::with_max_buffer(Box::new(Immediate), 3);

        acc.receive(make_offset_boundary(0, 100));
        acc.receive(make_offset_boundary(100, 200));
        acc.receive(make_offset_boundary(200, 300));
        assert_eq!(acc.metrics().buffered_count, 3);

        // This should drop the oldest (0, 100)
        let result = acc.receive(make_offset_boundary(300, 400));
        assert_eq!(result, ReceiveResult::AcceptedWithDrop);
        assert_eq!(acc.metrics().buffered_count, 3);

        // Drain and verify coalesced boundary starts at 100, not 0
        let ctx = acc.drain();
        let boundary = ctx.get("__boundary").unwrap();
        assert_eq!(boundary["kind"]["start"], 100);
        assert_eq!(boundary["kind"]["end"], 400);
    }

    #[test]
    fn test_simple_accumulator_buffer_within_limit() {
        let mut acc = SimpleAccumulator::with_max_buffer(Box::new(Immediate), 100);

        for i in 0..50 {
            let result = acc.receive(make_offset_boundary(i * 10, (i + 1) * 10));
            assert_eq!(result, ReceiveResult::Accepted);
        }
        assert_eq!(acc.metrics().buffered_count, 50);
    }

    #[test]
    fn test_metrics_accurate_after_interleaved_receive_drain() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));

        // Receive 3 boundaries
        acc.receive(make_offset_boundary(0, 100));
        acc.receive(make_offset_boundary(100, 200));
        acc.receive(make_offset_boundary(200, 300));

        let m = acc.metrics();
        assert_eq!(m.buffered_count, 3);
        assert!(m.oldest_boundary_emitted_at.is_some());
        assert!(m.newest_boundary_emitted_at.is_some());
        assert_eq!(m.total_boundaries_received, 3);

        // Drain
        acc.drain();
        let m = acc.metrics();
        assert_eq!(m.buffered_count, 0);
        assert!(m.oldest_boundary_emitted_at.is_none());
        assert!(m.newest_boundary_emitted_at.is_none());
        assert_eq!(m.drain_count, 1);

        // Receive again
        acc.receive(make_offset_boundary(300, 400));
        let m = acc.metrics();
        assert_eq!(m.buffered_count, 1);
        assert!(m.oldest_boundary_emitted_at.is_some());
        assert_eq!(m.total_boundaries_received, 4);
        assert_eq!(m.drain_count, 1);
    }

    #[test]
    fn test_set_consumer_watermark_enables_late_detection() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));
        assert!(acc.consumer_watermark().is_none());

        // Set watermark as if restored from persistence
        acc.set_consumer_watermark(make_offset_boundary(0, 500));
        assert!(acc.consumer_watermark().is_some());

        let wm = acc.consumer_watermark().unwrap();
        if let BoundaryKind::OffsetRange { end, .. } = &wm.kind {
            assert_eq!(*end, 500);
        } else {
            panic!("expected OffsetRange");
        }
    }

    #[test]
    fn test_try_drain_when_not_ready() {
        let mut acc = SimpleAccumulator::new(Box::new(BoundaryCount::new(10)));
        acc.receive(make_offset_boundary(0, 100)); // only 1, need 10
        assert!(acc.try_drain().is_none());
    }

    #[test]
    fn test_try_drain_when_ready() {
        let mut acc = SimpleAccumulator::new(Box::new(Immediate));
        acc.receive(make_offset_boundary(0, 100));
        let ctx = acc.try_drain();
        assert!(ctx.is_some());
        assert_eq!(acc.metrics().buffered_count, 0);
    }
}
