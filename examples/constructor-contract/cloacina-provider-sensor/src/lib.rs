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

//! CLOACI-T-0825 — seed TRIGGER provider: a file-presence sensor.
//!
//! The classic "sensor" (Airflow's FileSensor): fires when the configured path
//! exists, skips when it doesn't. The check runs INSIDE the WASM sandbox, so it
//! only sees the filesystem the consuming workflow granted — with no
//! `fs = ["ro:<dir>"]` grant the path is invisible (default-closed) and the
//! trigger simply never fires.
//!
//! Same author-crate shape as `cloacina-provider-fs`: the clean constructor form
//! (struct + one `poll` body) with the guest glue generated under
//! `#[cfg(target_arch = "wasm32")]`, so the crate also builds on the host for
//! `emit_manifest`.

#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (mirrors the loader's own allow).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Fires when the bound `#[config] path` exists inside the sandbox; skips
/// otherwise. Visibility of the path is grant-gated (`fs = ["ro:<dir>"]`).
#[constructor(
    kind = trigger,
    name = "file_present",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Fires when a configured path exists inside the sandbox (grant-gated file sensor).",
    author = "CLOACI-T-0825"
)]
pub struct FilePresent {
    /// The path whose presence fires the trigger, bound once per instance at load.
    #[config]
    path: String,
}

impl FilePresent {
    /// Fire when the path exists. `Path::exists()` swallows sandbox denials as
    /// `false`, so an ungranted path reads as "not present" — the trigger fails
    /// closed by never firing rather than erroring every poll.
    fn poll(&self) -> Result<bool, ConstructorError> {
        if std::path::Path::new(&self.path).exists() {
            self.set("path", self.path.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// The provider suite shell (CLOACI-A-0011): one member behind one component.
constructor_provider!(contract = constructor_contract, trigger = [FilePresent],);
