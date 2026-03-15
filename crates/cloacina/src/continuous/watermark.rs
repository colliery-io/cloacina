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

//! Source watermark tracking for continuous scheduling.
//!
//! The `BoundaryLedger` stores per-data-source watermarks — assertions from
//! detectors about what data has been fully produced. Accumulators check these
//! to determine data completeness before firing.
//!
//! See CLOACI-S-0006 for the full specification.

use super::boundary::{BoundaryKind, ComputationBoundary};
use std::collections::HashMap;

/// Error from watermark operations.
#[derive(Debug, thiserror::Error)]
pub enum WatermarkError {
    /// Attempted to move a watermark backward.
    #[error("watermark for source '{source_name}' cannot move backward")]
    BackwardMovement { source_name: String },
    /// Incompatible boundary kinds for comparison.
    #[error("cannot compare watermarks of different kinds for source '{source_name}'")]
    IncompatibleKinds { source_name: String },
}

/// In-memory source watermark store.
///
/// Tracks data completeness per data source. Watermarks are monotonic —
/// they can only advance forward, never backward.
///
/// Thread safety: wrap in `Arc<RwLock<BoundaryLedger>>` for concurrent access.
#[derive(Debug, Default)]
pub struct BoundaryLedger {
    watermarks: HashMap<String, ComputationBoundary>,
}

impl BoundaryLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self {
            watermarks: HashMap::new(),
        }
    }

    /// Advance the watermark for a data source.
    ///
    /// Watermarks are monotonic — this rejects backward movement.
    /// First advance for a source always succeeds.
    pub fn advance(
        &mut self,
        source_name: &str,
        watermark: ComputationBoundary,
    ) -> Result<(), WatermarkError> {
        if let Some(existing) = self.watermarks.get(source_name) {
            if is_backward(&existing.kind, &watermark.kind)? {
                return Err(WatermarkError::BackwardMovement {
                    source_name: source_name.to_string(),
                });
            }
        }
        self.watermarks.insert(source_name.to_string(), watermark);
        Ok(())
    }

    /// Does the watermark for this source cover the given boundary?
    ///
    /// Returns `false` if the source has no watermark.
    pub fn covers(&self, source_name: &str, boundary: &ComputationBoundary) -> bool {
        match self.watermarks.get(source_name) {
            Some(watermark) => boundary_covered(&watermark.kind, &boundary.kind),
            None => false,
        }
    }

    /// Get the current watermark for a data source.
    pub fn watermark(&self, source_name: &str) -> Option<&ComputationBoundary> {
        self.watermarks.get(source_name)
    }

    /// Get all tracked source names.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        self.watermarks.keys().map(|s| s.as_str())
    }
}

/// Check if a new watermark would be a backward movement.
fn is_backward(existing: &BoundaryKind, proposed: &BoundaryKind) -> Result<bool, WatermarkError> {
    match (existing, proposed) {
        (
            BoundaryKind::TimeRange {
                end: existing_end, ..
            },
            BoundaryKind::TimeRange {
                end: proposed_end, ..
            },
        ) => Ok(proposed_end < existing_end),
        (
            BoundaryKind::OffsetRange {
                end: existing_end, ..
            },
            BoundaryKind::OffsetRange {
                end: proposed_end, ..
            },
        ) => Ok(proposed_end < existing_end),
        (
            BoundaryKind::Cursor {
                value: existing_val,
            },
            BoundaryKind::Cursor {
                value: proposed_val,
            },
        ) => {
            // Cursors are opaque — we can't determine ordering, so we never reject.
            // The user is asserting "this is the new position."
            Ok(false)
        }
        (
            BoundaryKind::FullState {
                value: existing_val,
            },
            BoundaryKind::FullState {
                value: proposed_val,
            },
        ) => {
            // FullState: same value = no movement (not backward), different = forward
            Ok(false)
        }
        _ => {
            // Different kinds — can't compare, allow the advance
            Ok(false)
        }
    }
}

