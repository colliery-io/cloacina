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

//! Computation boundary types for continuous scheduling.
//!
//! A `ComputationBoundary` describes what slice of data a signal or execution
//! covers. It is advisory data — the framework carries it between components,
//! coalesces it when signals pile up, and puts it in context for tasks to read.
//!
//! See CLOACI-S-0002 for the full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// A serializable message describing what slice of data a signal or execution covers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationBoundary {
    /// The kind of boundary and its data.
    pub kind: BoundaryKind,
    /// Domain-specific context that isn't about merging.
    pub metadata: Option<serde_json::Value>,
    /// When the detector created this boundary.
    pub emitted_at: DateTime<Utc>,
}

/// The specific type and data of a computation boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BoundaryKind {
    /// Classic Airflow-style intervals — coalesces via min(start), max(end).
    TimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    /// Kafka-style partition offsets — coalesces via min(start), max(end).
    OffsetRange { start: i64, end: i64 },
    /// Opaque "resume from here" token — coalesces via latest-wins.
    Cursor { value: String },
    /// Entire dataset is the unit of change — coalesces via latest-wins.
    /// Value is a user-provided state identifier (hash, version counter, commit SHA, etc.).
    FullState { value: String },
    /// User-defined boundary type with schema-validated payload.
    Custom {
        kind: String,
        value: serde_json::Value,
    },
}

/// A boundary buffered in an accumulator, with receipt timestamp for backpressure measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedBoundary {
    /// The original boundary.
    pub boundary: ComputationBoundary,
    /// When the accumulator received this boundary.
    pub received_at: DateTime<Utc>,
}

impl BufferedBoundary {
    /// Create a new buffered boundary with the current time as receipt time.
    pub fn new(boundary: ComputationBoundary) -> Self {
        Self {
            boundary,
            received_at: Utc::now(),
        }
    }

    /// Calculate ingestion lag (received_at - emitted_at).
    pub fn lag(&self) -> chrono::Duration {
        self.received_at
            .signed_duration_since(self.boundary.emitted_at)
    }
}

/// Schema definition for custom boundary types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomBoundarySchema {
    /// Unique type name (must match `Custom { kind }`).
    pub kind: String,
    /// JSON Schema defining the expected shape of `value`.
    pub schema: serde_json::Value,
}

/// Global registry for custom boundary schemas.
static CUSTOM_SCHEMAS: std::sync::LazyLock<RwLock<HashMap<String, CustomBoundarySchema>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Register a custom boundary schema.
///
/// Custom boundary types must be registered before they can be used.
/// Unregistered custom boundary kinds are rejected at signal ingestion.
pub fn register_custom_boundary(kind: &str, schema: serde_json::Value) {
    let mut schemas = CUSTOM_SCHEMAS.write().unwrap();
    schemas.insert(
        kind.to_string(),
        CustomBoundarySchema {
            kind: kind.to_string(),
            schema,
        },
    );
}

/// Validate a custom boundary payload against its registered schema.
///
/// Returns `Ok(())` if the payload is valid, or an error message if not.
pub fn validate_custom_boundary(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    let schemas = CUSTOM_SCHEMAS.read().unwrap();
    let schema = schemas
        .get(kind)
        .ok_or_else(|| format!("unregistered custom boundary kind: '{}'", kind))?;

    validate_against_schema(value, &schema.schema)
}

/// Clear all registered custom boundary schemas (for testing).
#[cfg(test)]
pub fn clear_custom_schemas() {
    let mut schemas = CUSTOM_SCHEMAS.write().unwrap();
    schemas.clear();
}

/// Simple JSON schema validation.
///
/// Validates `required` fields and basic `type` checks. Not a full JSON Schema
/// implementation — covers the common cases needed for boundary validation.
fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), String> {
    // Check type constraint
    if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
        let actual_type = match value {
            serde_json::Value::Object(_) => "object",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Number(_) => {
                if value.as_i64().is_some() || value.as_u64().is_some() {
                    "integer"
                } else {
                    "number"
                }
            }
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Null => "null",
        };

        // "number" matches both "number" and "integer"
        let type_matches =
            actual_type == expected_type || (expected_type == "number" && actual_type == "integer");

        if !type_matches {
            return Err(format!(
                "expected type '{}', got '{}'",
                expected_type, actual_type
            ));
        }
    }

    // Check required fields for objects
    if let (Some(required), Some(obj)) = (
        schema.get("required").and_then(|r| r.as_array()),
        value.as_object(),
    ) {
        for field in required {
            if let Some(field_name) = field.as_str() {
                if !obj.contains_key(field_name) {
                    return Err(format!("missing required field: '{}'", field_name));
                }
            }
        }
    }

    // Check properties types for objects
    if let (Some(properties), Some(obj)) = (
        schema.get("properties").and_then(|p| p.as_object()),
        value.as_object(),
    ) {
        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = obj.get(prop_name) {
                validate_against_schema(prop_value, prop_schema)?;
            }
        }
    }

    Ok(())
}

