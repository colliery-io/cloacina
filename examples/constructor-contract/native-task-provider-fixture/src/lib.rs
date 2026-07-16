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

// The provider suite shell (CLOACI-A-0011). For a HOST cdylib build,
// `constructor_provider!` emits the native variant (`crate = fidius_core`,
// `#[cfg(not(wasm32))]`) → the `__ProviderTask` plugin the loader selects.
constructor_provider!(
    name = "native-task-provider-fixture",
    version = "0.1.0",
    contract = constructor_contract,
    task = [Prefix],
);
