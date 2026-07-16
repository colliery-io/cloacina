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

//! CLOACI-I-0139 / T-0902 — NATIVE task-provider fixture.
//!
//! Identical author surface to `task-constructor-macro-fixture` (the clean
//! `#[constructor]` struct + one `execute` body + `constructor_provider!`
//! suite shell) — but this crate builds to a HOST cdylib, so
//! `constructor_provider!` emits its native shell (`#[cfg(not(wasm32))]`,
//! `crate = fidius_core`, plugin `__ProviderTask`) and the cloacina loader
//! `dlopen`s it via `load_library` + `configure_from_loaded` instead of
//! `load_wasm_configured`.

// On the host build the guest-glue paths are cfg'd out; the native shell +
// struct + manifest fn are what's reachable. Silence never-used noise.
#![allow(dead_code)]
// fidius's `#[plugin_interface]` emits a check-cfg the workspace lint flags as
// unknown — benign (mirrors the loader's own allow; CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Prefixes the context's `name` into `result` — the native analogue of the
/// wasm `task-constructor-macro-fixture::Prefix`.
#[constructor(
    kind = task,
    name = "prefix",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Prefixes the context `name` into `result` (native-authored).",
    author = "CLOACI-T-0902"
)]
pub struct Prefix {
    /// Bound once per instance at load via the generated `configure` hook.
    #[config]
    prefix: String,
    /// Declared input, pulled from the task context (required).
    #[param(required)]
    name: String,
}

impl Prefix {
    /// The only thing the author writes: read the bound `#[config]` + `#[param]`
    /// and `set` an output key back into the context.
    fn execute(&self) -> Result<(), ConstructorError> {
        self.set("result", format!("{}{}", self.prefix, self.name));
        Ok(())
    }
}

/// A second kind in the SAME native suite: an accumulator that emits a boundary
/// only when an event's numeric `value` crosses the config-bound `threshold`.
/// Proves the generic native `load_native_member` path across a second holder
/// (`__ProviderAccumulator`) — and is the shape T-0904 builds its stream
/// accumulator on.
#[constructor(
    kind = accumulator,
    name = "threshold",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Emits a boundary when an event value crosses a configured threshold (native-authored).",
    author = "CLOACI-T-0902"
)]
pub struct Threshold {
    /// Bound once per instance at load via the generated `configure` hook.
    #[config]
    threshold: f64,
}

impl Threshold {
    /// Parse the event, emit `{crossed: value}` when `value >= threshold`
    /// (config-bound), else buffer (`Ok(None)`).
    fn ingest(&self, event_json: &str) -> Result<Option<String>, ConstructorError> {
        let event: serde_json::Value = serde_json::from_str(event_json)
            .map_err(|e| ConstructorError::msg(format!("decode event: {e}")))?;
        let value = event
            .get("value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ConstructorError::msg("event missing numeric `value`"))?;
        if value >= self.threshold {
            Ok(Some(serde_json::json!({ "crossed": value }).to_string()))
        } else {
            Ok(None)
        }
    }
}

/// A STREAM (loop-owning) accumulator (`mode = stream`, CLOACI-T-0904): instead of
/// a per-event `ingest`, it yields the WHOLE stream of boundary events. Here it
/// emits `count` synthetic boundaries `{"tick": <base + i>}` — the native analogue
/// of a Kafka source (T-0906 supplies the real rdkafka one). The returned iterator
/// owns its state (edition-2021 `-> impl Iterator` can't borrow `&self`).
#[constructor(
    kind = accumulator,
    mode = stream,
    name = "counter",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Emits `count` synthetic boundary events starting at `base` (native stream source).",
    author = "CLOACI-T-0904"
)]
pub struct Counter {
    /// First tick value; bound once at load.
    #[config]
    base: u64,
    /// How many boundary events to emit before the stream ends.
    #[config]
    count: u64,
}

impl Counter {
    /// The ONLY thing the author writes for a stream accumulator: return an
    /// iterator of boundary-JSON strings. Owns `base`/`count` (moved into the map).
    fn source(&self) -> impl Iterator<Item = String> {
        let base = self.base;
        (0..self.count).map(move |i| serde_json::json!({ "tick": base + i }).to_string())
    }
}

// The provider suite shell (CLOACI-A-0011): THREE kinds behind one native cdylib.
// For a HOST cdylib build, `constructor_provider!` emits the native variants
// (`crate = fidius_core`, feature-gated) → `__ProviderTask` + `__ProviderAccumulator`
// + `__ProviderStreamAccumulator` (server-streaming `source`) plugins the loader
// selects by kind/interface.
constructor_provider!(
    name = "native-task-provider-fixture",
    version = "0.1.0",
    contract = constructor_contract,
    task = [Prefix],
    accumulator = [Threshold],
    stream_accumulator = [Counter],
);
