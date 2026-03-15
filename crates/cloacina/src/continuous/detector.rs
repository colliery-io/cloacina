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

//! Detector output types for continuous scheduling.
//!
//! Detector workflows are regular Cloacina workflows that produce
//! `DetectorOutput` in their output context. This module defines the
//! output enum and context key conventions.
//!
//! See CLOACI-S-0004 for the full specification.

use super::boundary::ComputationBoundary;
use serde::{Deserialize, Serialize};

/// Well-known context key for detector output.
pub const DETECTOR_OUTPUT_KEY: &str = "__detector_output";

/// Output produced by a detector workflow.
///
/// Detector workflows write this to their output context under the
/// `__detector_output` key. The `ContinuousScheduler` extracts it
/// after the detector workflow completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DetectorOutput {
    /// Data has changed — new boundaries to process.
    Change {
        boundaries: Vec<ComputationBoundary>,
    },
    /// Source watermark has advanced (no new data, but progress made).
    WatermarkAdvance { boundary: ComputationBoundary },
    /// Both data change and watermark advance.
    Both {
        boundaries: Vec<ComputationBoundary>,
        watermark: ComputationBoundary,
    },
}

impl DetectorOutput {
    /// Extract `DetectorOutput` from a task output context.
    ///
    /// Returns `None` if the key is missing or the value can't be deserialized.
    pub fn from_context(context: &cloacina_workflow::Context<serde_json::Value>) -> Option<Self> {
        let value = context.get(DETECTOR_OUTPUT_KEY)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Get all change boundaries from this output (empty for WatermarkAdvance-only).
    pub fn boundaries(&self) -> &[ComputationBoundary] {
        match self {
            DetectorOutput::Change { boundaries } => boundaries,
            DetectorOutput::Both { boundaries, .. } => boundaries,
            DetectorOutput::WatermarkAdvance { .. } => &[],
        }
    }

    /// Get the watermark boundary if present.
    pub fn watermark(&self) -> Option<&ComputationBoundary> {
        match self {
            DetectorOutput::WatermarkAdvance { boundary } => Some(boundary),
            DetectorOutput::Both { watermark, .. } => Some(watermark),
            DetectorOutput::Change { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::boundary::BoundaryKind;
    use chrono::Utc;

    fn make_boundary() -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::OffsetRange { start: 0, end: 100 },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    #[test]
    fn test_detector_output_change_serialization() {
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary()],
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["type"], "Change");
        assert_eq!(json["boundaries"].as_array().unwrap().len(), 1);

        let deserialized: DetectorOutput = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.boundaries().len(), 1);
    }

    #[test]
    fn test_detector_output_watermark_advance() {
        let output = DetectorOutput::WatermarkAdvance {
            boundary: make_boundary(),
        };
        assert!(output.boundaries().is_empty());
        assert!(output.watermark().is_some());
    }

    #[test]
    fn test_detector_output_both() {
        let output = DetectorOutput::Both {
            boundaries: vec![make_boundary(), make_boundary()],
            watermark: make_boundary(),
        };
        assert_eq!(output.boundaries().len(), 2);
        assert!(output.watermark().is_some());
    }

    #[test]
    fn test_detector_output_from_context() {
        let mut ctx = cloacina_workflow::Context::new();
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary()],
        };
        let value = serde_json::to_value(&output).unwrap();
        ctx.insert(DETECTOR_OUTPUT_KEY, value).unwrap();

        let extracted = DetectorOutput::from_context(&ctx).unwrap();
        assert_eq!(extracted.boundaries().len(), 1);
    }

    #[test]
    fn test_detector_output_from_context_missing() {
        let ctx = cloacina_workflow::Context::new();
        assert!(DetectorOutput::from_context(&ctx).is_none());
    }
}
