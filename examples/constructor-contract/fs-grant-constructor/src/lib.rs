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

//! CLOACI-T-0834 — a TASK constructor that reads a file from inside its WASM sandbox.
//!
//! The author writes ONLY the constructor struct (one `#[config] path`, bound once at
//! load) and the `execute` body that does `std::fs::read_to_string(&self.path)`. The
//! constructor code is IDENTICAL regardless of who instantiates it.
//!
//! What changes is the CONSUMER's `grants = { fs = [..] }` at the `constructor!(...)`
//! call site. fidius's capability model is default-closed: with no `fs` grant the
//! guest's `WasiCtx` has zero filesystem capabilities, so the `read_to_string` fails
//! and `execute` returns an error — the node (and workflow) fail closed. Grant
//! `fs = ["ro:<dir>"]` and the same read succeeds. See ../fs-grant-demo for the
//! end-to-end proof of both outcomes.
//!
//! `#[constructor(kind = task, ...)]` generates the raw fidius contract plus a
//! `pub fn __constructor_manifest() -> ConstructorManifest`. The generated guest glue
//! is `#[cfg(target_arch = "wasm32")]`, so this crate also builds on the host (the
//! `emit_manifest` bin reads `__constructor_manifest()` to produce `constructor.json`).

// On the host build only the struct + manifest fn are reachable (the wasm guest glue
// that calls `execute` / reads the fields is cfg'd out), so silence the never-used
// warnings rather than contort the proof.
#![allow(dead_code)]
// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate the
// workspace check-cfg lint flags as unknown — benign (mirrors the loader's own allow).
#![allow(unexpected_cfgs)]

use cloacina_macros::{constructor, constructor_provider};
use constructor_contract::ConstructorError;

/// Reads the file at the bound `#[config] path` and stores its contents in the
/// context under `contents`. The read only succeeds if the consuming workflow
/// granted `fs = ["ro:<dir>"]` (default-closed otherwise).
#[constructor(
    kind = task,
    name = "read_file",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Reads a file from inside the sandbox; succeeds only with an fs grant.",
    author = "CLOACI-T-0834"
)]
pub struct ReadFile {
    /// The absolute file path to read, bound once per instance at load.
    #[config]
    path: String,
}

impl ReadFile {
    /// The ONLY thing the author writes: read the bound `#[config] path` through the
    /// sandbox and `set` the contents back into the context. The read reaches the host
    /// filesystem ONLY if the tenant granted `fs = ["ro:<dir>"]`; otherwise fidius's
    /// zero-grant `WasiCtx` denies it and this returns an error (fail-closed).
    fn execute(&self) -> Result<(), ConstructorError> {
        let contents = std::fs::read_to_string(&self.path)
            .map_err(|e| ConstructorError::msg(format!("read {}: {e}", self.path)))?;
        self.set("contents", contents);
        Ok(())
    }
}

/// Writes the context's `contents` to the bound `#[config] path` and reports the
/// byte count back under `written_bytes`. The SECOND member of this suite — it
/// proves the provider-as-suite model (CLOACI-A-0011): two constructors in one
/// crate, one component, selected by `constructor = "read_file" | "write_file"`.
/// The write only succeeds if the consuming workflow granted `fs = ["rw:<dir>"]`
/// (default-closed otherwise — same fail-closed semantics as the read).
#[constructor(
    kind = task,
    name = "write_file",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Writes a file from inside the sandbox; succeeds only with a writable fs grant.",
    author = "CLOACI-T-0837"
)]
pub struct WriteFile {
    /// The absolute file path to write, bound once per instance at load.
    #[config]
    path: String,
    /// The content to write, pulled from the task context (required).
    #[param(required)]
    contents: String,
}

impl WriteFile {
    /// The ONLY thing the author writes: write the `#[param] contents` to the bound
    /// `#[config] path` through the sandbox. Reaches the host filesystem ONLY if the
    /// tenant granted a writable `fs` grant; otherwise the zero-grant `WasiCtx`
    /// denies it and this returns an error (fail-closed).
    fn execute(&self) -> Result<(), ConstructorError> {
        std::fs::write(&self.path, &self.contents)
            .map_err(|e| ConstructorError::msg(format!("write {}: {e}", self.path)))?;
        self.set("written_bytes", self.contents.len() as i64);
        Ok(())
    }
}

// The provider suite: one crate → one component → two task members
// (`read_file` + `write_file`), indexed by the `provider.json` this emits.
constructor_provider!(
    name = "cloacina-provider-fs",
    version = "0.1.0",
    contract = constructor_contract,
    task = [ReadFile, WriteFile],
);
