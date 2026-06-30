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

//! WASM constructor loader + primitive adapters (CLOACI-I-0132 / T-0823, T-0824).
//!
//! Loads a WASM **constructor** package (a `.wasm` component + a sidecar
//! `constructor.json` manifest) and wraps the configured fidius handle as the
//! cloacina primitive the manifest's `primitive_kind` names — a
//! [`crate::task::Task`] (T-0823) or a [`crate::trigger::Trigger`] (T-0824) —
//! that the existing async executor / scheduler can run unchanged, then
//! registers it into a [`Runtime`] so it participates in workflows/schedules
//! exactly like a macro-authored one.
//!
//! Gated behind the default-OFF `constructors-wasm` feature: enabling it turns on
//! `fidius-host`'s `wasm` feature (→ wasmtime/cranelift) and pulls
//! `fidius-macro` + `cloacina-constructor-contract`. The default cloacina build
//! pulls none of this (verified via `cargo tree`).
//!
//! ## Flow (per primitive)
//!
//! 1. Read the package's `constructor.json` into an [`ConstructorManifest`].
//! 2. Require the `primitive_kind` the caller asked for and an
//!    `interface_version` matching that primitive's contract.
//! 3. `load_wasm_configured(component, &config)` — config binds ONCE at load.
//! 4. Wrap the handle as the matching adapter and run the **async↔sync bridge**:
//!    serialize the async input → `JSON(...Invocation)` → `spawn_blocking` the
//!    wasmtime `call_method` → parse `JSON(...Outcome)` → rebuild the async
//!    result, surfacing a failed outcome as the primitive's normal error.
//!    - TASK: `Context` → [`TaskInvocation`] / [`TaskOutcome`] → `Context`.
//!    - TRIGGER: `()` → [`TriggerInvocation`] / [`PollOutcome`] →
//!      [`TriggerResult`].
//!
//! ## Runtime registration
//!
//! [`load_constructor`] reads the manifest and dispatches on `primitive_kind`,
//! registering the configured primitive into a [`Runtime`] (Task →
//! [`Runtime::register_task`], Trigger → [`Runtime::register_trigger`]). The
//! registered constructor hands out `Arc` clones that share the one configured
//! fidius handle, so every `get_task`/`get_trigger` call dispatches into the
//! same sandboxed instance.
//!
//! ## Accumulator + reactor (CLOACI-T-0828)
//!
//! These two primitives are NOT plain `Runtime` constructors, so they are wired
//! differently from task/trigger:
//!
//!   * ACCUMULATOR — [`WasmAccumulatorConstructor`] implements
//!     [`Accumulator`](crate::computation_graph::accumulator::Accumulator)
//!     directly; [`load_accumulator_constructor`] returns it for
//!     `accumulator_runtime` to drive. `Accumulator::process` is SYNC, so the
//!     bridge calls the blocking wasmtime `ingest` directly (no `spawn_blocking`).
//!   * REACTOR — the reactor's firing decision is now pluggable via the
//!     [`ReactorFireDecider`] seam added to
//!     [`Reactor`](crate::computation_graph::reactor::Reactor)
//!     (`with_evaluator`). [`WasmReactorConstructor`] implements it, bridging the
//!     async decision to a `spawn_blocking` `evaluate`. The bridge + seam are
//!     proven against `Reactor` directly; threading a reactor constructor through
//!     the CG SCHEDULER's package-load path is a documented remaining lift.
//!
//! The `#[constructor]` authoring macro covers all four kinds (task/trigger T-0826,
//! accumulator/reactor T-0828).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;

use cloacina_constructor_contract::{
    AccumulatorInvocation, AccumulatorOutcome, ConstructorManifest, PollOutcome, PrimitiveKind,
    ReactorInvocation, ReactorOutcome, TaskInvocation, TaskOutcome, TriggerInvocation,
    ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION, METHOD_EVALUATE, METHOD_EXECUTE, METHOD_INGEST,
    METHOD_POLL, REACTOR_CONSTRUCTOR_INTERFACE_VERSION, TASK_CONSTRUCTOR_INTERFACE_VERSION,
    TRIGGER_CONSTRUCTOR_INTERFACE_VERSION,
};
use fidius_host::PluginHost;

use super::grants::{lint_unmet_intents, translate, GrantSpec, ResolvedGrants};

use crate::computation_graph::accumulator::Accumulator;
use crate::computation_graph::reactor::ReactorFireDecider;
use crate::computation_graph::types::InputCache;
use crate::context::Context;
use crate::error::TaskError;
use crate::registry::error::LoaderError;
use crate::runtime::Runtime;
use crate::task::{Task, TaskNamespace};
use crate::trigger::{Trigger, TriggerResult};
// The leaf-crate `Trigger` trait (re-exported as `crate::trigger::Trigger`)
// returns `cloacina_workflow::TriggerError`, NOT the engine-local
// `crate::trigger::TriggerError`, so the `impl Trigger` must use that type.
use cloacina_workflow::TriggerError;

/// Host-side re-declaration of the TASK-constructor interface. This is the SAME
/// trait shape the guest implements; declaring it with `crate = "fidius_core"`
/// makes the fidius macro emit the matching `TaskConstructor_WASM_DESCRIPTOR` (in
/// the companion `__fidius_TaskConstructor` module) that the loader links the
/// component against. The `fidius-interface-hash` export then gates integrity at
/// load (CLOACI-T-0821).
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait TaskConstructor: Send + Sync {
    /// `JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out. SYNC.
    fn execute(&self, invocation_json: String) -> String;
}

/// Host-side re-declaration of the TRIGGER-constructor interface (CLOACI-T-0824).
/// Same trait shape the guest implements; the fidius macro emits the matching
/// `TriggerConstructor_WASM_DESCRIPTOR` (in `__fidius_TriggerConstructor`) the loader
/// links the component against. Single SYNC method — the WASM analogue of the
/// async `cloacina_workflow::Trigger::poll`.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait TriggerConstructor: Send + Sync {
    /// `JSON(TriggerInvocation)` in -> `JSON(PollOutcome)` out. SYNC.
    fn poll(&self, invocation_json: String) -> String;
}

/// The sidecar manifest filename inside a WASM constructor package.
pub const CONSTRUCTOR_MANIFEST_FILE: &str = "constructor.json";

/// A loaded, configured WASM task constructor wrapped as a cloacina [`Task`].
///
/// Holds the configured fidius [`PluginHandle`](fidius_host::PluginHandle) (the
/// `configure` hook bound the constructor's config once at load, so per-call
/// `execute` dispatches on the already-configured persistent store). The handle
/// is held behind an [`Arc`] so the async bridge can hand a `'static + Send`
/// clone to `spawn_blocking`; concurrent calls are serialized inside the handle
/// (the WASM backend guards its store with a mutex).
pub struct WasmTaskConstructor {
    id: String,
    handle: Arc<fidius_host::PluginHandle>,
    dependencies: Vec<TaskNamespace>,
}

