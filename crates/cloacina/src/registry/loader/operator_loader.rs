/*
 *  Copyright 2025-2026 Colliery Software
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

// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate that
// the workspace's check-cfg lint flags as unknown — benign (see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

//! WASM operator loader + task-executor adapter (CLOACI-I-0132 / T-0823).
//!
//! Loads a WASM **task operator** package (a `.wasm` component + a sidecar
//! `operator.json` manifest) and wraps the configured fidius handle as a
//! [`crate::task::Task`] the existing async executor can run unchanged.
//!
//! Gated behind the default-OFF `operators-wasm` feature: enabling it turns on
//! `fidius-host`'s `wasm` feature (→ wasmtime/cranelift) and pulls
//! `fidius-macro` + `cloacina-operator-contract`. The default cloacina build
//! pulls none of this (verified via `cargo tree`).
//!
//! ## Flow
//!
//! 1. Read the package's `operator.json` into an [`OperatorManifest`].
//! 2. Require `primitive_kind == Task` (trigger/accumulator/reactor loading is
//!    CLOACI-T-0824) and `interface_version` matching the contract.
//! 3. `load_wasm_configured(component, &config)` — config binds ONCE at load.
//! 4. Wrap the handle as a [`WasmTaskOperator`]; its `impl Task` is the
//!    **async↔sync bridge**: serialize `Context` → `JSON(TaskInvocation)` →
//!    `spawn_blocking` the wasmtime `call_method` → parse `JSON(TaskOutcome)` →
//!    rebuild `Context`, surfacing a failed outcome as a [`TaskError`].
//!
//! ## Deferred (CLOACI-T-0824)
//!
//! The trigger `poll` / accumulator `ingest` / reactor `evaluate` bridges and
//! the Runtime-registry wiring (registering a loaded operator into the scheduler
//! registries) live in T-0824. The `#[operator]` authoring macro is T-0826.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;

use cloacina_operator_contract::{
    OperatorManifest, PrimitiveKind, TaskInvocation, TaskOutcome, METHOD_EXECUTE,
    TASK_OPERATOR_INTERFACE_VERSION,
};
use fidius_host::PluginHost;

use crate::context::Context;
use crate::error::TaskError;
use crate::registry::error::LoaderError;
use crate::task::{Task, TaskNamespace};

/// Host-side re-declaration of the TASK-operator interface. This is the SAME
/// trait shape the guest implements; declaring it with `crate = "fidius_core"`
/// makes the fidius macro emit the matching `TaskOperator_WASM_DESCRIPTOR` (in
/// the companion `__fidius_TaskOperator` module) that the loader links the
/// component against. The `fidius-interface-hash` export then gates integrity at
/// load (CLOACI-T-0821).
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait TaskOperator: Send + Sync {
    /// `JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out. SYNC.
    fn execute(&self, invocation_json: String) -> String;
}

/// The sidecar manifest filename inside a WASM operator package.
pub const OPERATOR_MANIFEST_FILE: &str = "operator.json";

/// A loaded, configured WASM task operator wrapped as a cloacina [`Task`].
///
/// Holds the configured fidius [`PluginHandle`](fidius_host::PluginHandle) (the
/// `configure` hook bound the operator's config once at load, so per-call
/// `execute` dispatches on the already-configured persistent store). The handle
/// is held behind an [`Arc`] so the async bridge can hand a `'static + Send`
/// clone to `spawn_blocking`; concurrent calls are serialized inside the handle
/// (the WASM backend guards its store with a mutex).
pub struct WasmTaskOperator {
    id: String,
    handle: Arc<fidius_host::PluginHandle>,
    dependencies: Vec<TaskNamespace>,
}

impl WasmTaskOperator {
    /// The operator's declared name (its task id).
    pub fn name(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for WasmTaskOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTaskOperator")
            .field("id", &self.id)
            .field("dependencies", &self.dependencies)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Task for WasmTaskOperator {
    /// The async↔sync bridge: serialize the context, hop into the blocking
    /// wasmtime call on a `spawn_blocking` thread, and rebuild the context from
    /// the outcome. A failed [`TaskOutcome`] becomes a [`TaskError`].
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        let id = self.id.clone();

        // Context -> JSON(TaskInvocation).
        let context_json = context
            .to_json()
            .map_err(|e| exec_err(&id, format!("serialize context: {e}")))?;
        let inv_json = serde_json::to_string(&TaskInvocation { context_json })
            .map_err(|e| exec_err(&id, format!("serialize invocation: {e}")))?;

        // Blocking wasmtime call off the async executor.
        let handle = self.handle.clone();
        let call_id = id.clone();
        let out_json: String = tokio::task::spawn_blocking(move || {
            handle.call_method::<_, String>(METHOD_EXECUTE, &(inv_json,))
        })
        .await
        .map_err(|e| exec_err(&call_id, format!("operator task join: {e}")))?
        .map_err(|e| exec_err(&call_id, format!("operator FFI call: {e}")))?;

        // JSON(TaskOutcome) -> Context (or surface the failure).
        let outcome: TaskOutcome = serde_json::from_str(&out_json)
            .map_err(|e| exec_err(&id, format!("parse outcome: {e}")))?;

        if outcome.success {
            let updated = outcome
                .context_json
                .ok_or_else(|| exec_err(&id, "successful outcome missing context_json"))?;
            Context::from_json(updated)
                .map_err(|e| exec_err(&id, format!("rebuild context: {e}")))
        } else {
            Err(exec_err(
                &id,
                outcome
                    .error
                    .unwrap_or_else(|| "operator reported failure with no message".to_string()),
            ))
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

fn exec_err(task_id: &str, message: impl Into<String>) -> TaskError {
    TaskError::ExecutionFailed {
        message: message.into(),
        task_id: task_id.to_string(),
        timestamp: Utc::now(),
    }
}

/// Read and parse a package's sidecar `operator.json` manifest.
pub fn read_operator_manifest(package_dir: &Path) -> Result<OperatorManifest, LoaderError> {
    let path = package_dir.join(OPERATOR_MANIFEST_FILE);
    let raw = std::fs::read_to_string(&path).map_err(|e| LoaderError::FileSystem {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    OperatorManifest::from_json(&raw).map_err(|e| LoaderError::ManifestParse {
        reason: format!("{OPERATOR_MANIFEST_FILE}: {e}"),
    })
}

/// Load a WASM **task** operator and return it as a runnable [`Task`].
///
/// `search_path` is a directory containing operator package subdirectories;
/// `package_name` is the `[package].name` in the package's `package.toml` (what
/// fidius matches on). `config` binds the operator's per-instance configuration
/// once at load (the `configure` hook) — two loads with different configs yield
/// two independently-configured operators.
///
/// Fails closed (a clear [`LoaderError`]) if the package is missing, the
/// manifest is unreadable, the primitive is not a `Task`, or the interface
/// version doesn't match the contract. (fidius's `fidius-interface-hash` export
/// additionally gates ABI integrity inside `load_wasm_configured`.)
pub fn load_task_operator<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
) -> Result<Arc<dyn Task>, LoaderError> {
    let search_path = search_path.as_ref();

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    // Resolve the package directory so we can read its sidecar manifest.
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm operator package '{package_name}': {e}"),
        })?;

    let manifest = read_operator_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Task {
        return Err(LoaderError::Validation {
            reason: format!(
                "operator '{}' is {:?}, not Task; trigger/accumulator/reactor loading is CLOACI-T-0824",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TASK_OPERATOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "operator '{}' declares task-operator interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TASK_OPERATOR_INTERFACE_VERSION
            ),
        });
    }

    // Config binds ONCE here; the integrity hash is checked inside fidius.
    let handle = host
        .load_wasm_configured(
            package_name,
            &__fidius_TaskOperator::TaskOperator_WASM_DESCRIPTOR,
            config,
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured: {e}"),
        })?;

    Ok(Arc::new(WasmTaskOperator {
        id: manifest.name,
        handle: Arc::new(handle),
        dependencies: Vec::new(),
    }))
}
