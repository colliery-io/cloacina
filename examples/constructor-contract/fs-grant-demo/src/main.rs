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

//! CLOACI-T-0834 — runnable proof of the constructor capability-GRANT model.
//!
//! ONE author crate (`../fs-grant-constructor`) provides ONE constructor, `read_file`,
//! whose `execute` does `std::fs::read_to_string(self.path)` inside a WASM sandbox. We
//! run that IDENTICAL constructor in two `#[workflow]`s that differ in exactly one way:
//!
//!   * `granted`   wires it with `grants = { fs = ["ro:<dir>"] }` → the read SUCCEEDS.
//!   * `ungranted` wires it with NO `grants` field (default-closed) → the read is DENIED.
//!
//! fidius's capability model is fail-closed by construction: with no `fs` grant the
//! guest's `WasiCtx` carries zero filesystem capabilities, so the read errors, the
//! constructor returns `Err`, and the node — and the whole workflow — fail closed. The
//! tenant, not the constructor, decides what the sandbox can reach.
//!
//! NOTE: uses hard-coded unix paths under `/tmp` so the compile-time `constructor!`
//! grant/config string literals match the runtime file. This is a dev demo for
//! macOS/Linux; it is not meant to run on Windows.

// The `#[workflow]` macro emits `#[cfg(feature = "packaged")]` arms (resolved against
// this destination crate, which has no `packaged` feature); benign.
#![allow(unexpected_cfgs)]

use std::path::PathBuf;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::{set_provider_search_path, unpack_provider_archive};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError, WorkflowExecutor, WorkflowStatus};
use serde_json::json;

// Fixed on-disk locations. The `constructor!` macro needs STRING LITERALS for its
// `config` path and `grants`, so these must be `const`s the macro can see — and the
// runtime file we write below must live at exactly the same path. Unix-only (see the
// module note above).
const DATA_DIR: &str = "/tmp/cloacina-fs-grant-demo";
const SECRET: &str = "/tmp/cloacina-fs-grant-demo/secret.txt";
const SECRET_CONTENTS: &str = "the launch codes are 0000";

// ===========================================================================
// `granted` — the constructor IS handed `fs = ["ro:<dir>"]`, so the sandboxed
// read reaches the host file. A downstream #[task] echoes what it read.
// ===========================================================================
#[workflow(
    name = "granted",
    description = "read_file with an fs grant — the sandboxed read succeeds"
)]
pub mod granted {
    use super::*;

    constructor!(
        id = "reader",
        from = "read_file@0.1.0",
        constructor = "read_file",
        config = { path = "/tmp/cloacina-fs-grant-demo/secret.txt" },
        grants = { fs = ["ro:/tmp/cloacina-fs-grant-demo"] },
    );

    #[task(id = "show_granted", dependencies = ["reader"])]
    pub async fn show_granted(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let contents = context
            .get("contents")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        println!("    [granted]   downstream task read through the sandbox: {contents:?}");
        context.insert("echoed", json!(contents))?;
        Ok(())
    }
}

// ===========================================================================
// `ungranted` — the SAME constructor, wired with NO `grants` field at all. The
// default-closed WasiCtx denies the read, so the constructor's `execute` errors
// and the node fails. The downstream #[task] should never run.
// ===========================================================================
#[workflow(
    name = "ungranted",
    description = "read_file with NO fs grant — the sandboxed read is denied"
)]
pub mod ungranted {
    use super::*;

    constructor!(
        id = "reader_denied",
        from = "read_file@0.1.0",
        constructor = "read_file",
        config = { path = "/tmp/cloacina-fs-grant-demo/secret.txt" },
    );

    #[task(id = "show_ungranted", dependencies = ["reader_denied"])]
    pub async fn show_ungranted(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // The upstream `reader_denied` node fails closed, so it never sets `contents`.
        // The security property is that the SECRET never reaches this task — empty here.
        let contents = context
            .get("contents")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("    [ungranted] downstream task saw NO secret (contents={contents:?})");
        Ok(())
    }
}

