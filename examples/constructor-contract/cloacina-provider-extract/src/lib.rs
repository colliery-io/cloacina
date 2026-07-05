/*
 *  Copyright 2026 Colliery Software
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

//! CLOACI-T-0825 — seed ACCUMULATOR provider: field projection.
//!
//! Projects a configured field out of each incoming event: an event carrying the
//! field emits a boundary `{ "<field>": <value> }`; an event without it buffers
//! (no boundary). The everyday "map/filter the event stream into boundaries"
//! building block — configure `field = "order_id"` and only order events pass
//! through, already narrowed to the value the graph cares about.
//!
//! NOTE the authoring model is per-call: the instance is rebuilt from its bound
//! config for every `ingest`, so seed accumulators are STATELESS transforms.
//! Windowing/counting across events needs runtime-held state — a follow-on.

#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (mirrors the loader's own allow).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Emits a boundary containing the configured `#[config] field` when the event
/// carries it; buffers events that don't.
#[constructor(
    kind = accumulator,
    name = "extract",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Projects a configured field from each event into the boundary; buffers events without it.",
    author = "CLOACI-T-0825"
)]
pub struct Extract {
    /// The event field to project into the boundary, bound once at load.
    #[config]
    field: String,
}

impl Extract {
    /// The ONLY thing the author writes: decode the event, and emit the
    /// projected field as the boundary when present.
    fn ingest(&self, event_json: &str) -> Result<Option<String>, ConstructorError> {
        let event: serde_json::Value = serde_json::from_str(event_json)
            .map_err(|e| ConstructorError::msg(format!("decode event: {e}")))?;
        match event.get(&self.field) {
            Some(value) => {
                let boundary = serde_json::json!({ self.field.clone(): value });
                Ok(Some(boundary.to_string()))
            }
            None => Ok(None),
        }
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(contract = constructor_contract, accumulator = [Extract],);
