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

//! Boundary emission utilities for testing continuous tasks.
//!
//! This module provides [`BoundaryEmitter`] for simulating detector output
//! in tests. Available when the `continuous` feature is enabled.
//!
//! **Note**: The continuous scheduling types (`ComputationBoundary`, `DataConnection`)
//! are not yet available in `cloacina`. This module uses local placeholder types
//! designed against the specs (CLOACI-S-0001, CLOACI-S-0002). Once CLOACI-I-0023
//! lands, these will be replaced with the real types.

use chrono::{DateTime, Utc};
use cloacina_workflow::Context;
use serde_json::json;

/// A computation boundary representing a slice of data to process.
///
/// Placeholder type — will be replaced by `cloacina::ComputationBoundary`
/// once CLOACI-I-0023 lands.
#[derive(Debug, Clone)]
pub enum ComputationBoundary {
    /// A time-based boundary.
    TimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    /// An offset-based boundary.
    OffsetRange { start: i64, end: i64 },
}

/// Simulates detector output for testing continuous tasks.
///
/// Use the builder pattern to emit boundaries, then convert to a context
/// that matches the format an accumulator's `drain()` would produce.
///
/// # Example
///
/// ```rust,ignore
/// use cloacina_testing::BoundaryEmitter;
/// use chrono::Utc;
///
/// let ctx = BoundaryEmitter::new()
///     .emit_time_range(Utc::now() - chrono::Duration::hours(1), Utc::now())
///     .into_context();
/// ```
pub struct BoundaryEmitter {
    boundaries: Vec<ComputationBoundary>,
}

impl BoundaryEmitter {
    /// Create a new empty emitter.
    pub fn new() -> Self {
        Self {
            boundaries: Vec::new(),
        }
    }

    /// Emit a raw boundary.
    pub fn emit(mut self, boundary: ComputationBoundary) -> Self {
        self.boundaries.push(boundary);
        self
    }

    /// Emit a time-range boundary.
    pub fn emit_time_range(self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.emit(ComputationBoundary::TimeRange { start, end })
    }

    /// Emit an offset-range boundary.
    pub fn emit_offset_range(self, start: i64, end: i64) -> Self {
        self.emit(ComputationBoundary::OffsetRange { start, end })
    }

    /// Convert emitted boundaries into a context matching accumulator drain output.
    pub fn into_context(self) -> Context<serde_json::Value> {
        let mut ctx = Context::new();
        let boundaries: Vec<serde_json::Value> = self
            .boundaries
            .into_iter()
            .map(|b| match b {
                ComputationBoundary::TimeRange { start, end } => json!({
                    "type": "time_range",
                    "start": start.to_rfc3339(),
                    "end": end.to_rfc3339(),
                }),
                ComputationBoundary::OffsetRange { start, end } => json!({
                    "type": "offset_range",
                    "start": start,
                    "end": end,
                }),
            })
            .collect();
        let _ = ctx.insert("__boundaries", json!(boundaries));
        ctx
    }
}

impl Default for BoundaryEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_emitter() {
        let ctx = BoundaryEmitter::new().into_context();
        let boundaries = ctx.get("__boundaries").unwrap();
        assert_eq!(boundaries, &json!([]));
    }

    #[test]
    fn test_time_range_boundary() {
        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now();
        let ctx = BoundaryEmitter::new()
            .emit_time_range(start, end)
            .into_context();

        let boundaries = ctx.get("__boundaries").unwrap();
        let arr = boundaries.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "time_range");
    }

    #[test]
    fn test_offset_range_boundary() {
        let ctx = BoundaryEmitter::new()
            .emit_offset_range(0, 100)
            .into_context();

        let boundaries = ctx.get("__boundaries").unwrap();
        let arr = boundaries.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "offset_range");
        assert_eq!(arr[0]["start"], 0);
        assert_eq!(arr[0]["end"], 100);
    }

    #[test]
    fn test_multiple_boundaries() {
        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now();
        let ctx = BoundaryEmitter::new()
            .emit_time_range(start, end)
            .emit_offset_range(0, 50)
            .emit_offset_range(50, 100)
            .into_context();

        let boundaries = ctx.get("__boundaries").unwrap();
        let arr = boundaries.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }
}
