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

//! CLOACI-T-0829 — a TASK constructor with TWO `#[config]` fields, for proving
//! NAME-KEYED `config = { … }` binding.
//!
//! Declaration order is `prefix` THEN `suffix`. `execute` wraps the context `name`
//! as `"{prefix}{name}{suffix}"`, so the result is correct ONLY when each kwarg is
//! bound to its same-named `#[config]` field. A consumer that lists the kwargs in a
//! different order than this declaration order still gets the right answer iff the
//! `constructor!` consumer binds by NAME (the T-0829 fix), not by position.

// On the host build only the struct + manifest fn are reachable (the wasm guest
// glue is cfg'd out), so silence the never-used warnings.
#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Wraps the context `name` in a configured `prefix` + `suffix`.
#[constructor(
    kind = task,
    name = "affix",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Wraps the context `name` in a configured prefix + suffix.",
    author = "CLOACI-T-0829"
)]
pub struct Affix {
    /// Bound once per instance at load. DECLARED FIRST.
    #[config]
    prefix: String,
    /// Bound once per instance at load. DECLARED SECOND.
    #[config]
    suffix: String,
    /// Declared input, pulled from the task context (required).
    #[param(required)]
    name: String,
}

impl Affix {
    /// Read the bound `#[config]` + `#[param]` fields and `set` the wrapped result.
    fn execute(&self) -> Result<(), ConstructorError> {
        self.set(
            "result",
            format!("{}{}{}", self.prefix, self.name, self.suffix),
        );
        Ok(())
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(
    name = "affix",
    version = "0.1.0",
    contract = constructor_contract,
    task = [Affix],
);
