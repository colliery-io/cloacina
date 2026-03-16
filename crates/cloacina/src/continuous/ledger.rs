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

//! Execution ledger for continuous scheduling.
//!
//! An in-memory log recording all graph activity with configurable size limits.
//! The `ContinuousScheduler` writes to it; observers scan from cursors.
//!
//! See CLOACI-S-0007 for the full specification.

use super::boundary::ComputationBoundary;
use chrono::{DateTime, Utc};
use cloacina_workflow::Context;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::debug;

/// Events recorded in the execution ledger.
#[derive(Debug)]
pub enum LedgerEvent {
    /// A continuous task completed successfully.
    TaskCompleted {
        task: String,
        at: DateTime<Utc>,
        /// Output context snapshot (detector output is extracted from here).
        context: Context<serde_json::Value>,
    },
    /// A continuous task failed.
    TaskFailed {
        task: String,
        at: DateTime<Utc>,
        error: String,
    },
    /// A detector emitted boundaries for a data source.
    BoundaryEmitted {
        source: String,
        boundary: ComputationBoundary,
    },
    /// An accumulator drained and submitted work.
    AccumulatorDrained {
        task: String,
        boundary: ComputationBoundary,
    },
}

impl LedgerEvent {
    /// Get the task name if this event is task-related.
    pub fn task_name(&self) -> Option<&str> {
        match self {
            LedgerEvent::TaskCompleted { task, .. } => Some(task),
            LedgerEvent::TaskFailed { task, .. } => Some(task),
            LedgerEvent::AccumulatorDrained { task, .. } => Some(task),
            LedgerEvent::BoundaryEmitted { .. } => None,
        }
    }

    /// Returns true if this is a TaskCompleted event.
    pub fn is_task_completed(&self) -> bool {
        matches!(self, LedgerEvent::TaskCompleted { .. })
    }

    /// Returns true if this is a TaskFailed event.
    pub fn is_task_failed(&self) -> bool {
        matches!(self, LedgerEvent::TaskFailed { .. })
    }
}

/// Default maximum number of events in the ledger.
const DEFAULT_MAX_EVENTS: usize = 100_000;

/// Configuration for the execution ledger.
#[derive(Debug, Clone)]
pub struct LedgerConfig {
    /// Maximum number of events to retain. Oldest events are evicted when full.
    pub max_events: usize,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            max_events: DEFAULT_MAX_EVENTS,
        }
    }
}

/// In-memory log of graph activity with bounded size.
///
/// The ledger is the single observation point for all continuous scheduling
/// activity. Observers use cursor-based scanning to efficiently read new events.
/// When the ledger reaches `max_events`, the oldest events are evicted.
///
/// Cursors are absolute indices. A `base_offset` tracks how many events have
/// been evicted so that cursors remain valid across evictions.
///
/// Thread safety: wrap in `Arc<RwLock<ExecutionLedger>>` for concurrent access.
#[derive(Debug)]
pub struct ExecutionLedger {
    events: VecDeque<LedgerEvent>,
    /// Number of events evicted from the front. Cursor `n` maps to
    /// `events[n - base_offset]`.
    base_offset: usize,
    config: LedgerConfig,
    /// Notification channel for event-driven observers (e.g., LedgerTrigger).
    /// `notify_waiters()` is called on every `append()`.
    notify: Arc<Notify>,
}

impl Default for ExecutionLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionLedger {
    /// Create a new empty ledger with default configuration.
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            base_offset: 0,
            config: LedgerConfig::default(),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Create a new empty ledger with the given configuration.
    pub fn with_config(config: LedgerConfig) -> Self {
        Self {
            events: VecDeque::new(),
            base_offset: 0,
            config,
            notify: Arc::new(Notify::new()),
        }
    }

