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

    /// CLOACI-I-0133 / T-0859: marks this slot as an **encrypted secret** rather
    /// than a plaintext param. A secret slot is bound via a `{"$secret": name}`
    /// reference (never a literal value), resolved encrypted at fire time, and
    /// its resolved value never enters the durable `Context` (NFR-001). Defaults
    /// to `false` so pre-existing serialized slots (which omit the field)
    /// deserialize as ordinary params.
    #[serde(default)]
    pub encrypted: bool,
}

impl InputSlot {
    /// Construct a required (plaintext) slot with no default.
    pub fn required(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            schema,
            required: true,
            default: None,
            encrypted: false,
        }
    }

    /// Construct an optional (plaintext) slot with an optional default.
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
            encrypted: false,
        }
    }

    /// Construct a required **encrypted secret** slot (CLOACI-I-0133 / T-0859).
    ///
    /// A secret is an opaque `{field: value}` map declared by name; its schema is
    /// permissive (`{}`) because the fields are resolved at fire time, never
    /// validated as a plaintext param. It is `required` (a declared-but-unbound
    /// secret is a register-time error) and carries no default.
    pub fn secret(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: serde_json::json!({}),
            required: true,
            default: None,
            encrypted: true,
        }
    }
}

/// A declared injectable surface other than the workflow itself — a computation
/// graph, reactor, or accumulator (CLOACI-I-0128 Task D). Carries the surface's
/// declared input slots so the server can validate operator injections
/// (reactor fire / accumulator inject) and the UI can render typed forms.
///
/// Sourced from the package's `get_input_interface` FFI entrypoint at build
/// success and stored alongside the package metadata. An undeclared / untyped
/// surface has `slots` whose schemas are permissive (`{}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeclaredSurface {
    /// Surface kind: `"graph"`, `"reactor"`, or `"accumulator"`.
    pub kind: String,
    /// Surface name (graph name / reactor name / accumulator name).
    pub name: String,
    /// The surface's declared input slots.
    pub slots: Vec<InputSlot>,
}
