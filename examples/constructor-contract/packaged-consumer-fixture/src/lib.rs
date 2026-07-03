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

//! CLOACI-T-0836 — a PACKAGED workflow that consumes a constructor provider.
//!
//! In a `packaged` cdylib build the `constructor!` node lowers to a
//! `ConstructorEntry` DECLARATION (the cdylib cannot link the WASM loader); the
//! reconciler's Step 5b resolves it against the package's BUNDLED
//! `cloacina-provider-fs` provider and injects it into this workflow's DAG next to
//! the ordinary `#[task]` below.
//!
//! The `#[config] path` and `grants` are compile-time string literals, so the test
//! that runs this fixture stages the secret at exactly this path (mirrors
//! fs-grant-demo).

use cloacina_workflow::{task, workflow, Context, TaskError};

// I-0102 / T-C: unified plugin shell (projects ConstructorEntry declarations into
// `get_constructor_metadata()` for the reconciler).
cloacina_workflow_plugin::package!();

#[workflow(
    name = "provider_consumer",
    description = "packaged workflow consuming the bundled cloacina-provider-fs suite",
    author = "CLOACI-T-0836"
)]
pub mod provider_consumer {
    use super::*;

    constructor!(
        id = "reader",
        from = "cloacina-provider-fs@0.1.0",
        constructor = "read_file",
        config = { path = "/tmp/cloacina-packaged-consumer-fixture/secret.txt" },
        grants = { fs = ["ro:/tmp/cloacina-packaged-consumer-fixture"] },
    );

    #[task(id = "echo", dependencies = ["reader"], retry_attempts = 0)]
    pub async fn echo(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let contents = context
            .get("contents")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        context.insert("echoed", serde_json::json!(contents))?;
        Ok(())
    }
}