/// Coalesce a slice of computation boundaries into a single boundary.
///
/// Coalescing rules per variant:
/// - `TimeRange`: `min(starts)..max(ends)`
/// - `OffsetRange`: `min(starts)..max(ends)`
/// - `Cursor`: Latest wins (by emitted_at)
/// - `FullState`: Latest wins (by emitted_at)
/// - `Custom`: Not coalesced — returns the latest boundary unchanged
///
/// Returns `None` if the slice is empty. Returns the single boundary if length is 1.
/// All boundaries must be of the same variant; mixed variants return the latest.
pub fn coalesce(boundaries: &[ComputationBoundary]) -> Option<ComputationBoundary> {
    if boundaries.is_empty() {
        return None;
    }
    if boundaries.len() == 1 {
        return Some(boundaries[0].clone());
    }

    // Find the latest emitted_at for metadata
    let latest = boundaries.iter().max_by_key(|b| b.emitted_at).unwrap();

    let coalesced_kind = match &boundaries[0].kind {
        BoundaryKind::TimeRange { .. } => {
            let mut min_start = DateTime::<Utc>::MAX_UTC;
            let mut max_end = DateTime::<Utc>::MIN_UTC;
            for b in boundaries {
                if let BoundaryKind::TimeRange { start, end } = &b.kind {
                    if *start < min_start {
                        min_start = *start;
                    }
                    if *end > max_end {
                        max_end = *end;
                    }
                } else {
                    // Mixed variants — return latest
                    return Some(latest.clone());
                }
            }
            BoundaryKind::TimeRange {
                start: min_start,
                end: max_end,
            }
        }
        BoundaryKind::OffsetRange { .. } => {
            let mut min_start = i64::MAX;
            let mut max_end = i64::MIN;
            for b in boundaries {
                if let BoundaryKind::OffsetRange { start, end } = &b.kind {
                    if *start < min_start {
                        min_start = *start;
                    }
                    if *end > max_end {
                        max_end = *end;
                    }
                } else {
                    return Some(latest.clone());
                }
            }
            BoundaryKind::OffsetRange {
                start: min_start,
                end: max_end,
            }
        }
        BoundaryKind::Cursor { .. }
        | BoundaryKind::FullState { .. }
        | BoundaryKind::Custom { .. } => {
            // Latest-wins for Cursor, FullState, and Custom
            return Some(latest.clone());
        }
    };

    Some(ComputationBoundary {
        kind: coalesced_kind,
        metadata: latest.metadata.clone(),
        emitted_at: latest.emitted_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_time_boundary(start_offset_hours: i64, end_offset_hours: i64) -> ComputationBoundary {
        let now = Utc::now();
        ComputationBoundary {
            kind: BoundaryKind::TimeRange {
                start: now + Duration::hours(start_offset_hours),
                end: now + Duration::hours(end_offset_hours),
            },
            metadata: None,
            emitted_at: now,
        }
    }

    fn make_offset_boundary(start: i64, end: i64) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::OffsetRange { start, end },
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    fn make_cursor_boundary(value: &str, emitted_at: DateTime<Utc>) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at,
        }
    }

    fn make_fullstate_boundary(value: &str, emitted_at: DateTime<Utc>) -> ComputationBoundary {
        ComputationBoundary {
            kind: BoundaryKind::FullState {
                value: value.to_string(),
            },
            metadata: None,
            emitted_at,
        }
    }

    // --- Coalescing tests ---

    #[test]
    fn test_coalesce_empty() {
        assert!(coalesce(&[]).is_none());
    }

    #[test]
    fn test_coalesce_single() {
        let b = make_offset_boundary(0, 100);
        let result = coalesce(&[b.clone()]).unwrap();
        assert_eq!(result.kind, b.kind);
    }

    #[test]
    fn test_coalesce_time_ranges() {
        let b1 = make_time_boundary(0, 1);
        let b2 = make_time_boundary(1, 2);
        let b3 = make_time_boundary(2, 3);

        let result = coalesce(&[b1.clone(), b2, b3]).unwrap();
        if let BoundaryKind::TimeRange { start, end } = &result.kind {
            if let BoundaryKind::TimeRange {
                start: s1,
                end: _e1,
            } = &b1.kind
            {
                assert_eq!(start, s1); // min start
            }
            // end should be ~3 hours from now
            assert!(*end > *start);
        } else {
            panic!("expected TimeRange");
        }
    }

    #[test]
    fn test_coalesce_offset_ranges() {
        let b1 = make_offset_boundary(0, 100);
        let b2 = make_offset_boundary(100, 200);
        let b3 = make_offset_boundary(50, 150);

        let result = coalesce(&[b1, b2, b3]).unwrap();
        assert_eq!(
            result.kind,
            BoundaryKind::OffsetRange { start: 0, end: 200 }
        );
    }

    #[test]
    fn test_coalesce_cursors_latest_wins() {
        let now = Utc::now();
        let b1 = make_cursor_boundary("cursor_a", now - Duration::seconds(10));
        let b2 = make_cursor_boundary("cursor_b", now);
        let b3 = make_cursor_boundary("cursor_c", now - Duration::seconds(5));

        let result = coalesce(&[b1, b2, b3]).unwrap();
        if let BoundaryKind::Cursor { value } = &result.kind {
            assert_eq!(value, "cursor_b"); // latest emitted_at
        } else {
            panic!("expected Cursor");
        }
    }

    #[test]
    fn test_coalesce_fullstate_latest_wins() {
        let now = Utc::now();
        let b1 = make_fullstate_boundary("v7", now - Duration::seconds(10));
        let b2 = make_fullstate_boundary("v8", now);

        let result = coalesce(&[b1, b2]).unwrap();
        if let BoundaryKind::FullState { value } = &result.kind {
            assert_eq!(value, "v8");
        } else {
            panic!("expected FullState");
        }
    }

    // --- BufferedBoundary tests ---

    #[test]
    fn test_buffered_boundary_lag() {
        let boundary = ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: "test".into(),
            },
            metadata: None,
            emitted_at: Utc::now() - Duration::milliseconds(500),
        };
        let buffered = BufferedBoundary::new(boundary);
        assert!(buffered.lag().num_milliseconds() >= 400);
    }

    // --- Serialization tests ---

    #[test]
    fn test_boundary_serialization_roundtrip() {
        let boundary = ComputationBoundary {
            kind: BoundaryKind::OffsetRange { start: 0, end: 100 },
            metadata: Some(serde_json::json!({"row_count": 42})),
            emitted_at: Utc::now(),
        };
        let json = serde_json::to_string(&boundary).unwrap();
        let deserialized: ComputationBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(boundary.kind, deserialized.kind);
    }

    #[test]
    fn test_boundary_kind_tagged_serialization() {
        let kind = BoundaryKind::TimeRange {
            start: Utc::now(),
            end: Utc::now(),
        };
        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(json["type"], "TimeRange");
    }

    // --- Custom schema tests ---

    #[test]
    fn test_custom_schema_validation_passes() {
        register_custom_boundary(
            "sequence_range_test",
            serde_json::json!({
                "type": "object",
                "required": ["table", "min_id", "max_id"],
                "properties": {
                    "table": { "type": "string" },
                    "min_id": { "type": "integer" },
                    "max_id": { "type": "integer" }
                }
            }),
        );

        let result = validate_custom_boundary(
            "sequence_range_test",
            &serde_json::json!({
                "table": "events",
                "min_id": 1,
                "max_id": 100
            }),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_schema_missing_required_field() {
        register_custom_boundary(
            "missing_field_test_type",
            serde_json::json!({
                "type": "object",
                "required": ["name"],
            }),
        );

        let result = validate_custom_boundary("missing_field_test_type", &serde_json::json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required field"));
    }

    #[test]
    fn test_custom_schema_unregistered_kind() {
        // Use a name that is guaranteed not to be registered by any other test
        let result = validate_custom_boundary(
            "absolutely_never_registered_xyz_123",
            &serde_json::json!({}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unregistered"));
    }

    #[test]
    fn test_custom_schema_wrong_type() {
        // Use unique name to avoid global state race with other tests
        register_custom_boundary(
            "wrong_type_test_schema",
            serde_json::json!({
                "type": "object",
            }),
        );

        let result = validate_custom_boundary(
            "wrong_type_test_schema",
            &serde_json::json!("not an object"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected type"));
    }
}