/// Check if a watermark covers a boundary.
fn boundary_covered(watermark: &BoundaryKind, boundary: &BoundaryKind) -> bool {
    match (watermark, boundary) {
        (
            BoundaryKind::TimeRange { end: wm_end, .. },
            BoundaryKind::TimeRange { end: b_end, .. },
        ) => b_end <= wm_end,
        (
            BoundaryKind::OffsetRange { end: wm_end, .. },
            BoundaryKind::OffsetRange { end: b_end, .. },
        ) => b_end <= wm_end,
        (BoundaryKind::Cursor { value: wm_val }, BoundaryKind::Cursor { value: b_val }) => {
            wm_val == b_val
        }
        (BoundaryKind::FullState { value: wm_val }, BoundaryKind::FullState { value: b_val }) => {
            wm_val == b_val
        }
        _ => false, // Different kinds can't cover each other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn time_boundary(end_offset_hours: i64) -> ComputationBoundary {
        let now = Utc::now();
        ComputationBoundary {
            kind: BoundaryKind::TimeRange {
                start: now,
                end: now + Duration::hours(end_offset_hours),
            },
            metadata: None,
            emitted_at: now,
        }
    }

    fn offset_boundary(start: i64, end: i64) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::OffsetRange { start, end },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    fn cursor_boundary(value: &str) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    fn fullstate_boundary(value: &str) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::FullState {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    // --- advance tests ---

    #[test]
    fn test_advance_first_watermark_succeeds() {
        let mut ledger = BoundaryLedger::new();
        assert!(ledger.advance("src", offset_boundary(0, 100)).is_ok());
        assert!(ledger.watermark("src").is_some());
    }

    #[test]
    fn test_advance_forward_succeeds() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 100)).unwrap();
        assert!(ledger.advance("src", offset_boundary(0, 200)).is_ok());
    }

    #[test]
    fn test_advance_backward_rejected_offset() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 200)).unwrap();
        let result = ledger.advance("src", offset_boundary(0, 100));
        assert!(result.is_err());
        match result.unwrap_err() {
            WatermarkError::BackwardMovement { source_name } => {
                assert_eq!(source_name, "src");
            }
            other => panic!("expected BackwardMovement, got: {:?}", other),
        }
    }

    #[test]
    fn test_advance_same_value_accepted() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 100)).unwrap();
        assert!(ledger.advance("src", offset_boundary(0, 100)).is_ok());
    }

    #[test]
    fn test_advance_cursor_always_accepted() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", cursor_boundary("abc")).unwrap();
        // Cursors are opaque — no ordering, always accepted
        assert!(ledger.advance("src", cursor_boundary("def")).is_ok());
    }

    #[test]
    fn test_advance_fullstate_always_accepted() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", fullstate_boundary("v1")).unwrap();
        assert!(ledger.advance("src", fullstate_boundary("v2")).is_ok());
    }

    // --- covers tests ---

    #[test]
    fn test_covers_offset_within_watermark() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 200)).unwrap();
        assert!(ledger.covers("src", &offset_boundary(0, 100)));
        assert!(ledger.covers("src", &offset_boundary(0, 200)));
    }

    #[test]
    fn test_covers_offset_beyond_watermark() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 100)).unwrap();
        assert!(!ledger.covers("src", &offset_boundary(0, 200)));
    }

    #[test]
    fn test_covers_missing_source() {
        let ledger = BoundaryLedger::new();
        assert!(!ledger.covers("nonexistent", &offset_boundary(0, 100)));
    }

    #[test]
    fn test_covers_cursor_exact_match() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", cursor_boundary("abc")).unwrap();
        assert!(ledger.covers("src", &cursor_boundary("abc")));
        assert!(!ledger.covers("src", &cursor_boundary("xyz")));
    }

    #[test]
    fn test_covers_fullstate_exact_match() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", fullstate_boundary("v3")).unwrap();
        assert!(ledger.covers("src", &fullstate_boundary("v3")));
        assert!(!ledger.covers("src", &fullstate_boundary("v2")));
    }

    #[test]
    fn test_covers_different_kinds_returns_false() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("src", offset_boundary(0, 100)).unwrap();
        assert!(!ledger.covers("src", &cursor_boundary("abc")));
    }

    // --- watermark query ---

    #[test]
    fn test_watermark_returns_none_for_unknown() {
        let ledger = BoundaryLedger::new();
        assert!(ledger.watermark("unknown").is_none());
    }

    #[test]
    fn test_sources_iterator() {
        let mut ledger = BoundaryLedger::new();
        ledger.advance("a", offset_boundary(0, 100)).unwrap();
        ledger.advance("b", offset_boundary(0, 200)).unwrap();
        let mut sources: Vec<&str> = ledger.sources().collect();
        sources.sort();
        assert_eq!(sources, vec!["a", "b"]);
    }
}
