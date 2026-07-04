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

//! Demo (CLOACI-T-0836): a packaged workflow whose first node is a WASM
//! constructor-provider member.
//!
//! `reader` is `cloacina-provider-fs`'s `read_file`, resolved from the BUNDLED
//! provider at load (the compiler discovered the `from` ref, built the provider
//! to wasm, and stored it in `package_providers`) and executed in a sandbox that
//! can reach ONLY the granted `/etc` (default-closed otherwise). It reads
//! `/etc/hostname` — a REGULAR file docker bind-mounts into every container —
//! and the downstream `#[task]` summarizes what came through the sandbox.
//!
//! NOTE: deliberately NOT `/etc/os-release`, which is a SYMLINK to
//! `/usr/lib/os-release` in debian-slim: the WASI sandbox refuses symlinks that
//! escape the granted tree (proven live — the read fails closed with EPERM).

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "constructor_demo",
    description = "sandboxed provider read via a capability grant",
    author = "CLOACI-T-0836"
)]
pub mod constructor_demo {
    use super::*;

    constructor!(
        id = "reader",
        from = "cloacina-provider-fs@0.1.0",
        constructor = "read_file",
        config = { path = "/etc/hostname" },
        grants = { fs = ["ro:/etc"] },
    );

    #[task(id = "summarize", dependencies = ["reader"], retry_attempts = 0)]
    pub async fn summarize(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Own the value: `get` borrows the context, and `insert` needs it mutably.
        let contents = context
            .get("contents")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        context.insert("sandbox_read_bytes", serde_json::json!(contents.len()))?;
        context.insert("sandbox_read_hostname", serde_json::json!(contents.trim()))?;
        Ok(())
    }
}