impl WasmTaskConstructor {
    /// The constructor's declared name (its task id).
    pub fn name(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for WasmTaskConstructor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTaskConstructor")
            .field("id", &self.id)
            .field("dependencies", &self.dependencies)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Task for WasmTaskConstructor {
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
        .map_err(|e| exec_err(&call_id, format!("constructor task join: {e}")))?
        .map_err(|e| exec_err(&call_id, format!("constructor FFI call: {e}")))?;

        // JSON(TaskOutcome) -> Context (or surface the failure).
        let outcome: TaskOutcome = serde_json::from_str(&out_json)
            .map_err(|e| exec_err(&id, format!("parse outcome: {e}")))?;

        if outcome.success {
            let updated = outcome
                .context_json
                .ok_or_else(|| exec_err(&id, "successful outcome missing context_json"))?;
            Context::from_json(updated).map_err(|e| exec_err(&id, format!("rebuild context: {e}")))
        } else {
            Err(exec_err(
                &id,
                outcome
                    .error
                    .unwrap_or_else(|| "constructor reported failure with no message".to_string()),
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

/// Read and parse a package's sidecar `constructor.json` manifest.
pub fn read_constructor_manifest(package_dir: &Path) -> Result<ConstructorManifest, LoaderError> {
    let path = package_dir.join(CONSTRUCTOR_MANIFEST_FILE);
    let raw = std::fs::read_to_string(&path).map_err(|e| LoaderError::FileSystem {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    ConstructorManifest::from_json(&raw).map_err(|e| LoaderError::ManifestParse {
        reason: format!("{CONSTRUCTOR_MANIFEST_FILE}: {e}"),
    })
}

/// Load a WASM **task** constructor and return it as a runnable [`Task`].
///
/// `search_path` is a directory containing constructor package subdirectories;
/// `package_name` is the `[package].name` in the package's `package.toml` (what
/// fidius matches on). `config` binds the constructor's per-instance configuration
/// once at load (the `configure` hook) — two loads with different configs yield
/// two independently-configured constructors.
///
/// Fails closed (a clear [`LoaderError`]) if the package is missing, the
/// manifest is unreadable, the primitive is not a `Task`, or the interface
/// version doesn't match the contract. (fidius's `fidius-interface-hash` export
/// additionally gates ABI integrity inside `load_wasm_configured`.)
pub fn load_task_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    grants: &ResolvedGrants,
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
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_constructor_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Task {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Task; trigger/accumulator/reactor loading is CLOACI-T-0824",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TASK_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares task-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TASK_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    // Config binds ONCE here; the integrity hash is checked inside fidius. The
    // tenant's translated grants override the manifest caps + supply the egress
    // policy (default-closed when `grants` is deny-all).
    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_TaskConstructor::TaskConstructor_WASM_DESCRIPTOR,
            config,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(Arc::new(WasmTaskConstructor {
        id: manifest.name,
        handle: Arc::new(handle),
        dependencies: Vec::new(),
    }))
}

// ===========================================================================
// Packed provider resolution (CLOACI-T-0827)
// ===========================================================================

/// Unpack a packed **provider package** archive (a `.cloacina`/`.fid` produced by
/// [`crate::packaging::constructor_provider::package_constructor_provider`]) into
/// `dest` and, when `verify_keys` is non-empty, verify its Ed25519 `package.sig`
/// against those trusted keys. Returns the unpacked package directory (which
/// contains `package.toml`, the `constructor.json` sidecar, and the `.wasm`
/// component).
///
/// Fails closed at every stage:
///   * a structurally-hostile archive (path traversal, absolute path, symlink,
///     hardlink, size/ratio bomb, or no `package.toml`) is rejected by
///     [`fidius_core::package::unpack_package`];
///   * with `verify_keys` supplied, a missing or non-verifying signature — which
///     is exactly what tampering with the component or manifest after signing
///     produces (the digest no longer matches) — is a hard error via
///     [`fidius_host::package::verify_package`].
///
/// An empty `verify_keys` skips signature checking (the unsigned/dev flow); the
/// structural archive checks still apply.
pub fn unpack_provider_archive(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    verify_keys: &[ed25519_dalek::VerifyingKey],
) -> Result<std::path::PathBuf, LoaderError> {
    let archive = archive.as_ref();
    let dest = dest.as_ref();

    let pkg_dir = fidius_core::package::unpack_package(archive, dest).map_err(|e| {
        LoaderError::Validation {
            reason: format!("unpack provider package '{}': {e}", archive.display()),
        }
    })?;

    if !verify_keys.is_empty() {
        fidius_host::package::verify_package(&pkg_dir, verify_keys).map_err(|e| {
            LoaderError::Validation {
                reason: format!("verify provider signature for '{}': {e}", pkg_dir.display()),
            }
        })?;
    }

    Ok(pkg_dir)
}

/// Load a WASM **task** constructor from a packed provider archive and return it
/// as a runnable [`Task`].
///
/// The packaged analogue of [`load_task_constructor`]: it unpacks (and, with
/// `verify_keys`, verifies the signature of) the provider archive into `dest`,
/// then resolves `package_name` out of the unpacked tree exactly as the loose-dir
/// loader does (the unpacked package dir lives under `dest`, and fidius matches
/// the package by its `[package].name`, not its directory name). `config` binds
/// the constructor's per-instance configuration once at load.
///
/// Fails closed if the archive is hostile/unsigned-when-required, the package is
/// missing, the manifest is unreadable or not a `Task`, or the interface version
/// doesn't match the contract.
pub fn load_task_constructor_from_package<C: Serialize>(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    verify_keys: &[ed25519_dalek::VerifyingKey],
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Task>, LoaderError> {
    let dest = dest.as_ref();
    // Unpack (+ verify) first; this is the seam that rejects a tampered or
    // unsigned-but-required archive before any wasm is loaded.
    let _pkg_dir = unpack_provider_archive(archive, dest, verify_keys)?;
    // `dest` is the fidius search path; the package resolves by name out of the
    // freshly-unpacked `<name>-<version>/` directory it contains.
    load_task_constructor(dest, package_name, config, grants)
}

// ===========================================================================
// TRIGGER primitive (CLOACI-T-0824)
// ===========================================================================

/// Host-side scheduling metadata bound to a trigger constructor at load.
///
/// The WASM guest's `poll` decides *whether* to fire; the *cadence* and the
/// scheduler-routing metadata (poll interval, cron expression, target workflow,
/// concurrency) are host concerns the guest never sees. The constructor manifest
/// has no poll-interval field (it describes the component, not a deployment), so
/// these are bound here — the trigger analogue of "config binds once at load".
#[derive(Debug, Clone)]
pub struct TriggerBinding {
    /// How often the reconciler polls this trigger (ignored for cron triggers).
    pub poll_interval: Duration,
    /// Whether concurrent fires with the same context hash are allowed.
    pub allow_concurrent: bool,
    /// The workflow this trigger fires (the `#[trigger(on = "...")]` binding).
    pub workflow_name: String,
    /// Optional cron expression; `Some` routes to the cron scheduler instead of
    /// the poll-interval trigger registry.
    pub cron_expression: Option<String>,
}

impl Default for TriggerBinding {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(60),
            allow_concurrent: false,
            workflow_name: String::new(),
            cron_expression: None,
        }
    }
}

/// A loaded, configured WASM trigger constructor wrapped as a cloacina [`Trigger`].
///
/// Mirrors [`WasmTaskConstructor`]: holds the configured fidius handle behind an
/// [`Arc`] so the async `poll` bridge can hand a `'static + Send` clone to
/// `spawn_blocking`. The host-side [`TriggerBinding`] supplies the scheduling
/// metadata the `Trigger` trait exposes (`poll_interval`, `workflow_name`, …).
pub struct WasmTriggerConstructor {
    name: String,
    handle: Arc<fidius_host::PluginHandle>,
    binding: TriggerBinding,
}

impl std::fmt::Debug for WasmTriggerConstructor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTriggerConstructor")
            .field("name", &self.name)
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Trigger for WasmTriggerConstructor {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.binding.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.binding.allow_concurrent
    }

    fn cron_expression(&self) -> Option<String> {
        self.binding.cron_expression.clone()
    }

    fn workflow_name(&self) -> &str {
        &self.binding.workflow_name
    }

    /// The async↔sync bridge: serialize the (empty) invocation, hop into the
    /// blocking wasmtime `poll` on a `spawn_blocking` thread, and map the
    /// [`PollOutcome`] to a [`TriggerResult`]. A populated outcome `error`
    /// becomes a [`TriggerError::PollError`].
    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let name = self.name.clone();

        let inv_json = serde_json::to_string(&TriggerInvocation::default()).map_err(|e| {
            TriggerError::PollError {
                message: format!("trigger '{name}': serialize invocation: {e}"),
            }
        })?;

        let handle = self.handle.clone();
        let call_name = name.clone();
        let out_json: String = tokio::task::spawn_blocking(move || {
            handle.call_method::<_, String>(METHOD_POLL, &(inv_json,))
        })
        .await
        .map_err(|e| TriggerError::PollError {
            message: format!("trigger '{call_name}': poll join: {e}"),
        })?
        .map_err(|e| TriggerError::PollError {
            message: format!("trigger '{call_name}': poll FFI call: {e}"),
        })?;

        let outcome: PollOutcome =
            serde_json::from_str(&out_json).map_err(|e| TriggerError::PollError {
                message: format!("trigger '{name}': parse poll outcome: {e}"),
            })?;

        if let Some(err) = outcome.error {
            return Err(TriggerError::PollError {
                message: format!("trigger '{name}': {err}"),
            });
        }

        if !outcome.fire {
            return Ok(TriggerResult::Skip);
        }

        match outcome.context_json {
            None => Ok(TriggerResult::Fire(None)),
            Some(ctx_json) => {
                let ctx = Context::from_json(ctx_json).map_err(|e| TriggerError::PollError {
                    message: format!("trigger '{name}': rebuild fire context: {e}"),
                })?;
                Ok(TriggerResult::Fire(Some(ctx)))
            }
        }
    }
}

/// Load a WASM **trigger** constructor and return it as a runnable [`Trigger`].
///
/// `config` binds the constructor's per-instance WASM configuration once at load
/// (the guest's `configure` hook); `binding` supplies the host-side scheduling
/// metadata the `Trigger` trait exposes (poll interval / cron / target
/// workflow). Fails closed if the package is missing, the manifest is
/// unreadable, the primitive is not a `Trigger`, or the interface version
/// doesn't match the trigger contract.
pub fn load_trigger_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    binding: TriggerBinding,
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Trigger>, LoaderError> {
    let search_path = search_path.as_ref();

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_constructor_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Trigger {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Trigger",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TRIGGER_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares trigger-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TRIGGER_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_TriggerConstructor::TriggerConstructor_WASM_DESCRIPTOR,
            config,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(Arc::new(WasmTriggerConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
        binding,
    }))
}

// ===========================================================================
// Runtime registration — dispatch on primitive_kind (CLOACI-T-0824)
// ===========================================================================

/// Per-primitive binding the caller supplies to [`load_constructor`]. The variant
/// must match the loaded constructor's `primitive_kind` or the load fails closed.
pub enum ConstructorBinding {
    /// Register the loaded constructor as a [`Task`] under this namespace.
    Task {
        /// The namespace the task constructor is registered under (the same
        /// 4-tuple a macro-authored task would occupy).
        namespace: TaskNamespace,
    },
    /// Register the loaded constructor as a [`Trigger`] with this scheduling
    /// metadata. The trigger is keyed in the registry by the manifest `name`.
    Trigger(TriggerBinding),
}

/// Load a WASM constructor and register the configured primitive into `runtime`,
/// dispatching on the manifest's `primitive_kind`.
///
/// This is the seam that makes a loaded constructor a first-class participant in
/// workflows/schedules: after this returns, `runtime.get_task(ns)` /
/// `runtime.get_trigger(name)` hand out the configured constructor exactly as they
/// would a macro-authored one. The registered constructor clones a shared `Arc`
/// over the single configured fidius handle, so every instantiation dispatches
/// into the same sandboxed store.
///
/// Fails closed if the [`ConstructorBinding`] variant doesn't match the constructor's
/// declared `primitive_kind`, or on any underlying load error.
pub fn load_constructor<C: Serialize>(
    runtime: &Runtime,
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    binding: ConstructorBinding,
    grants: &ResolvedGrants,
) -> Result<(), LoaderError> {
    let search_path = search_path.as_ref();

    // Peek the manifest so a mismatched binding fails before we pay for a load.
    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;
    let manifest = read_constructor_manifest(&dir)?;

    match (&binding, manifest.primitive_kind) {
        (ConstructorBinding::Task { namespace }, PrimitiveKind::Task) => {
            let task = load_task_constructor(search_path, package_name, config, grants)?;
            let namespace = namespace.clone();
            runtime.register_task(namespace, move || task.clone());
            Ok(())
        }
        (ConstructorBinding::Trigger(tb), PrimitiveKind::Trigger) => {
            let name = manifest.name.clone();
            let trigger =
                load_trigger_constructor(search_path, package_name, config, tb.clone(), grants)?;
            runtime.register_trigger(name, move || trigger.clone());
            Ok(())
        }
        (_, kind) => Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, which does not match the supplied binding \
                 (accumulator/reactor registration is a CLOACI-T-0824 continuation)",
                manifest.name, kind
            ),
        }),
    }
}

// ===========================================================================
// ACCUMULATOR primitive (CLOACI-T-0828)
// ===========================================================================
//
// Unlike Task/Trigger, an accumulator is NOT a `Runtime` constructor: it is a
// stateful event sink (`Accumulator::process(Vec<u8>) -> Option<Output>`) driven
// by `accumulator_runtime`'s processor loop. The bridge below wires a configured
// WASM handle into that loop by implementing `Accumulator` directly.
//
// WRINKLE vs task/trigger: `Accumulator::process` is itself SYNCHRONOUS (the
// runtime calls it inline on its processor task and owns deserialization), so
// the bridge calls the blocking wasmtime `ingest` DIRECTLY — there is no async
// method to hang a `spawn_blocking` off. This is consistent with the native
// contract, whose `process` is sync CPU-bound work called sequentially.

/// Host-side re-declaration of the ACCUMULATOR-constructor interface (CLOACI-T-0828).
/// Same trait shape the guest implements; the fidius macro emits the matching
/// `AccumulatorConstructor_WASM_DESCRIPTOR` the loader links against. Single SYNC
/// method — the WASM analogue of `Accumulator::process`.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait AccumulatorConstructor: Send + Sync {
    /// `JSON(AccumulatorInvocation)` in -> `JSON(AccumulatorOutcome)` out. SYNC.
    fn ingest(&self, invocation_json: String) -> String;
}

/// A loaded, configured WASM accumulator constructor wrapped as a cloacina
/// [`Accumulator`].
///
/// Holds the configured fidius handle behind an [`Arc`]; each event's `process`
/// dispatches the sync `ingest` into the already-configured sandbox. Its
/// `Output` is the boundary's JSON **bytes** (`Vec<u8>`): the
/// `BoundarySender` then bincode-serializes that to `bincode(json_bytes)` —
/// exactly the canonical boundary frame the reactor's FFI bridge /
/// [`capture_fire_inputs`](crate::computation_graph::reactor) decode. Emitting a
/// `serde_json::Value` would NOT round-trip (bincode is not self-describing), so
/// the JSON-bytes shape is both correct and downstream-compatible.
pub struct WasmAccumulatorConstructor {
    name: String,
    handle: Arc<fidius_host::PluginHandle>,
}

impl WasmAccumulatorConstructor {
    /// The accumulator constructor's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for WasmAccumulatorConstructor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmAccumulatorConstructor")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl Accumulator for WasmAccumulatorConstructor {
    type Output = Vec<u8>;

