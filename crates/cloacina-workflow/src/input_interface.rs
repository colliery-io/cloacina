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

//! Injectable input interface — JSON Schema generation (CLOACI-I-0128).
//!
//! Canonical home for turning a Rust type into a JSON Schema descriptor and for
//! the [`InputSlot`] contract. Lives in `cloacina-workflow` (not core `cloacina`)
//! because the `#[workflow(params(...))]` macro emits calls to these helpers into
//! **packaged cdylibs**, which depend on `cloacina-workflow`, not core. Core
//! `cloacina` re-exports this module.
//!
//! Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007] (JSON Schema via
//! `schemars`).

pub use cloacina_api_types::InputSlot;

/// Generate a JSON Schema for `T` as a `serde_json::Value`.
///
/// The macro-generated `#[workflow(params(...))]` descriptor calls this per
/// declared param type; accumulator/reactor boundary derivation (Task D) calls
/// it per boundary type. Returns `Value::Null` only if the generated schema
/// fails to serialize (not expected for a well-formed `JsonSchema` impl).
pub fn schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    let root = schemars::gen::SchemaGenerator::default().into_root_schema_for::<T>();
    serde_json::to_value(root).unwrap_or(serde_json::Value::Null)
}

/// Serialize a default value to `serde_json::Value` for an [`InputSlot::default`].
/// Returns `None` if the value can't serialize.
pub fn default_json<T: serde::Serialize>(value: T) -> Option<serde_json::Value> {
    serde_json::to_value(value).ok()
}

/// Opt-in schema derivation for accumulator/reactor boundary types (CLOACI-I-0128
/// Task D).
///
/// Unlike workflow params (whose types we control via `#[workflow(params(...))]`),
/// computation-graph **boundary types** are defined in the author's crate and may
/// or may not derive [`schemars::JsonSchema`]. We do NOT want to force the derive
/// — so the `#[computation_graph]` macro can't unconditionally call
/// [`schema_for`] (that needs the bound at compile time).
///
/// Instead the macro emits a probe over [`SchemaProbe<T>`] that, via **autoref
/// specialization** (the stable-Rust dtolnay pattern), resolves to the real
/// schema when `T: JsonSchema` and to a permissive `{}` ("any") schema otherwise.
/// Authors opt a boundary type into rich typing simply by adding
/// `#[derive(schemars::JsonSchema)]`; types without it degrade to name-only slots
/// rather than failing to compile.
///
/// ## Usage (what the macro emits)
/// ```ignore
/// {
///     use ::cloacina_workflow::input_interface::{ProbeTyped as _, ProbeFallback as _};
///     (&::cloacina_workflow::input_interface::SchemaProbe::<MyType>::new())
///         .probe_input_schema()
/// }
/// ```
pub struct SchemaProbe<T>(core::marker::PhantomData<T>);

impl<T> SchemaProbe<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SchemaProbe(core::marker::PhantomData)
    }
}

/// Specific arm of the probe: selected when `T: JsonSchema` (zero autorefs on a
/// `&SchemaProbe<T>` receiver), yielding the real derived schema.
pub trait ProbeTyped {
    fn probe_input_schema(&self) -> serde_json::Value;
}
impl<T: schemars::JsonSchema> ProbeTyped for SchemaProbe<T> {
    fn probe_input_schema(&self) -> serde_json::Value {
        schema_for::<T>()
    }
}

/// Fallback arm of the probe: selected for any `T` (one autoref on a
/// `&SchemaProbe<T>` receiver), yielding a permissive "any" schema. Method
/// resolution prefers [`ProbeTyped`] whenever its `T: JsonSchema` bound holds.
pub trait ProbeFallback {
    fn probe_input_schema(&self) -> serde_json::Value;
}
impl<T> ProbeFallback for &SchemaProbe<T> {
    fn probe_input_schema(&self) -> serde_json::Value {
        // Empty object = "accept anything"; the slot is name-only (untyped).
        serde_json::json!({})
    }
}

