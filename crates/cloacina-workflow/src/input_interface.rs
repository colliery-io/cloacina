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
        assert_eq!(back[1].name, "limit");
        assert!(!back[1].required);
        assert_eq!(back[1].default, Some(serde_json::json!(100)));
    }
}