    /// The sync bridge: turn the raw event bytes into `JSON(AccumulatorInvocation)`,
    /// call the blocking wasmtime `ingest`, and hand the guest's boundary JSON back
    /// as bytes. Errors (non-UTF-8 event, FFI failure, invalid boundary JSON, a
    /// populated outcome `error`) are logged and surfaced as `None` — exactly the
    /// native `process` contract, which returns `None` when an event yields no
    /// boundary.
    fn process(&mut self, event: Vec<u8>) -> Option<Vec<u8>> {
        let event_json = match String::from_utf8(event) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(name = %self.name, "accumulator constructor: event is not UTF-8 JSON: {e}");
                return None;
            }
        };

        let inv_json = match serde_json::to_string(&AccumulatorInvocation { event_json }) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(name = %self.name, "accumulator constructor: serialize invocation: {e}");
                return None;
            }
        };

        // `process` is synchronous, so the blocking wasmtime call is made
        // directly (no spawn_blocking — there is no async context here).
        let out_json: String = match self
            .handle
            .call_method::<_, String>(METHOD_INGEST, &(inv_json,))
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(name = %self.name, "accumulator constructor: ingest FFI call: {e}");
                return None;
            }
        };

        let outcome: AccumulatorOutcome = match serde_json::from_str(&out_json) {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(name = %self.name, "accumulator constructor: parse ingest outcome: {e}");
                return None;
            }
        };

        if let Some(err) = outcome.error {
            tracing::error!(name = %self.name, "accumulator constructor ingest error: {err}");
            return None;
        }

        match outcome.boundary_json {
            None => None,
            Some(boundary) => {
                // Validate the guest emitted well-formed JSON, then forward its
                // bytes — `BoundarySender` bincode-wraps them into the canonical
                // `bincode(json_bytes)` boundary frame.
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&boundary) {
                    tracing::error!(name = %self.name, "accumulator constructor: boundary is not valid JSON: {e}");
                    return None;
                }
                Some(boundary.into_bytes())
            }
        }
    }
}

