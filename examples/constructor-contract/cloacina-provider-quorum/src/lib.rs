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

//! CLOACI-T-0825 — seed REACTOR provider: quorum firing criteria.
//!
//! Fires the graph when at least `required` accumulator boundaries are held —
//! the "wait for N of my inputs before running" criteria, configurable per
//! instance. `required = 1` is "fire on anything"; `required = <boundary
//! count>` is "wait for all".

#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (mirrors the loader's own allow).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Fires when at least `#[config] required` boundaries are held.
#[constructor(
    kind = reactor,
    name = "quorum",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Fires when at least `required` accumulator boundaries are held.",
    author = "CLOACI-T-0825"
)]
pub struct Quorum {
    /// How many held boundaries constitute a quorum, bound once at load.
    #[config]
    required: i64,
}

impl Quorum {
    /// The ONLY thing the author writes: count the held boundaries and fire
    /// (with the count in the fire payload) once the quorum is met.
    fn evaluate(&self, boundaries_json: &str) -> Result<Option<String>, ConstructorError> {
        let boundaries: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(boundaries_json)
                .map_err(|e| ConstructorError::msg(format!("decode boundaries: {e}")))?;

        let held = boundaries.len() as i64;
        if held >= self.required {
            Ok(Some(
                serde_json::json!({ "quorum": held, "required": self.required }).to_string(),
            ))
        } else {
            Ok(None)
        }
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(contract = constructor_contract, reactor = [Quorum],);
