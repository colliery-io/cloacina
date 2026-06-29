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

//! CLOACI-T-0826 — a TASK operator authored with the `#[operator]` macro.
//!
//! Contrast with the sibling `task-operator-fixture`, which hand-writes the raw
//! fidius contract (the `#[plugin_interface] TaskOperator` trait, the
//! `#[plugin_impl(config = Config)]` impl, the `configure` hook, and the
//! `TaskInvocation`/`TaskOutcome` JSON plumbing). Here the author writes ONLY:
//!
//!   * the operator struct, tagging each field `#[config]` (bound once per
//!     instance at load) or `#[param]` (pulled from the task context), and
//!   * the `execute` body.
//!
//! `#[operator(kind = task, ...)]` generates everything else, including a
//! `pub fn __operator_manifest() -> OperatorManifest`. The generated guest glue
//! is `#[cfg(target_arch = "wasm32")]`, so this crate also builds on the host
//! (the `emit_manifest` bin reads `__operator_manifest()` to produce
//! `operator.json`).
//!
//! `contract = operator_contract` points the macro at this example's vendored,
//! wasm-safe contract crate; the real-integration default is
//! `::cloacina_operator_contract`.

// On the host build only the struct + manifest fn are reachable (the wasm guest
// glue that calls `execute` / reads the fields is cfg'd out), so silence the
// never-used warnings rather than contort the proof.
#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (mirrors the loader's own
// allow; see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use cloacina_macros::operator;
use operator_contract::OperatorError;

/// Prefixes the context's `name` into `result` — the macro analogue of the raw
/// `task-operator-fixture`.
#[operator(
    kind = task,
    name = "prefix",
    version = "0.1.0",
    contract = operator_contract,
    description = "Prefixes the context `name` into `result` (macro-authored).",
    author = "CLOACI-T-0826"
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
    /// The ONLY thing the author writes: read the bound `#[config]` + `#[param]`
    /// fields and `set` an output key back into the context.
    fn execute(&self) -> Result<(), OperatorError> {
        self.set("result", format!("{}{}", self.prefix, self.name));
        Ok(())
    }
}