/// Load a WASM **accumulator** constructor and return it as a runnable
/// [`Accumulator`].
///
/// The returned value is driven by
/// [`accumulator_runtime`](crate::computation_graph::accumulator::accumulator_runtime):
/// hand it (plus an `AccumulatorContext` carrying the `BoundarySender` to the
/// reactor) to that runtime and each socket/source event flows through the
/// configured WASM `ingest`. `config` binds the constructor's per-instance WASM
/// configuration once at load (the guest's `configure` hook).
///
/// Fails closed if the package is missing, the manifest is unreadable, the
/// primitive is not an `Accumulator`, or the interface version doesn't match the
/// accumulator contract.
pub fn load_accumulator_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmAccumulatorConstructor, LoaderError> {
    let search_path = search_path.as_ref();

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_constructor_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Accumulator {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Accumulator",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares accumulator-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_AccumulatorConstructor::AccumulatorConstructor_WASM_DESCRIPTOR,
            config,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(WasmAccumulatorConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
    })
}

// ===========================================================================
// REACTOR primitive (CLOACI-T-0828)
// ===========================================================================
//
// A reactor is NOT a callable in the runtime: the `Reactor` is a concrete struct
// whose firing decision is hardcoded (`WhenAny`/`WhenAll` over `DirtyFlags`) in
// its executor loop. To let a WASM `evaluate` make that decision, T-0828 added a
// pluggable [`ReactorFireDecider`] seam to `Reactor` (`with_evaluator`): when a
// decider is present the executor consults IT instead of the dirty-flag criteria.
// `WasmReactorConstructor` implements that seam, bridging the firing decision to
// the sync WASM `evaluate`.
//
// The decider method IS async (it is our own trait, not the runtime's sync
// `process`), so — like task/trigger — the blocking wasmtime `evaluate` runs on
// `spawn_blocking`.
//
// CLOACI-T-0830 (done): the reactor constructor is now threaded through the CG
// SCHEDULER's package-load path. `ReactorDeclaration` carries an optional
// `ReactorConstructorRef` (from/constructor/config); `scheduler.rs::load_reactor`
// resolves it via [`load_reactor_constructor_node`] (below) and installs the
// loaded `WasmReactorConstructor` on the live `Reactor` via `with_evaluator`. The
// `#[reactor(from = .., constructor = .., config = { .. })]` authoring form
// populates the ref (carried on `ReactorRegistration`), and the resolved decider
// is reused across reactor restarts. Threading the ref through the FFI
// `ReactorPackageMetadata` (Rust cdylib packaging) remains a documented follow-on.

