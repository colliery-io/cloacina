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
            if is_backward(existing, &watermark)? {
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
            Some(watermark) => boundary_covered(watermark, boundary),
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
///
/// For TimeRange/OffsetRange: compares end positions (structural ordering).
/// For Cursor/FullState: compares emitted_at timestamps (temporal ordering).
///   Opaque values can't be structurally compared, so we use the detector's
///   emission time as a monotonicity proxy. A watermark emitted earlier than
///   the current one is rejected.
fn is_backward(
    existing: &ComputationBoundary,
    proposed: &ComputationBoundary,
) -> Result<bool, WatermarkError> {
    match (&existing.kind, &proposed.kind) {
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
        (BoundaryKind::Cursor { .. }, BoundaryKind::Cursor { .. }) => {
            // Cursor values are opaque — use emitted_at for monotonicity.
            Ok(proposed.emitted_at < existing.emitted_at)
        }
        (BoundaryKind::FullState { .. }, BoundaryKind::FullState { .. }) => {
            // FullState values are opaque — use emitted_at for monotonicity.
            // Same emitted_at is allowed (idempotent re-assertion).
            Ok(proposed.emitted_at < existing.emitted_at)
        }
        _ => {
            // Different kinds — can't compare, allow the advance
            Ok(false)
        }
    }
}

/// Check if a watermark covers a boundary.
///
/// For TimeRange/OffsetRange: boundary end <= watermark end (structural).
/// For Cursor/FullState: boundary emitted_at <= watermark emitted_at (temporal).
///   A boundary emitted at or before the watermark's emission time is considered
///   covered — the watermark represents a later point in time.
fn boundary_covered(watermark: &ComputationBoundary, boundary: &ComputationBoundary) -> bool {
    match (&watermark.kind, &boundary.kind) {
        (
            BoundaryKind::TimeRange { end: wm_end, .. },
            BoundaryKind::TimeRange { end: b_end, .. },
        ) => b_end <= wm_end,
        (
            BoundaryKind::OffsetRange { end: wm_end, .. },
            BoundaryKind::OffsetRange { end: b_end, .. },
        ) => b_end <= wm_end,
        (BoundaryKind::Cursor { .. }, BoundaryKind::Cursor { .. }) => {
            // Covered if the boundary was emitted at or before the watermark
            boundary.emitted_at <= watermark.emitted_at
        }
        (BoundaryKind::FullState { .. }, BoundaryKind::FullState { .. }) => {
            // Covered if the boundary was emitted at or before the watermark
            boundary.emitted_at <= watermark.emitted_at
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

    fn cursor_boundary_at(value: &str, emitted_at: chrono::DateTime<Utc>) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at,
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

    fn fullstate_boundary_at(
        value: &str,
        emitted_at: chrono::DateTime<Utc>,
    ) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::FullState {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at,
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
    fn test_advance_cursor_forward_accepted() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", cursor_boundary_at("abc", now))
            .unwrap();
        // Later emitted_at → forward movement → accepted
        assert!(ledger
            .advance(
                "src",
                cursor_boundary_at("def", now + Duration::seconds(10))
            )
            .is_ok());
    }

    #[test]
    fn test_advance_cursor_backward_rejected() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", cursor_boundary_at("abc", now))
            .unwrap();
        // Earlier emitted_at → backward movement → rejected
        let result = ledger.advance(
            "src",
            cursor_boundary_at("def", now - Duration::seconds(10)),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_fullstate_forward_accepted() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", fullstate_boundary_at("v1", now))
            .unwrap();
        assert!(ledger
            .advance(
                "src",
                fullstate_boundary_at("v2", now + Duration::seconds(1))
            )
            .is_ok());
    }

    #[test]
    fn test_advance_fullstate_backward_rejected() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", fullstate_boundary_at("v2", now))
            .unwrap();
        let result = ledger.advance(
            "src",
            fullstate_boundary_at("v1", now - Duration::seconds(10)),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_fullstate_same_emitted_at_accepted() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", fullstate_boundary_at("v1", now))
            .unwrap();
        // Same emitted_at = idempotent re-assertion → accepted
        assert!(ledger
            .advance("src", fullstate_boundary_at("v1", now))
            .is_ok());
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
    fn test_covers_cursor_by_timestamp() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        // Watermark emitted at T=now
        ledger
            .advance("src", cursor_boundary_at("wm", now))
            .unwrap();
        // Boundary emitted before watermark → covered
        assert!(ledger.covers(
            "src",
            &cursor_boundary_at("old", now - Duration::seconds(10))
        ));
        // Boundary emitted at same time → covered
        assert!(ledger.covers("src", &cursor_boundary_at("same", now)));
        // Boundary emitted after watermark → not covered
        assert!(!ledger.covers(
            "src",
            &cursor_boundary_at("new", now + Duration::seconds(10))
        ));
    }

    #[test]
    fn test_covers_fullstate_by_timestamp() {
        let now = Utc::now();
        let mut ledger = BoundaryLedger::new();
        ledger
            .advance("src", fullstate_boundary_at("v3", now))
            .unwrap();
        // Boundary emitted before watermark → covered (late arrival)
        assert!(ledger.covers(
            "src",
            &fullstate_boundary_at("v2", now - Duration::seconds(5))
        ));
        // Boundary emitted after watermark → not covered
        assert!(!ledger.covers(
            "src",
            &fullstate_boundary_at("v4", now + Duration::seconds(5))
        ));
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
