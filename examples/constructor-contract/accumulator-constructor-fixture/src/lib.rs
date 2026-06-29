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

//! CLOACI-T-0828 — an ACCUMULATOR constructor authored with the `#[constructor]` macro.
//!
//! The author writes ONLY:
//!   * the constructor struct, with `#[config]` fields bound once per instance
//!     at load, and
//!   * the `ingest` body: take one event's JSON, optionally return a boundary
//!     JSON string for the reactor (`Ok(Some(..))` emits, `Ok(None)` buffers).
//!
//! `#[constructor(kind = accumulator, ...)]` generates the fidius
//! `AccumulatorConstructor` trait + impl + `configure` + the JSON wire, plus a
//! `pub fn __constructor_manifest() -> ConstructorManifest`. The generated guest glue
//! is `#[cfg(target_arch = "wasm32")]`, so this crate also builds on the host
//! (the `emit_manifest` bin reads `__constructor_manifest()` to produce
//! `constructor.json`).

// On the host build only the struct + manifest fn are reachable (the wasm guest
// glue that calls `ingest` / reads the fields is cfg'd out).
#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use cloacina_macros::constructor;
use constructor_contract::ConstructorError;

/// Emits a boundary only when an event's numeric `value` crosses the configured
/// `threshold`; otherwise buffers (no boundary this event).
#[constructor(
    kind = accumulator,
    name = "threshold",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Emits a boundary when an event value crosses a configured threshold.",
    author = "CLOACI-T-0828"
)]
pub struct Threshold {
    /// Bound once per instance at load via the generated `configure` hook.
    #[config]
    threshold: f64,
}

impl Threshold {
    /// The ONLY thing the author writes: parse the event, and emit a boundary
    /// when `value >= threshold` (config-bound), else buffer.
    fn ingest(&self, event_json: &str) -> Result<Option<String>, ConstructorError> {
        let event: serde_json::Value = serde_json::from_str(event_json)
            .map_err(|e| ConstructorError::msg(format!("decode event: {e}")))?;
        let value = event
            .get("value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ConstructorError::msg("event missing numeric `value`"))?;

        if value >= self.threshold {
            let boundary = serde_json::json!({ "crossed": value });
            Ok(Some(boundary.to_string()))
        } else {
            Ok(None)
        }
    }
}