/// Host-side re-declaration of the REACTOR-constructor interface (CLOACI-T-0828).
/// Same trait shape the guest implements; the fidius macro emits the matching
/// `ReactorConstructor_WASM_DESCRIPTOR` the loader links against. Single SYNC
/// method — the WASM analogue of the reactor's firing-criteria evaluation.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait ReactorConstructor: Send + Sync {
    /// `JSON(ReactorInvocation)` in -> `JSON(ReactorOutcome)` out. SYNC.
    fn evaluate(&self, invocation_json: String) -> String;
}

/// A loaded, configured WASM reactor constructor.
///
/// Holds the configured fidius handle behind an [`Arc`]. Exposes the firing
/// decision two ways:
///   * [`WasmReactorConstructor::evaluate`] — call the bridge directly with a
///     pre-serialized boundaries JSON (used by tests / a future scheduler), and
///   * the [`ReactorFireDecider`] impl — what a live [`Reactor`] consults via
///     `with_evaluator` so the WASM guest IS the firing criteria.
pub struct WasmReactorConstructor {
    name: String,
    handle: Arc<fidius_host::PluginHandle>,
}

impl WasmReactorConstructor {
    /// The reactor constructor's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Bridge the firing decision: serialize-free `boundaries_json` in,
    /// [`ReactorOutcome`] out. Hops into the blocking wasmtime `evaluate` on a
    /// `spawn_blocking` thread (mirrors the task/trigger bridge).
    pub async fn evaluate(&self, boundaries_json: String) -> Result<ReactorOutcome, LoaderError> {
        let name = self.name.clone();
        let inv_json =
            serde_json::to_string(&ReactorInvocation { boundaries_json }).map_err(|e| {
                LoaderError::Validation {
                    reason: format!("reactor constructor '{name}': serialize invocation: {e}"),
                }
            })?;

        let handle = self.handle.clone();
        let call_name = name.clone();
        let out_json: String = tokio::task::spawn_blocking(move || {
            handle.call_method::<_, String>(METHOD_EVALUATE, &(inv_json,))
        })
        .await
        .map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{call_name}': evaluate join: {e}"),
        })?
        .map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{call_name}': evaluate FFI call: {e}"),
        })?;

        serde_json::from_str::<ReactorOutcome>(&out_json).map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{name}': parse evaluate outcome: {e}"),
        })
    }
}

impl std::fmt::Debug for WasmReactorConstructor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmReactorConstructor")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ReactorFireDecider for WasmReactorConstructor {
    /// Serialize the reactor's current boundary cache to the `boundaries_json`
    /// envelope and ask the WASM guest whether to fire. A bridge error or a
    /// populated outcome `error` is logged and treated as "do not fire".
    async fn should_fire(&self, snapshot: &InputCache) -> bool {
        // Decode each cached boundary to a JSON value (the frames are
        // `bincode(json_bytes)`; fall back to raw JSON, then null) — the same
        // shape the reactor's fire log captures.
        let boundaries: std::collections::HashMap<String, serde_json::Value> =
            crate::computation_graph::reactor::capture_fire_inputs(snapshot);
        let boundaries_json = match serde_json::to_string(&boundaries) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(name = %self.name, "reactor constructor: serialize boundaries: {e}");
                return false;
            }
        };

        match self.evaluate(boundaries_json).await {
            Ok(outcome) => {
                if let Some(err) = outcome.error {
                    tracing::error!(name = %self.name, "reactor constructor evaluate error: {err}");
                    return false;
                }
                outcome.fire
            }
            Err(e) => {
                tracing::error!(name = %self.name, "reactor constructor evaluate failed: {e}");
                false
            }
        }
    }
}

/// Load a WASM **reactor** constructor and return its configured firing-decision
/// bridge.
///
/// Hand the returned [`WasmReactorConstructor`] to
/// [`Reactor::with_evaluator`](crate::computation_graph::reactor::Reactor::with_evaluator)
/// (as an `Arc<dyn ReactorFireDecider>`) so the running reactor consults the WASM
/// guest's `evaluate` for its firing decision. `config` binds the constructor's
/// per-instance WASM configuration once at load.
///
/// Fails closed if the package is missing, the manifest is unreadable, the
/// primitive is not a `Reactor`, or the interface version doesn't match the
/// reactor contract.
pub fn load_reactor_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmReactorConstructor, LoaderError> {
    let search_path = search_path.as_ref();

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_constructor_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Reactor {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Reactor",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != REACTOR_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares reactor-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, REACTOR_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_ReactorConstructor::ReactorConstructor_WASM_DESCRIPTOR,
            config,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(WasmReactorConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
    })
}

// ===========================================================================
// Consumer surface — `constructor!` workflow DAG node (CLOACI-T-0829)
// ===========================================================================
//
// The runtime half of the `constructor!(id = .., from = "..", constructor = "..",
// config = { .. }, dependencies = [..])` form a workflow author writes inside a
// `#[workflow]` module. The `#[workflow]` macro (cloacina-macros::workflow_attr)
// lowers each `constructor!` declaration into a call to [`load_constructor_node`],
// emitted in BOTH places a `#[task]` is: the workflow's DAG builder (so the node
// participates in topological planning + dependency edges) and a `TaskEntry`
// inventory submission (so `Runtime::get_task` resolves it for execution).
//
// ## `from` resolution seam
//
// `from = "name[@version]"` names a provider package by its fidius `[package].name`
// (the same name the loose-dir/`load_task_constructor` path matches on). It is
// resolved against a single PROVIDER SEARCH-PATH directory — a directory holding
// unpacked provider packages — chosen (highest precedence first) by:
//
//   1. the process-wide override set via [`set_provider_search_path`];
//   2. the `CLOACINA_PROVIDER_PATH` environment variable;
//   3. the `providers` directory (relative to the process CWD) as the default.
//
// This mirrors the embedded-first philosophy: no server, no registry service — a
// constructor provider is resolved from a directory on disk, exactly as
// [`load_task_constructor`] already does. An optional `@version` suffix is stripped
// (version pinning is advisory today; honoring it is a noted follow-on), and
// `constructor = "name"` selects WHICH constructor inside the provider — validated
// against the loaded `constructor.json`'s `name` so an author mismatch fails closed.

