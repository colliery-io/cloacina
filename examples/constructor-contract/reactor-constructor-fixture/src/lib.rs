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

//! CLOACI-T-0828 — a REACTOR constructor authored with the `#[constructor]` macro.
//!
//! The author writes ONLY:
//!   * the constructor struct, with `#[config]` fields bound once per instance
//!     at load, and
//!   * the `evaluate` body: given the held boundaries (a JSON object keyed by
//!     source name), decide whether to fire (`Ok(Some(context_json))`) or hold
//!     (`Ok(None)`).
//!
//! `#[constructor(kind = reactor, ...)]` generates the fidius `ReactorConstructor`
//! trait + impl + `configure` + the JSON wire, plus a
//! `pub fn __constructor_manifest() -> ConstructorManifest`. The generated guest glue
//! is `#[cfg(target_arch = "wasm32")]`, so this crate also builds on the host
//! (the `emit_manifest` bin reads `__constructor_manifest()`).

#![allow(dead_code)]
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Fires when ANY held boundary's numeric value crosses the configured `gate`.
/// Each boundary value is read as a bare number or as an object's `value` field.
#[constructor(
    kind = reactor,
    name = "gate",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Fires the graph when any held boundary value crosses a configured gate.",
    author = "CLOACI-T-0828"
)]
pub struct Gate {
    /// Bound once per instance at load via the generated `configure` hook.
    #[config]
    gate: f64,
}

impl Gate {
    /// The ONLY thing the author writes: scan the held boundaries and fire when
    /// any value crosses the config-bound `gate`.
    fn evaluate(&self, boundaries_json: &str) -> Result<Option<String>, ConstructorError> {
        let boundaries: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(boundaries_json)
                .map_err(|e| ConstructorError::msg(format!("decode boundaries: {e}")))?;

        let fire = boundaries.values().any(|v| {
            let n = v
                .get("value")
                .and_then(|x| x.as_f64())
                .or_else(|| v.as_f64());
            n.map(|n| n >= self.gate).unwrap_or(false)
        });

        if fire {
            Ok(Some(serde_json::json!({ "fired": true }).to_string()))
        } else {
            Ok(None)
        }
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(
    name = "gate",
    version = "0.1.0",
    contract = constructor_contract,
    reactor = [Gate],
);
