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

//! Injectable input interface — runtime JSON Schema generation.
//!
//! The descriptor format for a declared input slot is JSON Schema
//! (CLOACI-A-0007). This module is the single place a Rust type is turned into a
//! JSON Schema `serde_json::Value`, used to populate
//! [`cloacina_api_types::InputSlot`] for the injectable surfaces:
//! workflow params, accumulator boundaries, and reactor sources (I-0128).
//!
//! The wire/API type ([`cloacina_api_types::InputSlot`]) carries the schema as
//! an opaque `serde_json::Value`; generation lives here so callers only need a
//! `T: schemars::JsonSchema` bound.

pub use cloacina_api_types::InputSlot;

/// Generate a JSON Schema for `T` as a `serde_json::Value`.
///
/// Returns `Value::Null` only if serialization of the generated schema fails,
/// which is not expected for a well-formed `JsonSchema` impl. Callers building
/// an [`InputSlot`] use the result directly as `InputSlot::schema`.
pub fn schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    let root = schemars::gen::SchemaGenerator::default().into_root_schema_for::<T>();
    serde_json::to_value(root).unwrap_or(serde_json::Value::Null)
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
        // schemars emits object schemas with a `properties` map.
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("struct schema has properties");
        assert!(props.contains_key("order_id"));
        assert!(props.contains_key("limit"));
        assert!(props.contains_key("enabled"));
    }

    #[test]
    fn schema_round_trips_into_input_slot() {
        let slot = InputSlot::required("order_id", schema_for::<String>());
        let json = serde_json::to_string(&slot).expect("serialize");
        let back: InputSlot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "order_id");
        assert!(back.required);
        assert!(back.default.is_none());
    }
}