/// Process-wide override for the provider search path the `constructor!` consumer
/// form resolves `from = "..."` against. `None` falls back to
/// [`PROVIDER_PATH_ENV`] then [`DEFAULT_PROVIDER_DIR`]. Set it once at startup
/// (e.g. from a deployment's config) via [`set_provider_search_path`].
static PROVIDER_SEARCH_PATH: std::sync::RwLock<Option<std::path::PathBuf>> =
    std::sync::RwLock::new(None);

/// Environment variable naming the directory `constructor!(from = ...)` resolves
/// provider packages in (overridden by [`set_provider_search_path`]).
pub const PROVIDER_PATH_ENV: &str = "CLOACINA_PROVIDER_PATH";

/// Default provider search-path directory (relative to CWD) when neither the
/// process override nor [`PROVIDER_PATH_ENV`] is set.
pub const DEFAULT_PROVIDER_DIR: &str = "providers";

/// Set the process-wide provider search path used to resolve `constructor!`
/// `from = "..."` references. Takes precedence over [`PROVIDER_PATH_ENV`].
pub fn set_provider_search_path(path: impl Into<std::path::PathBuf>) {
    *PROVIDER_SEARCH_PATH.write().unwrap() = Some(path.into());
}

/// Clear the process-wide provider search-path override (falls back to env/default).
pub fn clear_provider_search_path() {
    *PROVIDER_SEARCH_PATH.write().unwrap() = None;
}

/// The directory `constructor!(from = ...)` resolves provider packages in:
/// the [`set_provider_search_path`] override, else [`PROVIDER_PATH_ENV`], else
/// [`DEFAULT_PROVIDER_DIR`].
pub fn provider_search_path() -> std::path::PathBuf {
    if let Some(p) = PROVIDER_SEARCH_PATH.read().unwrap().clone() {
        return p;
    }
    if let Ok(p) = std::env::var(PROVIDER_PATH_ENV) {
        if !p.is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    std::path::PathBuf::from(DEFAULT_PROVIDER_DIR)
}

/// Strip an optional `@version` suffix from a `from = "name[@version]"` reference,
/// yielding the fidius `[package].name` the loader matches on.
fn provider_package_name(from: &str) -> &str {
    match from.split_once('@') {
        Some((name, _ver)) => name,
        None => from,
    }
}

/// A workflow DAG node backed by a packaged WASM task constructor — the runtime
/// representation of a `constructor!(...)` declaration (CLOACI-T-0829).
///
/// Wraps the `Arc<dyn Task>` [`load_task_constructor`] returns, overriding its
/// `id` (the DAG node id the workflow author chose, which other tasks depend on —
/// distinct from the constructor's own `constructor.json` name) and its
/// `dependencies` (the workflow-namespaced deps the `constructor!` declared).
/// Everything else delegates to the loaded constructor.
pub struct ConstructorNode {
    id: String,
    inner: Arc<dyn Task>,
    dependencies: Vec<TaskNamespace>,
}

impl std::fmt::Debug for ConstructorNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructorNode")
            .field("id", &self.id)
            .field("dependencies", &self.dependencies)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Task for ConstructorNode {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        self.inner.execute(context).await
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }

    fn retry_policy(&self) -> crate::retry::RetryPolicy {
        self.inner.retry_policy()
    }

    fn trigger_rules(&self) -> serde_json::Value {
        self.inner.trigger_rules()
    }

    fn code_fingerprint(&self) -> Option<String> {
        self.inner.code_fingerprint()
    }

    fn requires_handle(&self) -> bool {
        self.inner.requires_handle()
    }
}

/// A single `#[config]` value resolved from a `constructor!(config = { … })`
/// literal, typed per the constructor manifest's declared field type (CLOACI-T-0829).
///
/// fidius binds config via bincode, which is positional + width-sensitive (an `i64`
/// field is 8 bytes, an `i32` field 4). Serializing a `serde_json::Value` directly
/// would emit the wrong bytes (an enum tag, the wrong width), so each kwarg value is
/// coerced into the concrete variant matching the guest's declared field type and
/// serialized with the matching serde method — byte-identical to what the guest's
/// generated config struct decodes.
enum TypedConfigValue {
    Str(String),
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl Serialize for TypedConfigValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            TypedConfigValue::Str(v) => s.serialize_str(v),
            TypedConfigValue::Bool(v) => s.serialize_bool(*v),
            TypedConfigValue::I8(v) => s.serialize_i8(*v),
            TypedConfigValue::I16(v) => s.serialize_i16(*v),
            TypedConfigValue::I32(v) => s.serialize_i32(*v),
            TypedConfigValue::I64(v) => s.serialize_i64(*v),
            TypedConfigValue::U8(v) => s.serialize_u8(*v),
            TypedConfigValue::U16(v) => s.serialize_u16(*v),
            TypedConfigValue::U32(v) => s.serialize_u32(*v),
            TypedConfigValue::U64(v) => s.serialize_u64(*v),
            TypedConfigValue::F32(v) => s.serialize_f32(*v),
            TypedConfigValue::F64(v) => s.serialize_f64(*v),
        }
    }
}

/// The reordered `#[config]` values in the guest's DECLARATION order, serialized as
/// a bincode TUPLE (no length prefix, no field names) — byte-identical to the
/// guest's generated config struct. This is what crosses the sandbox to the
/// `configure` hook once at load.
struct OrderedConfig(Vec<TypedConfigValue>);

impl Serialize for OrderedConfig {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTuple;
        let mut tup = s.serialize_tuple(self.0.len())?;
        for v in &self.0 {
            tup.serialize_element(v)?;
        }
        tup.end()
    }
}

