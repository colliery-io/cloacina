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

//! Injectable input interface — the shared contract for declared inputs across
//! injectable surfaces (workflow execute context, accumulator ingest, reactor
//! fire). CLOACI-I-0128 / spec CLOACI-S-0013 / ADR CLOACI-A-0007.

use serde::{Deserialize, Serialize};

/// One declared input slot of an injectable surface: a named, typed value the
/// surface accepts. `schema` is a JSON Schema fragment (the type descriptor —
/// `schemars`-derived for Rust, type-hint-derived for Python) that the UI can
/// render a form from and the server can validate an injection against.
///
/// `required` slots must be supplied; `default` (when present) is applied when a
/// slot is omitted. A surface with no declared interface exposes an empty slot
/// list (the "undeclared" state) and accepts free-form input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InputSlot {
    /// Slot name — the context key (workflows) or source/event name
    /// (accumulators/reactors).
    pub name: String,

    /// JSON Schema fragment describing the accepted value's type.
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub schema: serde_json::Value,

    /// Whether this slot must be supplied for the injection to be accepted.
    pub required: bool,

    /// Optional default applied when the slot is omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Object, nullable))]
    pub default: Option<serde_json::Value>,
}

impl InputSlot {
    /// Construct a required slot with no default.
    pub fn required(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            schema,
            required: true,
            default: None,
        }
    }

    /// Construct an optional slot with an optional default.
    pub fn optional(
        name: impl Into<String>,
        schema: serde_json::Value,
        default: Option<serde_json::Value>,
    ) -> Self {
        Self {
            name: name.into(),
            schema,
            required: false,
            default,
        }
    }
}
