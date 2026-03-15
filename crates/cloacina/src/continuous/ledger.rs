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
//! An in-memory append-only log recording all graph activity. The
//! `ContinuousScheduler` writes to it; observers scan from cursors.
//!
//! See CLOACI-S-0007 for the full specification.

use super::boundary::ComputationBoundary;
use chrono::{DateTime, Utc};
use cloacina_workflow::Context;

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

/// In-memory append-only log of graph activity.
///
/// The ledger is the single observation point for all continuous scheduling
/// activity. Observers use cursor-based scanning to efficiently read new events.
///
/// Thread safety: wrap in `Arc<RwLock<ExecutionLedger>>` for concurrent access.
#[derive(Debug, Default)]
pub struct ExecutionLedger {
    events: Vec<LedgerEvent>,
}

impl ExecutionLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append an event to the ledger.
    pub fn append(&mut self, event: LedgerEvent) {
        self.events.push(event);
    }

    /// Get all events since the given cursor position.
    ///
    /// The cursor is an index into the events vector. Returns a slice
    /// of events from `cursor` to the end. If cursor >= len, returns empty.
    pub fn events_since(&self, cursor: usize) -> &[LedgerEvent] {
        if cursor >= self.events.len() {
            &[]
        } else {
            &self.events[cursor..]
        }
    }

    /// Get the current length of the ledger (usable as next cursor position).
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get a specific event by index.
    pub fn get(&self, index: usize) -> Option<&LedgerEvent> {
        self.events.get(index)
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
}