/// Coerce one kwarg JSON value into the [`TypedConfigValue`] matching the declared
/// Rust type `ty` (the manifest's `ConfigField::ty`). Returns a clear, key-named
/// error if the literal's JSON kind doesn't fit the declared type.
fn coerce_config_value(
    key: &str,
    ty: &str,
    value: &serde_json::Value,
) -> Result<TypedConfigValue, String> {
    let wrong =
        |want: &str| format!("config field '{key}' expects {want} (declared `{ty}`), got {value}");
    match ty {
        "String" | "str" => value
            .as_str()
            .map(|s| TypedConfigValue::Str(s.to_string()))
            .ok_or_else(|| wrong("a string")),
        "bool" => value
            .as_bool()
            .map(TypedConfigValue::Bool)
            .ok_or_else(|| wrong("a boolean")),
        "i8" => int_in_range::<i8>(value)
            .map(TypedConfigValue::I8)
            .ok_or_else(|| wrong("an i8")),
        "i16" => int_in_range::<i16>(value)
            .map(TypedConfigValue::I16)
            .ok_or_else(|| wrong("an i16")),
        "i32" => int_in_range::<i32>(value)
            .map(TypedConfigValue::I32)
            .ok_or_else(|| wrong("an i32")),
        // `isize` is serialized as i64 by serde/bincode, so it shares this arm.
        "i64" | "isize" => value
            .as_i64()
            .map(TypedConfigValue::I64)
            .ok_or_else(|| wrong("an i64")),
        "u8" => uint_in_range::<u8>(value)
            .map(TypedConfigValue::U8)
            .ok_or_else(|| wrong("a u8")),
        "u16" => uint_in_range::<u16>(value)
            .map(TypedConfigValue::U16)
            .ok_or_else(|| wrong("a u16")),
        "u32" => uint_in_range::<u32>(value)
            .map(TypedConfigValue::U32)
            .ok_or_else(|| wrong("a u32")),
        // `usize` is serialized as u64 by serde/bincode, so it shares this arm.
        "u64" | "usize" => value
            .as_u64()
            .map(TypedConfigValue::U64)
            .ok_or_else(|| wrong("a u64")),
        "f32" => value
            .as_f64()
            .map(|f| TypedConfigValue::F32(f as f32))
            .ok_or_else(|| wrong("a number")),
        "f64" => value
            .as_f64()
            .map(TypedConfigValue::F64)
            .ok_or_else(|| wrong("a number")),
        other => Err(format!(
            "config field '{key}' has unsupported declared type `{other}`; \
             constructor config supports string/bool/integer/float literals"
        )),
    }
}

/// Narrow a JSON integer into a signed target width, rejecting out-of-range.
fn int_in_range<T: TryFrom<i64>>(value: &serde_json::Value) -> Option<T> {
    value.as_i64().and_then(|v| T::try_from(v).ok())
}

/// Narrow a JSON integer into an unsigned target width, rejecting out-of-range.
fn uint_in_range<T: TryFrom<u64>>(value: &serde_json::Value) -> Option<T> {
    value.as_u64().and_then(|v| T::try_from(v).ok())
}

/// Bind the author's `config = { name = value }` kwargs BY NAME against the
/// constructor's declared `#[config]` schema (CLOACI-T-0829), reordering them into
/// the guest's declaration order and coercing each to its declared type.
///
/// Enforces true kwarg semantics, failing closed with a key-named error on:
///   * an UNKNOWN config key (not a declared `#[config]` field),
///   * a DUPLICATE config key,
///   * a MISSING declared field, or
///   * a value whose JSON kind doesn't fit the declared type.
fn bind_config_by_name(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    manifest: &ConstructorManifest,
    mut author: Vec<(String, serde_json::Value)>,
) -> Result<OrderedConfig, LoaderError> {
    let ctx = |reason: String| {
        LoaderError::Validation {
        reason: format!(
            "constructor node '{node_id}' (from = '{from}', constructor = '{constructor_name}'): {reason}"
        ),
    }
    };
    let declared: Vec<&str> = manifest
        .config_fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();

    // Reject unknown keys up front (names the offending key, lists the valid set).
    for (k, _) in &author {
        if !declared.contains(&k.as_str()) {
            return Err(ctx(format!(
                "config key '{k}' is not a #[config] field of constructor '{}'. \
                 Declared config fields: [{}]",
                manifest.name,
                declared.join(", ")
            )));
        }
    }

    // Pull each declared field's value in DECLARATION order; absence is an error.
    let mut ordered = Vec::with_capacity(manifest.config_fields.len());
    for field in &manifest.config_fields {
        let pos = author.iter().position(|(k, _)| k == &field.name);
        let value = match pos {
            Some(i) => author.remove(i).1,
            None => {
                return Err(ctx(format!(
                    "missing required config field '{}' for constructor '{}'",
                    field.name, manifest.name
                )))
            }
        };
        let typed = coerce_config_value(&field.name, &field.ty, &value).map_err(ctx)?;
        ordered.push(typed);
    }

    // Anything left over (unknown keys were already rejected) is a duplicate key.
    if let Some((dup, _)) = author.into_iter().next() {
        return Err(ctx(format!("duplicate config key '{dup}'")));
    }

    Ok(OrderedConfig(ordered))
}

/// Load-time capability lint (REQ-1.3.1 / [[CLOACI-S-0014]]): read the package's
/// declared `[wasm].capabilities` (the author's stated intent) and emit a warning
/// for each capability the tenant's `grants` don't cover. Advisory only —
/// best-effort (a manifest we can't read is silently skipped); enforcement still
/// fails closed at runtime. This turns "this constructor wants `http` you didn't
/// grant" from a mystery runtime denial into a load-time heads-up.
fn lint_constructor_grants(node_id: &str, from: &str, dir: &Path, grants: &GrantSpec) {
    let Ok(pkg) = fidius_core::package::load_manifest_untyped(dir) else {
        return;
    };
    let Some(wasm) = pkg.wasm.as_ref() else {
        return;
    };
    for warning in lint_unmet_intents(&wasm.capabilities, grants) {
        tracing::warn!(node = %node_id, from = %from, "{warning}");
    }
}

