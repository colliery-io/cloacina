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

//! WASM operator loader + primitive adapters (CLOACI-I-0132 / T-0823, T-0824).
//!
//! Loads a WASM **operator** package (a `.wasm` component + a sidecar
//! `operator.json` manifest) and wraps the configured fidius handle as the
//! cloacina primitive the manifest's `primitive_kind` names — a
//! [`crate::task::Task`] (T-0823) or a [`crate::trigger::Trigger`] (T-0824) —
//! that the existing async executor / scheduler can run unchanged, then
//! registers it into a [`Runtime`] so it participates in workflows/schedules
//! exactly like a macro-authored one.
//!
//! Gated behind the default-OFF `operators-wasm` feature: enabling it turns on
//! `fidius-host`'s `wasm` feature (→ wasmtime/cranelift) and pulls
//! `fidius-macro` + `cloacina-operator-contract`. The default cloacina build
//! pulls none of this (verified via `cargo tree`).
//!
//! ## Flow (per primitive)
//!
//! 1. Read the package's `operator.json` into an [`OperatorManifest`].
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
//! [`load_operator`] reads the manifest and dispatches on `primitive_kind`,
//! registering the configured primitive into a [`Runtime`] (Task →
//! [`Runtime::register_task`], Trigger → [`Runtime::register_trigger`]). The
//! registered constructor hands out `Arc` clones that share the one configured
//! fidius handle, so every `get_task`/`get_trigger` call dispatches into the
//! same sandboxed instance.
//!
//! ## Continuation (CLOACI-T-0824 follow-up)
//!
//! The ACCUMULATOR `ingest` / REACTOR `evaluate` bridges have their sync
//! contract traits ([`AccumulatorOperator`] / [`ReactorOperator`]) and wire
//! types defined here + in the contract crate, and their adapter shape is
//! sketched ([`WasmAccumulatorOperator`] / [`WasmReactorOperator`]); a full
//! event-loop / reactor-firing impl + fixtures is a noted continuation because
//! those primitives are not simple `Runtime` constructors (the accumulator has
//! no `Runtime` registry; a reactor registers a `ReactorRegistration`
//! descriptor, not a callable). The `#[operator]` authoring macro is T-0826.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;

use cloacina_operator_contract::{
    OperatorManifest, PollOutcome, PrimitiveKind, TaskInvocation, TaskOutcome, TriggerInvocation,
    METHOD_EXECUTE, METHOD_POLL, TASK_OPERATOR_INTERFACE_VERSION,
    TRIGGER_OPERATOR_INTERFACE_VERSION,
};
use fidius_host::PluginHost;

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

/// Host-side re-declaration of the TRIGGER-operator interface (CLOACI-T-0824).
/// Same trait shape the guest implements; the fidius macro emits the matching
/// `TriggerOperator_WASM_DESCRIPTOR` (in `__fidius_TriggerOperator`) the loader
/// links the component against. Single SYNC method — the WASM analogue of the
/// async `cloacina_workflow::Trigger::poll`.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait TriggerOperator: Send + Sync {
    /// `JSON(TriggerInvocation)` in -> `JSON(PollOutcome)` out. SYNC.
    fn poll(&self, invocation_json: String) -> String;
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

// ===========================================================================
// TRIGGER primitive (CLOACI-T-0824)
// ===========================================================================

/// Host-side scheduling metadata bound to a trigger operator at load.
///
/// The WASM guest's `poll` decides *whether* to fire; the *cadence* and the
/// scheduler-routing metadata (poll interval, cron expression, target workflow,
/// concurrency) are host concerns the guest never sees. The operator manifest
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

/// A loaded, configured WASM trigger operator wrapped as a cloacina [`Trigger`].
///
/// Mirrors [`WasmTaskOperator`]: holds the configured fidius handle behind an
/// [`Arc`] so the async `poll` bridge can hand a `'static + Send` clone to
/// `spawn_blocking`. The host-side [`TriggerBinding`] supplies the scheduling
/// metadata the `Trigger` trait exposes (`poll_interval`, `workflow_name`, …).
pub struct WasmTriggerOperator {
    name: String,
    handle: Arc<fidius_host::PluginHandle>,
    binding: TriggerBinding,
}

impl std::fmt::Debug for WasmTriggerOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTriggerOperator")
            .field("name", &self.name)
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Trigger for WasmTriggerOperator {
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

/// Load a WASM **trigger** operator and return it as a runnable [`Trigger`].
///
/// `config` binds the operator's per-instance WASM configuration once at load
/// (the guest's `configure` hook); `binding` supplies the host-side scheduling
/// metadata the `Trigger` trait exposes (poll interval / cron / target
/// workflow). Fails closed if the package is missing, the manifest is
/// unreadable, the primitive is not a `Trigger`, or the interface version
/// doesn't match the trigger contract.
pub fn load_trigger_operator<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    binding: TriggerBinding,
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
            reason: format!("locate wasm operator package '{package_name}': {e}"),
        })?;

    let manifest = read_operator_manifest(&dir)?;

    if manifest.primitive_kind != PrimitiveKind::Trigger {
        return Err(LoaderError::Validation {
            reason: format!(
                "operator '{}' is {:?}, not Trigger",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TRIGGER_OPERATOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "operator '{}' declares trigger-operator interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TRIGGER_OPERATOR_INTERFACE_VERSION
            ),
        });
    }

    let handle = host
        .load_wasm_configured(
            package_name,
            &__fidius_TriggerOperator::TriggerOperator_WASM_DESCRIPTOR,
            config,
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured: {e}"),
        })?;

    Ok(Arc::new(WasmTriggerOperator {
        name: manifest.name,
        handle: Arc::new(handle),
        binding,
    }))
}

