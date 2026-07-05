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

//! CLOACI-T-0829 — a TRIGGER constructor authored with the `#[constructor]` macro.
//!
//! The macro counterpart to the hand-written `trigger-constructor-fixture`: the
//! author writes ONLY a struct (with `#[config]` fields, bound once per instance at
//! load) and a single `poll` body that returns a FIRE DECISION. `#[constructor(kind
//! = trigger, ...)]` generates the fidius `TriggerConstructor` trait + impl +
//! `configure` + the `TriggerInvocation`/`PollOutcome` JSON wire, plus a
//! `pub fn __constructor_manifest() -> ConstructorManifest`. The generated guest glue
//! is `#[cfg(target_arch = "wasm32")]`, so this crate also builds on the host (the
//! `emit_manifest` bin reads `__constructor_manifest()` to produce `constructor.json`).
//!
//! `contract = constructor_contract` points the macro at this example's vendored,
//! wasm-safe contract crate; the real-integration default is
//! `::cloacina_constructor_contract`.

// On the host build only the struct + manifest fn are reachable (the wasm guest
// glue that calls `poll` / reads the fields is cfg'd out), so silence the
// never-used warnings rather than contort the proof.
#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Fires (or skips) on each poll based on its bound config — the macro analogue of
/// the raw `trigger-constructor-fixture`.
#[constructor(
    kind = trigger,
    name = "heartbeat",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Fires (or skips) on each poll based on its bound config (macro-authored).",
    author = "CLOACI-T-0829"
)]
pub struct Heartbeat {
    /// Whether this trigger instance fires when polled (bound once at load).
    #[config]
    should_fire: bool,
    /// The message written into the fired context's `reason` key.
    #[config]
    message: String,
}

impl Heartbeat {
    /// The ONLY thing the author writes: decide whether to fire. `Ok(true)` fires
    /// with whatever was `set` into the fire context; `Ok(false)` skips this tick.
    fn poll(&self) -> Result<bool, ConstructorError> {
        if self.should_fire {
            self.set("reason", self.message.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(
    name = "heartbeat",
    version = "0.1.0",
    contract = constructor_contract,
    trigger = [Heartbeat],
);