/// Resolve + load a packaged constructor as a workflow DAG node (the runtime half
/// of the `constructor!(...)` consumer form).
///
/// Resolves `from` against the [`provider_search_path`], reads the resolved
/// constructor's manifest to learn its `#[config]` schema, binds the author's
/// `config = { name = value }` kwargs BY NAME ([`bind_config_by_name`]), loads the
/// named `constructor` via the packaged task-constructor loader (binding the
/// reordered config once), validates that the provider actually carries the
/// requested constructor, and wraps it as a [`ConstructorNode`] carrying the
/// author-chosen `node_id` + the workflow-namespaced `dependencies`.
///
/// `config` is the author's `config = { … }` entries as `(name, value)` pairs in
/// WRITTEN order. fidius binds config via bincode (positional, NOT self-describing),
/// so before loading we **reorder** the values into the constructor's `#[config]`
/// DECLARATION order — read from the manifest's `config_fields` — and coerce each to
/// its declared type. This gives `config = { name = value }` true kwarg semantics:
/// the field NAMES bind the values, not their written order.
///
/// Fails closed (a clear [`LoaderError`]) if the provider is missing, the manifest
/// is unreadable / not a `Task`, the interface version mismatches, a config kwarg is
/// unknown / duplicated / missing / mistyped, or the resolved constructor name does
/// not match `constructor_name`.
pub fn load_constructor_node(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
) -> Result<Arc<dyn Task>, LoaderError> {
    let search_path = provider_search_path();
    let package_name = provider_package_name(from);

    // Translate the tenant's grants into the fidius caps + egress policy. Fail
    // closed: a malformed grant aborts the load (it never silently widens access).
    let resolved = translate(&grants).map_err(|e| LoaderError::Validation {
        reason: format!("constructor node '{node_id}' (from = '{from}'): {e}"),
    })?;

    // Peek the manifest to learn the constructor's #[config] schema, so we can bind
    // `config = { name = value }` by NAME (reorder into declaration order) before
    // the positional bincode load.
    let host = PluginHost::builder()
        .search_path(&search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!(
                "resolve constructor node '{node_id}' (from = '{from}', constructor = \
                 '{constructor_name}'): locate provider package '{package_name}' in \
                 provider search path '{}': {e}",
                search_path.display()
            ),
        })?;
    let manifest = read_constructor_manifest(&dir)?;

    // Load-time capability lint (REQ-1.3.1, advisory): warn when the package
    // declares an intent to use a capability the tenant didn't grant — it will be
    // denied at runtime, so surface it now rather than as a mystery failure later.
    lint_constructor_grants(node_id, from, &dir, &grants);

    let ordered_config = bind_config_by_name(node_id, from, constructor_name, &manifest, config)?;

    let task = load_task_constructor(&search_path, package_name, &ordered_config, &resolved)
        .map_err(|e| LoaderError::Validation {
            reason: format!(
                "resolve constructor node '{node_id}' (from = '{from}', constructor = \
                     '{constructor_name}') in provider search path '{}': {e}",
                search_path.display()
            ),
        })?;

    if task.id() != constructor_name {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor node '{node_id}': provider '{from}' (package '{package_name}') \
                 carries constructor '{}', but the workflow asked for constructor = \
                 '{constructor_name}'",
                task.id()
            ),
        });
    }

    Ok(Arc::new(ConstructorNode {
        id: node_id.to_string(),
        inner: task,
        dependencies,
    }))
}

// ===========================================================================
// Reactor-constructor consumer surface — CG scheduler load path (CLOACI-T-0830)
// ===========================================================================
//
// A reactor is NOT a `Runtime` constructor (task/trigger) or a DAG node — it is
// the CG SCHEDULER's firing engine. So a packaged reactor constructor is consumed
// differently from the `constructor!(...)` node form: instead of producing an
// `Arc<dyn Task>` for the workflow DAG, it produces an `Arc<dyn ReactorFireDecider>`
// that the scheduler installs on the live `Reactor` via `with_evaluator`.
//
// This is the runtime half of the `#[reactor(from = .., constructor = .., config =
// { .. })]` authoring form: the CG scheduler (`load_reactor`) calls this to resolve
// the declaration's `ReactorConstructorRef`. It reuses the SAME T-0829 provider
// resolution (`provider_search_path`/`provider_package_name`) and name-keyed config
// binding (`bind_config_by_name`) as the task/trigger node form, so a reactor
// constructor is authored and resolved exactly like the others.

/// Resolve + load a packaged WASM **reactor** constructor as a firing decider for
/// the CG scheduler (the runtime half of the `#[reactor(from = .., constructor =
/// .., config = { .. })]` authoring form — CLOACI-T-0830).
///
/// Resolves `from` against the [`provider_search_path`], reads the resolved
/// constructor's manifest to learn its `#[config]` schema, binds the author's
/// `config = { name = value }` kwargs BY NAME ([`bind_config_by_name`], reordered
/// into the guest's declaration order), loads the named reactor constructor via
/// [`load_reactor_constructor`] (binding the reordered config once), validates the
/// resolved constructor's name matches `constructor_name`, and returns it as an
/// `Arc<dyn ReactorFireDecider>` ready for
/// [`Reactor::with_evaluator`](crate::computation_graph::reactor::Reactor::with_evaluator).
///
/// `config` is the author's `config = { … }` entries as `(name, value)` pairs in
/// WRITTEN order; the reorder/coerce is identical to [`load_constructor_node`] (the
/// task/trigger node form) so config binds by NAME, not written order.
///
/// Fails closed (a clear [`LoaderError`]) if the provider is missing, the manifest
/// is unreadable / not a `Reactor`, the interface version mismatches, a config kwarg
/// is unknown / duplicated / missing / mistyped, or the resolved constructor name
/// does not match `constructor_name`.
pub fn load_reactor_constructor_node(
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError> {
    let search_path = provider_search_path();
    let package_name = provider_package_name(from);

    // Translate the tenant's grants (fail closed on a malformed grant) before any
    // load work, so an invalid grant never silently widens access.
    let resolved = translate(&grants).map_err(|e| LoaderError::Validation {
        reason: format!("reactor constructor '{constructor_name}' (from = '{from}'): {e}"),
    })?;

    // Peek the manifest to learn the constructor's #[config] schema, so we can bind
    // `config = { name = value }` by NAME before the positional bincode load.
    let host = PluginHost::builder()
        .search_path(&search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!(
                "resolve reactor constructor (from = '{from}', constructor = '{constructor_name}'): \
                 locate provider package '{package_name}' in provider search path '{}': {e}",
                search_path.display()
            ),
        })?;
    let manifest = read_constructor_manifest(&dir)?;

    // Load-time capability lint (REQ-1.3.1, advisory).
    lint_constructor_grants(constructor_name, from, &dir, &grants);

    // Reuse the T-0829 name-keyed config binding (node_id = the constructor name,
    // since a reactor has no separate DAG node id).
    let ordered_config =
        bind_config_by_name(constructor_name, from, constructor_name, &manifest, config)?;

    let reactor_constructor =
        load_reactor_constructor(&search_path, package_name, &ordered_config, &resolved).map_err(
            |e| LoaderError::Validation {
                reason: format!(
                    "resolve reactor constructor (from = '{from}', constructor = \
                     '{constructor_name}') in provider search path '{}': {e}",
                    search_path.display()
                ),
            },
        )?;

    if reactor_constructor.name() != constructor_name {
        return Err(LoaderError::Validation {
            reason: format!(
                "reactor constructor: provider '{from}' (package '{package_name}') carries \
                 constructor '{}', but the reactor declared constructor = '{constructor_name}'",
                reactor_constructor.name()
            ),
        });
    }

    Ok(Arc::new(reactor_constructor) as Arc<dyn ReactorFireDecider>)
}