    /// Get a handle to the notification channel.
    /// Observers can `await` on `notified()` to be woken when new events arrive.
    pub fn subscribe(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    /// Append an event to the ledger, evicting the oldest if at capacity.
    /// Notifies all waiting observers after appending.
    pub fn append(&mut self, event: LedgerEvent) {
        // Evict oldest if at capacity
        if self.events.len() >= self.config.max_events {
            let to_evict = self.events.len() - self.config.max_events + 1;
            for _ in 0..to_evict {
                self.events.pop_front();
                self.base_offset += 1;
            }
            debug!(
                "Ledger evicted {} events, base_offset now {}",
                to_evict, self.base_offset
            );
        }
        self.events.push_back(event);
        // Wake up any observers waiting for new events
        self.notify.notify_waiters();
    }

    /// Get all events since the given cursor position.
    ///
    /// The cursor is an absolute index. If the cursor points to evicted events,
    /// returns all available events from the earliest retained event.
    /// If cursor >= len, returns empty.
    pub fn events_since(&self, cursor: usize) -> Vec<&LedgerEvent> {
        let effective_cursor = if cursor < self.base_offset {
            0
        } else {
            cursor - self.base_offset
        };

        if effective_cursor >= self.events.len() {
            Vec::new()
        } else {
            self.events.iter().skip(effective_cursor).collect()
        }
    }

    /// Get the current length (absolute index of next append).
    /// Usable as next cursor position.
    pub fn len(&self) -> usize {
        self.base_offset + self.events.len()
    }

    /// Check if the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get a specific event by absolute index.
    pub fn get(&self, index: usize) -> Option<&LedgerEvent> {
        if index < self.base_offset {
            return None; // evicted
        }
        self.events.get(index - self.base_offset)
    }

    /// Get the base offset (number of evicted events).
    pub fn base_offset(&self) -> usize {
        self.base_offset
    }

    /// Get the number of events currently retained.
    pub fn retained_count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::boundary::{BoundaryKind, ComputationBoundary};

    fn make_completed_event(task: &str) -> LedgerEvent {
        LedgerEvent::TaskCompleted {
            task: task.to_string(),
            at: Utc::now(),
            context: Context::new(),
        }
    }

    fn make_failed_event(task: &str, error: &str) -> LedgerEvent {
        LedgerEvent::TaskFailed {
            task: task.to_string(),
            at: Utc::now(),
            error: error.to_string(),
        }
    }

    fn make_boundary_event(source: &str) -> LedgerEvent {
        LedgerEvent::BoundaryEmitted {
            source: source.to_string(),
            boundary: ComputationBoundary {
                kind: BoundaryKind::Cursor {
                    value: "test".into(),
                },
                metadata: None,
                emitted_at: Utc::now(),
            },
        }
    }

    #[test]
    fn test_ledger_append_and_len() {
        let mut ledger = ExecutionLedger::new();
        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);

        ledger.append(make_completed_event("task_a"));
        assert_eq!(ledger.len(), 1);
        assert!(!ledger.is_empty());

        ledger.append(make_completed_event("task_b"));
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn test_ledger_events_since() {
        let mut ledger = ExecutionLedger::new();
        ledger.append(make_completed_event("a"));
        ledger.append(make_failed_event("b", "boom"));
        ledger.append(make_boundary_event("source_1"));

        // From beginning
        assert_eq!(ledger.events_since(0).len(), 3);

        // From middle
        assert_eq!(ledger.events_since(1).len(), 2);
        assert!(ledger.events_since(1)[0].is_task_failed());

        // From end
        assert_eq!(ledger.events_since(3).len(), 0);

        // Past end
        assert_eq!(ledger.events_since(100).len(), 0);
    }

    #[test]
    fn test_ledger_cursor_advancement() {
        let mut ledger = ExecutionLedger::new();

        // Simulate cursor-based reading
        let mut cursor: usize = 0;

        ledger.append(make_completed_event("a"));
        ledger.append(make_completed_event("b"));

        let new_events = ledger.events_since(cursor);
        assert_eq!(new_events.len(), 2);
        cursor = ledger.len(); // advance cursor

        // Add more events
        ledger.append(make_completed_event("c"));

        let new_events = ledger.events_since(cursor);
        assert_eq!(new_events.len(), 1);
        assert_eq!(new_events[0].task_name(), Some("c"));
        cursor = ledger.len();

        // No new events
        let new_events = ledger.events_since(cursor);
        assert_eq!(new_events.len(), 0);
    }

    #[test]
    fn test_ledger_event_helpers() {
        let completed = make_completed_event("task_a");
        assert!(completed.is_task_completed());
        assert!(!completed.is_task_failed());
        assert_eq!(completed.task_name(), Some("task_a"));

        let failed = make_failed_event("task_b", "error");
        assert!(!failed.is_task_completed());
        assert!(failed.is_task_failed());
        assert_eq!(failed.task_name(), Some("task_b"));

        let boundary = make_boundary_event("src");
        assert!(boundary.task_name().is_none());
    }

    #[test]
    fn test_ledger_get() {
        let mut ledger = ExecutionLedger::new();
        ledger.append(make_completed_event("a"));

        assert!(ledger.get(0).is_some());
        assert!(ledger.get(1).is_none());
    }

    // --- Eviction tests ---

    #[test]
    fn test_ledger_eviction_on_overflow() {
        let config = LedgerConfig { max_events: 3 };
        let mut ledger = ExecutionLedger::with_config(config);

        ledger.append(make_completed_event("a")); // index 0
        ledger.append(make_completed_event("b")); // index 1
        ledger.append(make_completed_event("c")); // index 2
        assert_eq!(ledger.retained_count(), 3);
        assert_eq!(ledger.base_offset(), 0);

        // This should evict "a"
        ledger.append(make_completed_event("d")); // index 3
        assert_eq!(ledger.retained_count(), 3);
        assert_eq!(ledger.base_offset(), 1);
        assert_eq!(ledger.len(), 4); // absolute index

        // "a" is evicted
        assert!(ledger.get(0).is_none());
        // "b" still accessible
        assert!(ledger.get(1).is_some());
        assert_eq!(ledger.get(1).unwrap().task_name(), Some("b"));
    }

    #[test]
    fn test_ledger_cursor_adjustment_after_eviction() {
        let config = LedgerConfig { max_events: 2 };
        let mut ledger = ExecutionLedger::with_config(config);

        ledger.append(make_completed_event("a")); // abs 0
        ledger.append(make_completed_event("b")); // abs 1

        let mut cursor = ledger.len(); // cursor = 2

        ledger.append(make_completed_event("c")); // abs 2, evicts "a"
        ledger.append(make_completed_event("d")); // abs 3, evicts "b"

        // Cursor 2 still works — it points to "c"
        let events = ledger.events_since(cursor);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].task_name(), Some("c"));
        assert_eq!(events[1].task_name(), Some("d"));
    }

    #[test]
    fn test_ledger_cursor_before_base_offset_returns_all_retained() {
        let config = LedgerConfig { max_events: 2 };
        let mut ledger = ExecutionLedger::with_config(config);

        ledger.append(make_completed_event("a"));
        ledger.append(make_completed_event("b"));
        ledger.append(make_completed_event("c")); // evicts "a"

        // Cursor 0 is before base_offset (1) — should return all retained events
        let events = ledger.events_since(0);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].task_name(), Some("b"));
        assert_eq!(events[1].task_name(), Some("c"));
    }

    // --- Hardening: stress tests ---

    #[test]
    fn test_ledger_heavy_eviction_stress() {
        let config = LedgerConfig { max_events: 10 };
        let mut ledger = ExecutionLedger::with_config(config);

        // Append 10,000 events — only last 10 should be retained
        for i in 0..10_000 {
            ledger.append(make_completed_event(&format!("task_{}", i)));
        }

        assert_eq!(ledger.retained_count(), 10);
        assert_eq!(ledger.len(), 10_000);
        assert_eq!(ledger.base_offset(), 9990);

        // Latest event should be accessible
        assert!(ledger.get(9999).is_some());
        assert_eq!(ledger.get(9999).unwrap().task_name(), Some("task_9999"));

        // Early events should be evicted
        assert!(ledger.get(0).is_none());
        assert!(ledger.get(9989).is_none());
    }

    #[test]
    fn test_ledger_cursor_tracking_through_eviction() {
        let config = LedgerConfig { max_events: 5 };
        let mut ledger = ExecutionLedger::with_config(config);

        let mut cursor: usize = 0;

        // Simulate multiple poll cycles with eviction
        for batch in 0..20 {
            // Append 3 events per cycle
            for j in 0..3 {
                ledger.append(make_completed_event(&format!("b{}_{}", batch, j)));
            }

            // Read new events from cursor
            let events = ledger.events_since(cursor);
            // Should always get the events we just appended (possibly more if cursor was behind)
            assert!(!events.is_empty(), "batch {} should have events", batch);

            cursor = ledger.len();
        }

        // After 20 batches of 3, total = 60, retained = 5
        assert_eq!(ledger.len(), 60);
        assert_eq!(ledger.retained_count(), 5);

        // Cursor at end should give empty
        assert!(ledger.events_since(cursor).is_empty());
    }

    #[test]
    fn test_ledger_notify_on_append() {
        let ledger = ExecutionLedger::new();
        let notify = ledger.subscribe();
        // Just verify subscribe works and returns a handle
        assert!(std::sync::Arc::strong_count(&notify) >= 2);
    }
}