// ===========================================================================
// Runtime registration — dispatch on primitive_kind (CLOACI-T-0824)
// ===========================================================================

/// Per-primitive binding the caller supplies to [`load_operator`]. The variant
/// must match the loaded operator's `primitive_kind` or the load fails closed.
pub enum OperatorBinding {
    /// Register the loaded operator as a [`Task`] under this namespace.
    Task {
        /// The namespace the task constructor is registered under (the same
        /// 4-tuple a macro-authored task would occupy).
        namespace: TaskNamespace,
    },
    /// Register the loaded operator as a [`Trigger`] with this scheduling
    /// metadata. The trigger is keyed in the registry by the manifest `name`.
    Trigger(TriggerBinding),
}

/// Load a WASM operator and register the configured primitive into `runtime`,
/// dispatching on the manifest's `primitive_kind`.
///
/// This is the seam that makes a loaded operator a first-class participant in
/// workflows/schedules: after this returns, `runtime.get_task(ns)` /
/// `runtime.get_trigger(name)` hand out the configured operator exactly as they
/// would a macro-authored one. The registered constructor clones a shared `Arc`
/// over the single configured fidius handle, so every instantiation dispatches
/// into the same sandboxed store.
///
/// Fails closed if the [`OperatorBinding`] variant doesn't match the operator's
/// declared `primitive_kind`, or on any underlying load error.
pub fn load_operator<C: Serialize>(
    runtime: &Runtime,
    search_path: impl AsRef<Path>,
    package_name: &str,
    config: &C,
    binding: OperatorBinding,
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
            reason: format!("locate wasm operator package '{package_name}': {e}"),
        })?;
    let manifest = read_operator_manifest(&dir)?;

    match (&binding, manifest.primitive_kind) {
        (OperatorBinding::Task { namespace }, PrimitiveKind::Task) => {
            let task = load_task_operator(search_path, package_name, config)?;
            let namespace = namespace.clone();
            runtime.register_task(namespace, move || task.clone());
            Ok(())
        }
        (OperatorBinding::Trigger(tb), PrimitiveKind::Trigger) => {
            let name = manifest.name.clone();
            let trigger = load_trigger_operator(search_path, package_name, config, tb.clone())?;
            runtime.register_trigger(name, move || trigger.clone());
            Ok(())
        }
        (_, kind) => Err(LoaderError::Validation {
            reason: format!(
                "operator '{}' is {:?}, which does not match the supplied binding \
                 (accumulator/reactor registration is a CLOACI-T-0824 continuation)",
                manifest.name, kind
            ),
        }),
    }
}

// ===========================================================================
// ACCUMULATOR + REACTOR primitives — sketched continuation (CLOACI-T-0824)
// ===========================================================================
//
// The contract traits + wire types are defined (here and in the contract
// crate). The host bridges below mirror the task/trigger pattern but are left
// as a clearly-noted continuation: unlike Task/Trigger, neither primitive is a
// plain `Runtime` constructor.
//
//   * ACCUMULATOR — `cloacina::computation_graph::accumulator::Accumulator` is a
//     stateful `&mut self` event sink (`process(Vec<u8>) -> Option<Output>`)
//     driven by an async runtime loop (`accumulator_runtime`), NOT a `Runtime`
//     registry entry. A `WasmAccumulatorOperator` would own the configured
//     handle and, per event, `spawn_blocking` the sync `ingest`
//     (`JSON(AccumulatorInvocation)` -> `JSON(AccumulatorOutcome)`) and forward
//     any `boundary_json` to the reactor via the `BoundarySender`. Wiring it in
//     means standing up that event loop against a loaded handle.
//
//   * REACTOR — the runtime represents a reactor as a `ReactorRegistration`
//     *descriptor* (name + accumulator set + reaction mode), not a callable;
//     firing is evaluated by the CG scheduler. A `WasmReactorOperator` would
//     bridge the firing *decision* (`JSON(ReactorInvocation)` ->
//     `JSON(ReactorOutcome)`), but registering it requires threading a callable
//     evaluator through the scheduler, which is beyond this task's surface.
//
// The sync contract traits are declared host-side so the descriptors exist and
// the shapes are validated by the compiler; the adapters are intentionally not
// yet implemented.

/// Host-side re-declaration of the ACCUMULATOR-operator interface (continuation).
/// `JSON(AccumulatorInvocation)` in -> `JSON(AccumulatorOutcome)` out. SYNC.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait AccumulatorOperator: Send + Sync {
    /// Ingest one event, optionally producing a boundary for the reactor.
    fn ingest(&self, invocation_json: String) -> String;
}

/// Host-side re-declaration of the REACTOR-operator interface (continuation).
/// `JSON(ReactorInvocation)` in -> `JSON(ReactorOutcome)` out. SYNC.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait ReactorOperator: Send + Sync {
    /// Evaluate firing criteria over the held boundaries.
    fn evaluate(&self, invocation_json: String) -> String;
}