/// Serialize a slot list to the JSON array string carried across the FFI
/// descriptor entrypoint (`InputInterfaceEntry::slots_json`). Falls back to an
/// empty array on serialization failure.
pub fn slots_to_json(slots: &[InputSlot]) -> String {
    serde_json::to_string(slots).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(JsonSchema, Serialize, Deserialize)]
    #[allow(dead_code)]
    struct SampleBoundary {
        order_id: String,
        limit: u32,
        enabled: bool,
    }

    #[test]
    fn schema_for_scalar_type() {
        let schema = schema_for::<String>();
        assert_eq!(
            schema.get("type").and_then(|t| t.as_str()),
            Some("string"),
            "String should produce a string-typed JSON Schema, got: {schema}"
        );
    }

    #[test]
    fn schema_for_struct_has_properties() {
        let schema = schema_for::<SampleBoundary>();
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("struct schema has properties");
        assert!(props.contains_key("order_id"));
        assert!(props.contains_key("limit"));
        assert!(props.contains_key("enabled"));
    }

    // A type that deliberately does NOT derive JsonSchema (only serde).
    #[derive(Serialize, Deserialize)]
    #[allow(dead_code)]
    struct UntypedBoundary {
        raw: Vec<u8>,
    }

    #[test]
    fn probe_typed_boundary_yields_real_schema() {
        // Only the typed arm is exercised here (SampleBoundary: JsonSchema), so
        // just `ProbeTyped` is imported — importing `ProbeFallback` too would be
        // an unused import.
        use super::ProbeTyped as _;
        // SampleBoundary derives JsonSchema → the typed arm wins.
        let schema = (&SchemaProbe::<SampleBoundary>::new()).probe_input_schema();
        assert!(
            schema.get("properties").is_some(),
            "JsonSchema boundary should yield a real object schema, got: {schema}"
        );
    }

    #[test]
    fn probe_untyped_boundary_falls_back_to_any() {
        // UntypedBoundary does NOT derive JsonSchema, so only the fallback arm
        // applies; importing `ProbeTyped` too would be an unused import.
        use super::ProbeFallback as _;
        // UntypedBoundary does NOT derive JsonSchema → fallback arm → permissive {}.
        let schema = (&SchemaProbe::<UntypedBoundary>::new()).probe_input_schema();
        assert_eq!(
            schema,
            serde_json::json!({}),
            "non-JsonSchema boundary should fall back to a permissive schema, got: {schema}"
        );
    }

    #[test]
    fn probe_scalar_typed_yields_schema() {
        // `String: JsonSchema`, so the typed arm wins; `ProbeFallback` would be
        // an unused import here.
        use super::ProbeTyped as _;
        let schema = (&SchemaProbe::<String>::new()).probe_input_schema();
        assert_eq!(schema.get("type").and_then(|t| t.as_str()), Some("string"));
    }

    #[test]
    fn slots_round_trip_through_json() {
        let slots = vec![
            InputSlot::required("order_id", schema_for::<String>()),
            InputSlot::optional("limit", schema_for::<u32>(), default_json(100u32)),
        ];
        let json = slots_to_json(&slots);
        let back: Vec<InputSlot> = serde_json::from_str(&json).expect("parse slots_json");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].name, "order_id");
        assert!(back[0].required);
        // Plaintext params carry the `encrypted: false` marker.
        assert!(!back[0].encrypted);
        assert_eq!(back[1].name, "limit");
        assert!(!back[1].required);
        assert_eq!(back[1].default, Some(serde_json::json!(100)));
    }

    // CLOACI-T-0859: an encrypted secret slot round-trips through the same
    // slots-JSON the FFI/manifest metadata carries, with the marker preserved.
    #[test]
    fn secret_slot_round_trips_with_encrypted_marker() {
        let slots = vec![
            InputSlot::required("order_id", schema_for::<String>()),
            InputSlot::secret("db_prod"),
        ];
        let json = slots_to_json(&slots);
        // The marker is visible in the serialized manifest form.
        assert!(json.contains("\"encrypted\":true"), "json: {json}");

        let back: Vec<InputSlot> = serde_json::from_str(&json).expect("parse slots_json");
        let secret = back.iter().find(|s| s.name == "db_prod").unwrap();
        assert!(secret.encrypted);
        assert!(secret.required);
        assert!(secret.default.is_none());
        // A plaintext param stays unmarked.
        assert!(
            !back
                .iter()
                .find(|s| s.name == "order_id")
                .unwrap()
                .encrypted
        );
    }

    // A pre-existing serialized slot (no `encrypted` field) still deserializes,
    // defaulting to a plaintext param.
    #[test]
    fn legacy_slot_json_without_encrypted_field_defaults_to_plaintext() {
        let legacy = r#"[{"name":"x","schema":{"type":"string"},"required":true}]"#;
        let back: Vec<InputSlot> = serde_json::from_str(legacy).expect("parse legacy slots");
        assert_eq!(back.len(), 1);
        assert!(!back[0].encrypted);
    }
}