/// Package `../fs-grant-constructor` into an (unsigned) provider archive and unpack it
/// into `providers` — the `stage_into` of the workflow-node test, inlined for the demo.
fn stage_constructor(work_dir: &std::path::Path, providers: &PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fs-grant-constructor");
    let archive = work_dir.join("fs-grant-constructor.cloacina");

    let opts = ProviderPackageOptions {
        crate_dir,
        output: Some(archive.clone()),
        sign_key: None,
        manifest_bin: "emit_manifest".to_string(),
        release: true,
    };
    println!("==> Packaging fs-grant-constructor to a WASM provider (slow on first run)...");
    package_constructor_provider(&opts).expect("package_constructor_provider");
    unpack_provider_archive(&archive, providers, &[]).expect("unpack provider archive");
    set_provider_search_path(providers);
    println!("==> Provider staged at {}", providers.display());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("fs_grant_demo=info,cloacina=warn")
        .init();

    println!();
    println!("== Constructor capability-grant demo (CLOACI-T-0834) ==");
    println!("   Same `read_file` constructor; only the tenant's `grants` differ.");
    println!();

    // 1) Materialize the secret the constructor will try to read.
    std::fs::create_dir_all(DATA_DIR)?;
    std::fs::write(SECRET, SECRET_CONTENTS)?;
    println!("Wrote secret to {SECRET}: {SECRET_CONTENTS:?}");
    println!();

    // 2) Build + stage the WASM provider. Keep `work` alive for the whole run.
    let work = tempfile::TempDir::new()?;
    let providers = work.path().join("providers");
    stage_constructor(work.path(), &providers);
    println!();

    // 3) Embedded runner against in-memory SQLite (background reconciler off).
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()?;
    let runner = DefaultRunner::with_config(":memory:", config).await?;

    // -- Case 1: GRANTED -----------------------------------------------------
    println!("--- Case 1: `granted` (fs = [\"ro:{DATA_DIR}\"]) ---");
    let granted_result = runner.execute("granted", Context::new()).await?;
    let read_contents = granted_result
        .final_context
        .get("contents")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if granted_result.status == WorkflowStatus::Completed
        && read_contents.as_deref() == Some(SECRET_CONTENTS)
    {
        println!(
            "    [granted]   SUCCESS: constructor read the secret THROUGH the grant: {:?}",
            read_contents.unwrap()
        );
    } else {
        eprintln!(
            "    [granted]   UNEXPECTED FAILURE: status={:?} contents={:?} error={:?}",
            granted_result.status, read_contents, granted_result.error_message
        );
        runner.shutdown().await?;
        std::process::exit(1);
    }
    println!();

    // -- Case 2: UNGRANTED ---------------------------------------------------
    println!("--- Case 2: `ungranted` (no `grants` field — default-closed) ---");
    let ungranted_result = runner.execute("ungranted", Context::new()).await;

    let denied = match &ungranted_result {
        // execute() may return Err (workflow failed) ...
        Err(e) => {
            println!("    [ungranted] DENIED as expected (no fs grant): {e}");
            true
        }
        // ... or Ok with a non-Completed status + no contents read.
        Ok(result) => {
            let leaked = result
                .final_context
                .get("contents")
                .and_then(|v| v.as_str());
            if result.status == WorkflowStatus::Completed && leaked == Some(SECRET_CONTENTS) {
                false
            } else {
                println!(
                    "    [ungranted] DENIED as expected (no fs grant): status={:?} error={:?}",
                    result.status, result.error_message
                );
                true
            }
        }
    };

    if !denied {
        eprintln!();
        eprintln!(
            "!!! SECURITY FAILURE: the `ungranted` workflow READ THE SECRET without an fs grant."
        );
        eprintln!("!!! The default-closed capability guarantee is BROKEN.");
        runner.shutdown().await?;
        std::process::exit(1);
    }
    println!();

    println!("== Result ==");
    println!("   granted   → read the secret (the tenant granted fs access).");
    println!("   ungranted → denied (no grant; the sandbox reached nothing).");
    println!("   The constructor code was IDENTICAL; only the tenant's grants changed.");

    runner.shutdown().await?;
    Ok(())
}
