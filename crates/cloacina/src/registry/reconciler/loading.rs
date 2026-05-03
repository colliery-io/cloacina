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

//! Package loading, unloading, and task/workflow registration.

use tracing::{debug, error, info, warn};

use super::{PackageState, RegistryReconciler};
use crate::registry::error::RegistryError;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::TaskNamespace;
use crate::Runtime;
use std::sync::Arc;

impl RegistryReconciler {
    /// Load a package into the global registries.
    ///
    /// Since the compiler service (CLOACI-I-0097) owns all `cargo build`
    /// invocations, the reconciler no longer compiles anything. It:
    /// 1. Fetches the source archive + prebuilt cdylib (`compiled_data`)
    ///    from the workflow registry — `get_workflow` only returns
    ///    packages in `build_status = 'success'`.
    /// 2. Unpacks the source for manifest + (Python) task extraction.
    /// 3. Dispatches by language: Rust hands `compiled_data` to fidius FFI,
    ///    Python imports from source.
    pub(super) async fn load_package(
        &self,
        metadata: WorkflowMetadata,
    ) -> Result<(), RegistryError> {
        debug!(
            "Loading package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Get the package archive data from the registry
        let loaded_workflow = self
            .registry
            .get_workflow(&metadata.package_name, &metadata.version)
            .await?
            .ok_or_else(|| RegistryError::PackageNotFound {
                package_name: metadata.package_name.clone(),
                version: metadata.version.clone(),
            })?;

        // --- Step 1: write archive to a temp file ---
        let work_dir = tempfile::TempDir::new().map_err(|e| RegistryError::RegistrationFailed {
            message: format!("Failed to create temp dir: {}", e),
        })?;

        let archive_path = work_dir.path().join(format!(
            "{}-{}.cloacina",
            metadata.package_name, metadata.version
        ));

        tokio::fs::write(&archive_path, &loaded_workflow.package_data)
            .await
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to write archive to temp file: {}", e),
            })?;

        // --- Step 2: unpack archive ---
        let extract_dir = work_dir.path().join("source");
        tokio::fs::create_dir_all(&extract_dir).await.map_err(|e| {
            RegistryError::RegistrationFailed {
                message: format!("Failed to create extract dir: {}", e),
            }
        })?;

        let archive_path_clone = archive_path.clone();
        let extract_dir_clone = extract_dir.clone();
        let source_dir = tokio::task::spawn_blocking(move || {
            fidius_core::package::unpack_package(&archive_path_clone, &extract_dir_clone).map_err(
                |e| RegistryError::RegistrationFailed {
                    message: format!("Failed to unpack source archive: {}", e),
                },
            )
        })
        .await
        .map_err(|e| RegistryError::RegistrationFailed {
            message: format!("spawn_blocking failed during unpack: {}", e),
        })??;

        // --- Step 3: load manifest and validate ---
        // T-E / I-0102: `#[serde(deny_unknown_fields)]` on CloacinaMetadata
        // makes legacy `package_type` and `[[triggers]]` keys hard-error at
        // deserialization. Wrap the deserializer error with a friendlier
        // migration hint so users see what to change rather than the raw
        // "unknown field" message.
        let source_dir_clone = source_dir.clone();
        let pkg_name_for_err = metadata.package_name.clone();
        let cloacina_manifest = tokio::task::spawn_blocking(move || {
            fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
                &source_dir_clone,
            )
            .map_err(|e| {
                let raw = e.to_string();
                let migration_hint = if raw.contains("package_type") {
                    " — `package_type` was removed in CLOACI-I-0102; primitives are now \
                     self-declared via the unified `cloacina::package!()` shell macro and \
                     per-primitive macros (`#[workflow]`, `#[reactor]`, `#[trigger]`, \
                     `#[computation_graph]`)"
                } else if raw.contains("triggers") {
                    " — `[[triggers]]` in package.toml was removed in CLOACI-I-0102; declare \
                     workflow → trigger subscriptions via `#[workflow(triggers = [...])]` on \
                     the workflow module instead"
                } else {
                    ""
                };
                RegistryError::RegistrationFailed {
                    message: format!(
                        "Failed to load package.toml for {}: {}{}",
                        pkg_name_for_err, raw, migration_hint
                    ),
                }
            })
        })
        .await
        .map_err(|e| RegistryError::RegistrationFailed {
            message: format!("spawn_blocking failed during manifest load: {}", e),
        })??;

        debug!(
            "Package manifest loaded: {} v{} language={}",
            cloacina_manifest.package.name,
            cloacina_manifest.package.version,
            cloacina_manifest.metadata.language
        );

        // T-E / I-0102: deprecation warnings removed; `[[triggers]]` and
        // `package_type` are now hard-errored at deserialization via
        // `#[serde(deny_unknown_fields)]` on `CloacinaMetadata`. The
        // friendly migration message is wrapped at the manifest-load
        // boundary below.

        // --- Step 4, 5, 6: language-specific loading ---
        //
        // `compiled_data` is populated by the compiler service for Rust / mixed
        // packages. Hold onto it here so the computation-graph step below can
        // reuse the same bytes without another DB round-trip.
        let rust_cdylib_bytes = loaded_workflow.compiled_data.clone();

        // T-0554 Phase 2: per-language reactor_names tracking. The
        // earlier pre/post inventory-diff approach didn't work for
        // independently-compiled cdylibs (each fixture/example crate
        // has its own `[workspace]`, so `cloacina-workflow-plugin` is
        // a separate compilation with distinct linker symbols and
        // `inventory::iter` doesn't see entries submitted by the
        // dlopen'd cdylib). For Rust packages we use the FFI metadata
        // directly; for Python we keep the diff path since the scoped
        // Runtime is the authoritative source for that language.
        let mut rust_reactor_names: Vec<String> = Vec::new();
        let pre_load_reactor_names: std::collections::HashSet<String> = self
            .runtime
            .as_ref()
            .map(|rt| rt.reactor_names().into_iter().collect())
            .unwrap_or_default();

        let (task_namespaces, workflow_name, trigger_names, rust_graph_name) = if cloacina_manifest
            .metadata
            .language
            == "rust"
        {
            // T-0554 / I-0102: Rust path now runs the precedence-ordered
            // pipeline. Extract a unified PackageLoadView, then call six
            // step helpers in fixed order.
            let library_data =
                rust_cdylib_bytes
                    .clone()
                    .ok_or_else(|| RegistryError::RegistrationFailed {
                        message: format!(
                            "Rust package {} v{} has no compiled_data — compiler service must \
                             produce the cdylib before the reconciler loads it",
                            metadata.package_name, metadata.version
                        ),
                    })?;
            info!(
                "Loaded compiled cdylib ({} bytes) for {}",
                library_data.len(),
                metadata.package_name
            );

            let view = self.build_view_rust(&library_data).await?;
            rust_reactor_names = view.reactors.iter().map(|r| r.name.clone()).collect();

            // Step 1: cron triggers (no-op pending T-0553).
            self.step_load_cron_triggers(&metadata, &view)?;
            // Step 2: custom triggers (validated against runtime).
            let trigger_names = self.step_load_custom_triggers(&metadata, &view)?;
            // Step 3: reactors → graph scheduler.
            self.step_load_reactors(&metadata, &view, &cloacina_manifest.metadata)
                .await?;
            // Step 4: trigger-less CGs (handled at execute-time today).
            self.step_load_triggerless_cgs(&metadata, &view)?;
            // Step 5: reactor-bound CG → graph scheduler. We do this
            // BEFORE workflow registration so the (newly-supported)
            // workflow→graph dispatch can resolve graph names if it
            // wants to. Order is unchanged for legacy fixtures since
            // their workflows don't reference graphs.
            let rust_graph_name = self
                .step_load_reactor_bound_cgs(
                    &metadata,
                    &view,
                    &cloacina_manifest.metadata,
                    &library_data,
                )
                .await?;
            // Step 6: workflows (tasks + workflow + trigger-subscription
            // validation).
            let (task_namespaces, workflow_name) =
                self.step_load_workflows(&metadata, &library_data).await?;

            (
                task_namespaces,
                workflow_name,
                trigger_names,
                rust_graph_name,
            )
        } else if cloacina_manifest.metadata.language == "python"
            && !cloacina_manifest.metadata.has_computation_graph()
        {
            // Python workflow path — dispatched through the `PythonRuntime`
            // trait. Binaries that register no runtime (e.g. the compiler
            // service) error cleanly here; the server registers an impl
            // at startup.
            debug!("Loading Python package: {}", metadata.package_name);
            // T-0554 Phase 2: snapshot scoped runtime registries BEFORE
            // the Python import so `build_view_python` can compute the
            // diff (= primitives this package introduced) post-import
            // and feed the unified pipeline helpers.
            let py_pre_reactor_names: std::collections::HashSet<String> = self
                .runtime
                .as_ref()
                .map(|rt| rt.reactor_names().into_iter().collect())
                .unwrap_or_default();
            let py_pre_trigger_names: std::collections::HashSet<String> = self
                .runtime
                .as_ref()
                .map(|rt| rt.trigger_names().into_iter().collect())
                .unwrap_or_default();
            let py_pre_graph_names: std::collections::HashSet<String> = self
                .runtime
                .as_ref()
                .map(|rt| rt.computation_graph_names().into_iter().collect())
                .unwrap_or_default();
            let runtime = crate::python_runtime::python_runtime().ok_or_else(|| {
                RegistryError::RegistrationFailed {
                    message: "Python package {} received but no PythonRuntime is attached \
                              to this process — this binary does not support Python workflows"
                        .replace("{}", &metadata.package_name),
                }
            })?;

            let staging = work_dir.path().join("python-staging");
            let tenant_id = self.config.default_tenant_id.clone();
            let cloacina_runtime =
                self.runtime
                    .clone()
                    .ok_or_else(|| RegistryError::RegistrationFailed {
                        message: format!(
                        "Python package {} received but the reconciler has no Runtime attached — \
                         Python loads require a scoped Runtime",
                        metadata.package_name
                    ),
                    })?;
            let loaded = {
                let archive_data = loaded_workflow.package_data.clone();
                let runtime = runtime.clone();
                let cloacina_runtime = cloacina_runtime.clone();
                tokio::task::spawn_blocking(move || {
                    runtime
                        .load_workflow_package(
                            &archive_data,
                            &staging,
                            &tenant_id,
                            &cloacina_runtime,
                        )
                        .map_err(|e| RegistryError::RegistrationFailed { message: e })
                })
                .await
                .map_err(|e| RegistryError::RegistrationFailed {
                    message: format!("spawn_blocking failed during Python load: {}", e),
                })??
            };

            // Python triggers are registered during the runtime's import
            // pass. Track them from the manifest so we can unload later.
            let trigger_names =
                self.register_package_triggers(&metadata, &cloacina_manifest.metadata)?;

            // T-0554 Phase 2: route the Python load through the same
            // precedence-pipeline validation helpers as Rust. This is a
            // pure validation pass — the actual reactor + graph dispatch
            // still runs through the existing
            // `dispatch_runtime_reactors_into_scheduler` /
            // `register_package_triggers` paths below. The view here
            // surfaces wire-format consistent with `build_view_rust` so
            // the helpers behave identically.
            let py_view = self.build_view_python(
                &metadata.package_name,
                &py_pre_reactor_names,
                &py_pre_trigger_names,
                &py_pre_graph_names,
                &cloacina_manifest.metadata.accumulators,
            );
            self.step_load_cron_triggers(&metadata, &py_view)?;
            let _ = self.step_load_custom_triggers(&metadata, &py_view)?;

            // T-0545 M3a: dispatch any reactors the Python module declared
            // via `@cloaca.reactor` into the ComputationGraphScheduler. Lets
            // a Python workflow package that also declares reactors bring
            // them up at load time, without a co-located CG subscriber.
            {
                let scheduler_guard = self.graph_scheduler.read().await;
                if let Some(ref scheduler) = *scheduler_guard {
                    if let Err(e) =
                        crate::computation_graph::packaging_bridge::dispatch_runtime_reactors_into_scheduler(
                            cloacina_runtime.as_ref(),
                            scheduler,
                            &cloacina_manifest.metadata.accumulators,
                            Some(self.config.default_tenant_id.clone()),
                        )
                        .await
                    {
                        warn!(
                            "Failed to dispatch Python reactors from package {} into scheduler: {}",
                            metadata.package_name, e
                        );
                    }
                }
            }

            info!(
                "Python package loaded: {} v{} — {} tasks, workflow '{}'",
                metadata.package_name,
                metadata.version,
                loaded.task_namespaces.len(),
                loaded.workflow_name,
            );

            (
                loaded.task_namespaces,
                Some(loaded.workflow_name),
                trigger_names,
                None,
            )
        } else if cloacina_manifest.metadata.language == "python"
            && cloacina_manifest.metadata.has_computation_graph()
        {
            // Python CG packages: no workflow tasks to register.
            // The CG import happens in step 7 below (Python branch only;
            // Rust CG handled in the unified pipeline above).
            (vec![], None, vec![], None)
        } else {
            return Err(RegistryError::RegistrationFailed {
                    message: format!(
                        "Unsupported package language '{}' for package {} — only 'rust' and 'python' are supported",
                        cloacina_manifest.metadata.language, metadata.package_name
                    ),
                });
        };

        // --- Step 7: Python computation graph routing ---
        // T-0554: Rust CG handling moved into the unified pipeline above
        // (`step_load_reactor_bound_cgs`). This step now only handles the
        // Python CG path, which still needs the dedicated PythonRuntime
        // dispatch.
        let graph_name = if rust_graph_name.is_some() {
            rust_graph_name
        } else if cloacina_manifest.metadata.has_computation_graph() {
            if cloacina_manifest.metadata.language == "rust" {
                // Reuse the cdylib bytes already fetched for task registration
                // above — no recompilation, no extra DB hit.
                let library_data =
                    rust_cdylib_bytes
                        .clone()
                        .ok_or_else(|| RegistryError::RegistrationFailed {
                            message: format!(
                                "Rust CG package {} v{} has no compiled_data",
                                metadata.package_name, metadata.version
                            ),
                        })?;

                match self
                    .package_loader
                    .extract_graph_metadata(&library_data)
                    .await
                {
                    Ok(Some(graph_meta)) => {
                        info!(
                            "Computation graph detected: {} (accumulators: {:?})",
                            graph_meta.graph_name,
                            graph_meta
                                .accumulators
                                .iter()
                                .map(|a| &a.name)
                                .collect::<Vec<_>>()
                        );

                        // Merge manifest accumulator configs into FFI defaults
                        let mut graph_meta = graph_meta;
                        for manifest_acc in &cloacina_manifest.metadata.accumulators {
                            if let Some(ffi_acc) = graph_meta
                                .accumulators
                                .iter_mut()
                                .find(|a| a.name == manifest_acc.name)
                            {
                                ffi_acc.accumulator_type = manifest_acc.accumulator_type.clone();
                                ffi_acc.config = manifest_acc.config.clone();
                            }
                        }

                        let scheduler_guard = self.graph_scheduler.read().await;
                        if let Some(ref scheduler) = *scheduler_guard {
                            let mut decl =
                                crate::computation_graph::packaging_bridge::build_declaration_from_ffi(
                                    &graph_meta,
                                    library_data.clone(),
                                );
                            decl.tenant_id = Some(self.config.default_tenant_id.clone());

                            // Also register the CG on the attached Runtime so
                            // executors can look it up without going through
                            // the global CG registry.
                            if let Some(runtime) = &self.runtime {
                                let graph_fn = decl.reactor.graph_fn.clone();
                                let accumulator_names: Vec<String> =
                                    decl.accumulators.iter().map(|a| a.name.clone()).collect();
                                let reaction_mode = graph_meta.reaction_mode.clone();
                                runtime.register_computation_graph(
                                    graph_meta.graph_name.clone(),
                                    move || crate::ComputationGraphRegistration {
                                        graph_fn: graph_fn.clone(),
                                        // Packaged CG loading predates the
                                        // split form (I-0101). Entry
                                        // accumulators mirror the declared
                                        // accumulator set, and the reactor
                                        // is bundled with the graph, so we
                                        // don't carry a separate reactor
                                        // binding.
                                        entry_accumulators: accumulator_names.clone(),
                                        trigger_reactor: None,
                                        accumulator_names: accumulator_names.clone(),
                                        reaction_mode: reaction_mode.clone(),
                                    },
                                );
                            }

                            if let Err(e) = scheduler.load_graph(decl).await {
                                warn!(
                                    "Failed to load computation graph '{}': {}",
                                    graph_meta.graph_name, e
                                );
                            } else {
                                info!(
                                    "Computation graph '{}' loaded into ComputationGraphScheduler",
                                    graph_meta.graph_name
                                );
                            }
                        } else {
                            warn!(
                                "Computation graph '{}' detected but no ComputationGraphScheduler configured",
                                graph_meta.graph_name
                            );
                        }

                        Some(graph_meta.graph_name)
                    }
                    Ok(None) => {
                        debug!(
                            "Package claims computation_graph type but plugin doesn't support get_graph_metadata"
                        );
                        None
                    }
                    Err(e) => {
                        warn!("Failed to extract graph metadata: {}", e);
                        None
                    }
                }
            } else if cloacina_manifest.metadata.language == "python" {
                // Python computation graph: dispatch through the
                // `PythonRuntime` trait. Same reasoning as the workflow
                // branch — binaries without Python support error out here
                // instead of trying to run pyo3 they don't link.
                if let (Some(ref graph_name), Some(ref entry_module)) = (
                    &cloacina_manifest.metadata.graph_name,
                    &cloacina_manifest.metadata.entry_module,
                ) {
                    // T-0554 Phase 2: pre-snapshot scoped runtime so the
                    // post-load view can drive cross-package contract
                    // validation through the unified pipeline helpers.
                    let cg_pre_reactor_names: std::collections::HashSet<String> = self
                        .runtime
                        .as_ref()
                        .map(|rt| rt.reactor_names().into_iter().collect())
                        .unwrap_or_default();
                    let cg_pre_trigger_names: std::collections::HashSet<String> = self
                        .runtime
                        .as_ref()
                        .map(|rt| rt.trigger_names().into_iter().collect())
                        .unwrap_or_default();
                    let cg_pre_graph_names: std::collections::HashSet<String> = self
                        .runtime
                        .as_ref()
                        .map(|rt| rt.computation_graph_names().into_iter().collect())
                        .unwrap_or_default();
                    let runtime = crate::python_runtime::python_runtime().ok_or_else(|| {
                        RegistryError::RegistrationFailed {
                            message: format!(
                                "Python CG package {} received but no PythonRuntime \
                                     is attached to this process",
                                metadata.package_name
                            ),
                        }
                    })?;

                    let staging = work_dir.path().join("python-cg-staging");
                    let tenant = self.config.default_tenant_id.clone();
                    let acc_overrides = cloacina_manifest.metadata.accumulators.clone();
                    let gn = graph_name.clone();
                    let em = entry_module.clone();

                    let cloacina_runtime =
                        self.runtime
                            .clone()
                            .ok_or_else(|| RegistryError::RegistrationFailed {
                                message: format!(
                                "Python CG package {} received but the reconciler has no Runtime \
                                 attached — Python loads require a scoped Runtime",
                                metadata.package_name
                            ),
                            })?;
                    let maybe_decl = {
                        let archive_data = loaded_workflow.package_data.clone();
                        let gn_inner = gn.clone();
                        let em_inner = em.clone();
                        let tenant_inner = tenant.clone();
                        let runtime = runtime.clone();
                        let cloacina_runtime = cloacina_runtime.clone();
                        tokio::task::spawn_blocking(move || {
                            runtime
                                .load_cg_package(
                                    &archive_data,
                                    &staging,
                                    &tenant_inner,
                                    &gn_inner,
                                    &em_inner,
                                    &acc_overrides,
                                    &cloacina_runtime,
                                )
                                .map_err(|e| RegistryError::RegistrationFailed { message: e })
                        })
                        .await
                        .map_err(|e| {
                            RegistryError::RegistrationFailed {
                                message: format!(
                                    "spawn_blocking failed during Python CG load: {}",
                                    e
                                ),
                            }
                        })??
                    };

                    // T-0545 M3a: dispatch reactors the Python CG module
                    // declared via `@cloaca.reactor` BEFORE loading the
                    // graph itself. The reactor must be running first so
                    // load_graph's idempotent path finds it (T-0544 M2);
                    // otherwise it would synthesize a per-graph reactor
                    // and miss cross-package fan-out.
                    {
                        let scheduler_guard = self.graph_scheduler.read().await;
                        if let Some(ref scheduler) = *scheduler_guard {
                            if let Err(e) =
                                crate::computation_graph::packaging_bridge::dispatch_runtime_reactors_into_scheduler(
                                    cloacina_runtime.as_ref(),
                                    scheduler,
                                    &cloacina_manifest.metadata.accumulators,
                                    Some(self.config.default_tenant_id.clone()),
                                )
                                .await
                            {
                                warn!(
                                    "Failed to dispatch Python reactors from CG package {} into scheduler: {}",
                                    metadata.package_name, e
                                );
                            }
                        }
                    }

                    // T-0554 Phase 2: build the Python view + run
                    // pre-validation BEFORE handing the declaration to
                    // the scheduler. This catches cross-package contract
                    // mismatches (subscriber declares accumulator names
                    // not present on the upstream reactor) with a clear
                    // package-named error rather than the scheduler's
                    // generic "different contract" rejection.
                    let cg_view = self.build_view_python(
                        &metadata.package_name,
                        &cg_pre_reactor_names,
                        &cg_pre_trigger_names,
                        &cg_pre_graph_names,
                        &cloacina_manifest.metadata.accumulators,
                    );
                    self.step_load_cron_triggers(&metadata, &cg_view)?;
                    let _ = self.step_load_custom_triggers(&metadata, &cg_view)?;
                    if let Some(graph_meta) = cg_view.graph.as_ref() {
                        if let Some(upstream_reactor_name) = graph_meta.trigger_reactor.as_deref() {
                            let publisher_in_same_package = cg_view
                                .reactors
                                .iter()
                                .any(|r| r.name == upstream_reactor_name);
                            if !publisher_in_same_package {
                                let scheduler_guard = self.graph_scheduler.read().await;
                                if let Some(ref scheduler) = *scheduler_guard {
                                    if let Some(upstream_acc_names) = scheduler
                                        .reactor_accumulator_names(upstream_reactor_name)
                                        .await
                                    {
                                        let upstream_set: std::collections::HashSet<&str> =
                                            upstream_acc_names.iter().map(|s| s.as_str()).collect();
                                        let missing: Vec<String> = graph_meta
                                            .accumulators
                                            .iter()
                                            .filter(|a| !upstream_set.contains(a.name.as_str()))
                                            .map(|a| a.name.clone())
                                            .collect();
                                        if !missing.is_empty() {
                                            return Err(RegistryError::RegistrationFailed {
                                                message: format!(
                                                    "package '{}' subscribes to reactor '{}' but declares accumulator(s) {:?} \
                                                     that are not part of the upstream reactor's contract (upstream declares {:?})",
                                                    metadata.package_name,
                                                    upstream_reactor_name,
                                                    missing,
                                                    upstream_acc_names,
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(decl) = maybe_decl {
                        let scheduler_guard = self.graph_scheduler.read().await;
                        if let Some(ref scheduler) = *scheduler_guard {
                            if let Err(e) = scheduler.load_graph(decl).await {
                                warn!(
                                    "Failed to load Python CG '{}' into ComputationGraphScheduler: {}",
                                    gn, e
                                );
                            } else {
                                info!(
                                    "Python computation graph '{}' loaded into ComputationGraphScheduler",
                                    gn
                                );
                            }
                        }
                    }

                    info!(
                        "Python computation graph '{}' imported from '{}'",
                        graph_name, entry_module
                    );
                    Some(graph_name.clone())
                } else {
                    warn!("Python computation graph package missing graph_name or entry_module");
                    None
                }
            } else {
                debug!("Unsupported language for computation graph");
                None
            }
        } else {
            None
        };

        // Tasks, workflows, and CGs are registered directly on the Runtime by
        // the Rust + Python load paths above. For Rust cdylib packages, we
        // additionally seed the Runtime from `inventory` after dlopen so any
        // `#[trigger]`/`#[computation_graph]`/`#[task]` entries emitted by the
        // loaded library's `inventory::submit!` blocks land in the Runtime.
        // Python packages register through the thread-local scope and do not
        // need this re-seed.
        if let Some(runtime) = &self.runtime {
            if cloacina_manifest.metadata.language == "rust" {
                runtime.seed_from_inventory();
            }
        }

        // T-0554 Phase 2: track reactors this package owns. Rust path
        // uses FFI metadata (cross-cdylib safe). Python path uses the
        // pre/post `Runtime::reactor_names()` diff (works because the
        // scoped Runtime is the authoritative registry for Python).
        let reactor_names: Vec<String> = if cloacina_manifest.metadata.language == "rust" {
            rust_reactor_names
        } else {
            let post_load_reactor_names: std::collections::HashSet<String> = self
                .runtime
                .as_ref()
                .map(|rt| rt.reactor_names().into_iter().collect())
                .unwrap_or_default();
            post_load_reactor_names
                .difference(&pre_load_reactor_names)
                .cloned()
                .collect()
        };

        // Track the loaded package state
        let package_state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
            trigger_names,
            graph_name,
            reactor_names,
        };

        let mut loaded_packages = self.loaded_packages.write().await;
        loaded_packages.insert(metadata.id, package_state);
        drop(loaded_packages);

        Ok(())
    }

    /// Unload a package from the global registries.
    ///
    /// T-0554 Phase 2: tear-down runs in REVERSE precedence order
    /// (workflows → CGs → reactors → triggers → tasks). This mirrors the
    /// load pipeline and lets T-0544 M4's `unload_reactor` reject-with-
    /// subscribers guard fire cleanly when an operator tries to drop a
    /// publishing package while subscribers are still bound: the workflow
    /// + CG steps run first, but the reactor step refuses if any
    /// out-of-package subscriber remains, and the unload as a whole
    /// surfaces the rejection.
    pub(super) async fn unload_package(
        &self,
        package_id: WorkflowPackageId,
    ) -> Result<(), RegistryError> {
        debug!("Unloading package: {}", package_id);

        // Get the package state to know what to unload
        let mut loaded_packages = self.loaded_packages.write().await;
        let package_state =
            loaded_packages
                .remove(&package_id)
                .ok_or_else(|| RegistryError::PackageNotFound {
                    package_name: package_id.to_string(),
                    version: "unknown".to_string(),
                })?;
        drop(loaded_packages);

        // --- Step 1 (reverse): workflows ---
        // Drop the workflow registration so future executions can't kick
        // off new runs of this package's workflow.
        if let Some(runtime) = &self.runtime {
            if let Some(workflow_name) = &package_state.workflow_name {
                runtime.unregister_workflow(workflow_name);
            }
        }

        // --- Step 2 (reverse): computation graphs ---
        // Unbind the graph from its reactor (a no-op for trigger-less or
        // bundled-form graphs). For bundled-form, `unload_graph` ALSO
        // tears the reactor down if this was the last subscriber — that
        // path is preserved here for back-compat. Cross-package
        // subscribers leave the upstream reactor intact; the reactor's
        // own owning package tears it down in step 3.
        if let Some(graph_name) = &package_state.graph_name {
            if let Some(runtime) = &self.runtime {
                runtime.unregister_computation_graph(graph_name);
            }
            let scheduler_guard = self.graph_scheduler.read().await;
            if let Some(ref scheduler) = *scheduler_guard {
                if let Err(e) = scheduler.unload_graph(graph_name).await {
                    warn!("Failed to unload computation graph '{}': {}", graph_name, e);
                }
            }
        }

        // --- Step 3 (reverse): reactors owned by THIS package ---
        // Iterate the reactors this package introduced and ask the
        // scheduler to tear each one down. The T-0544 M4 guard inside
        // `unload_reactor` rejects when any cross-package subscriber is
        // still bound; we surface the first such rejection as the
        // unload's overall error so operators get a clean signal that
        // they need to unload subscribers first.
        let mut reactor_unload_error: Option<String> = None;
        if !package_state.reactor_names.is_empty() {
            let scheduler_guard = self.graph_scheduler.read().await;
            if let Some(ref scheduler) = *scheduler_guard {
                for reactor_name in &package_state.reactor_names {
                    match scheduler.unload_reactor(reactor_name).await {
                        Ok(()) => {
                            debug!(
                                "Reactor '{}' unloaded for package {}",
                                reactor_name, package_state.metadata.package_name
                            );
                        }
                        Err(e) => {
                            // Bundled-form CG packages: the reactor was
                            // already torn down by `unload_graph` above
                            // when this package was the last subscriber.
                            // Treat "not loaded" as a clean no-op.
                            if e.contains("not loaded") {
                                continue;
                            }
                            warn!(
                                "Failed to unload reactor '{}' from package {}: {}",
                                reactor_name, package_state.metadata.package_name, e
                            );
                            if reactor_unload_error.is_none() {
                                reactor_unload_error = Some(format!(
                                    "package '{}' owns reactor '{}' which has bound subscribers \
                                     from another package: {}; unload subscribers first",
                                    package_state.metadata.package_name, reactor_name, e
                                ));
                            }
                        }
                    }
                }
            }
        }

        // --- Step 4 (reverse): triggers ---
        if let Some(runtime) = &self.runtime {
            for trigger_name in &package_state.trigger_names {
                runtime.unregister_trigger(trigger_name);
            }
        }

        // --- Step 5 (reverse): tasks (drops the cdylib via task registrar) ---
        // Always run this step so the dlopen handle gets dropped even if
        // an earlier step warned. Reactor-only packages that fail to
        // unload due to bound subscribers will be rejected below; the
        // task registrar drop is harmless either way.
        if let Some(runtime) = &self.runtime {
            for ns in &package_state.task_namespaces {
                runtime.unregister_task(ns);
            }
        }
        let package_id_str = package_id.to_string();
        self.task_registrar
            .unregister_package_tasks(&package_id_str)
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to unregister package tasks: {}", e),
            })?;

        if let Some(msg) = reactor_unload_error {
            return Err(RegistryError::RegistrationFailed { message: msg });
        }

        info!(
            "Unloaded package: {} v{}",
            package_state.metadata.package_name, package_state.metadata.version
        );

        Ok(())
    }

    /// Register tasks from a package into the global task registry
    pub(super) async fn register_package_tasks(
        &self,
        metadata: &WorkflowMetadata,
        package_data: &[u8],
    ) -> Result<Vec<TaskNamespace>, RegistryError> {
        debug!(
            "Loading tasks for package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Extract metadata from the .so file using PackageLoader
        let package_metadata = self
            .package_loader
            .extract_metadata(package_data)
            .await
            .map_err(RegistryError::Loader)?;

        debug!(
            "Package {} contains {} tasks",
            package_metadata.package_name,
            package_metadata.tasks.len()
        );

        // Register tasks using TaskRegistrar
        let package_id = metadata.id.to_string();
        let tenant_id = Some(self.config.default_tenant_id.as_str());

        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| RegistryError::RegistrationFailed {
                message: format!(
                    "Rust package {} received but the reconciler has no Runtime attached — \
                     Rust loads require a scoped Runtime",
                    metadata.package_name
                ),
            })?;

        let task_namespaces = self
            .task_registrar
            .register_package_tasks(
                &package_id,
                package_data,
                &package_metadata,
                tenant_id,
                runtime,
            )
            .await
            .map_err(RegistryError::Loader)?;

        info!(
            "Successfully registered {} tasks for package {} v{}",
            task_namespaces.len(),
            metadata.package_name,
            metadata.version
        );

        Ok(task_namespaces)
    }

    /// Register workflows from a package into the global workflow registry
    pub(super) async fn register_package_workflows(
        &self,
        metadata: &WorkflowMetadata,
        package_data: &[u8],
    ) -> Result<Option<String>, RegistryError> {
        debug!(
            "Loading workflows for package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Extract metadata from the .so file using PackageLoader
        let package_metadata = self
            .package_loader
            .extract_metadata(package_data)
            .await
            .map_err(RegistryError::Loader)?;

        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| RegistryError::RegistrationFailed {
                message: format!(
                    "Rust package {} received but the reconciler has no Runtime attached — \
                     Rust loads require a scoped Runtime",
                    metadata.package_name
                ),
            })?;

        // Check if package has tasks (which means it has a workflow since it was compiled with the macro)
        if !package_metadata.tasks.is_empty() {
            debug!(
                "Package {} has {} tasks - workflow exists since it compiled with packaged_workflow macro",
                metadata.package_name,
                package_metadata.tasks.len()
            );

            // Extract the workflow name from the package metadata
            // The workflow name comes from the #[packaged_workflow(name = "...")] macro
            // Since package_loader::PackageMetadata doesn't have workflow_name field directly,
            // we need to extract it from the task metadata namespaced templates
            let workflow_name = {
                // Extract workflow name from namespaced_id_template
                if let Some(first_task) = package_metadata.tasks.first() {
                    let template = &first_task.namespaced_id_template;
                    debug!("Parsing workflow_name from template: '{}'", template);

                    // Split by "::" and extract the workflow_id part (3rd component)
                    let parts: Vec<&str> = template.split("::").collect();
                    if parts.len() >= 3 {
                        let workflow_part = parts[2];
                        // Handle both {workflow} placeholder and actual workflow_id
                        if workflow_part == "{workflow}" {
                            // This is a template, need to look up actual workflow_id from registered tasks
                            let mut found_id = None;
                            for namespace in runtime.task_namespaces() {
                                if namespace.package_name == metadata.package_name
                                    && namespace.tenant_id == self.config.default_tenant_id
                                {
                                    debug!(
                                        "Found registered task with workflow_id: '{}'",
                                        namespace.workflow_id
                                    );
                                    found_id = Some(namespace.workflow_id.clone());
                                    break;
                                }
                            }
                            // Use found ID or fallback
                            found_id.unwrap_or_else(|| metadata.package_name.clone())
                        } else {
                            // This is the actual workflow_id
                            workflow_part.to_string()
                        }
                    } else {
                        debug!("Template format unexpected, using package name as fallback");
                        metadata.package_name.clone()
                    }
                } else {
                    debug!("No tasks in package metadata, using package name as fallback");
                    metadata.package_name.clone()
                }
            };

            debug!(
                "Using workflow_name '{}' for workflow registration",
                workflow_name
            );

            // Use the actual package name from metadata — the namespaced_id_template
            // contains unresolved placeholders like {pkg} that don't match registered tasks
            let task_package_name = metadata.package_name.clone();

            debug!(
                "Using task_package_name '{}' for task lookup",
                task_package_name
            );

            // Create the workflow directly using the runtime-scoped task registry
            // (avoid FFI isolation issues).
            let _workflow = self.create_workflow_from_host_registry(
                &task_package_name, // Use the correct package name from task metadata
                &workflow_name,
                &self.config.default_tenant_id,
            )?;

            // Register workflow constructor on the runtime so it recreates the
            // workflow from the runtime's task registry each time.
            let workflow_name_for_closure = workflow_name.clone();
            let package_name_for_closure = task_package_name.clone();
            let workflow_name_for_closure_static = workflow_name.clone();
            let tenant_id_for_closure = self.config.default_tenant_id.clone();
            let runtime_for_closure: Arc<Runtime> = runtime.clone();

            runtime.register_workflow(workflow_name.clone(), move || {
                debug!(
                    "Creating workflow instance for {} using runtime registry",
                    workflow_name_for_closure
                );

                // Recreate the workflow from the runtime task registry each time
                match Self::create_workflow_from_host_registry_static(
                    &runtime_for_closure,
                    &package_name_for_closure,
                    &workflow_name_for_closure_static,
                    &tenant_id_for_closure,
                ) {
                    Ok(workflow) => workflow,
                    Err(e) => {
                        error!("Failed to create workflow from runtime registry: {}", e);
                        // Fallback to empty workflow
                        crate::workflow::Workflow::new(&workflow_name_for_closure)
                    }
                }
            });

            info!(
                "Registered workflow '{}' for package {} v{}",
                workflow_name, metadata.package_name, metadata.version
            );

            Ok(Some(workflow_name))
        } else {
            debug!(
                "Package {} has no workflow data - registering as task-only package",
                metadata.package_name
            );
            Ok(None)
        }
    }

    /// Create a workflow using the runtime-scoped task registry (avoiding FFI isolation).
    pub(super) fn create_workflow_from_host_registry(
        &self,
        package_name: &str,
        workflow_name: &str,
        tenant_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| RegistryError::RegistrationFailed {
                message: "create_workflow_from_host_registry called without a Runtime attached"
                    .to_string(),
            })?;
        Self::create_workflow_from_host_registry_static(
            runtime,
            package_name,
            workflow_name,
            tenant_id,
        )
    }

    /// Static version of create_workflow_from_host_registry for use in closures.
    pub(super) fn create_workflow_from_host_registry_static(
        runtime: &Arc<Runtime>,
        package_name: &str,
        workflow_name: &str,
        tenant_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        // Create workflow and add registered tasks from the runtime task registry
        let mut workflow = crate::workflow::Workflow::new(workflow_name);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        let mut found_tasks = 0;
        for namespace in runtime.task_namespaces() {
            // Only include tasks from this package, workflow, and tenant
            if namespace.package_name == package_name
                && namespace.workflow_id == workflow_name
                && namespace.tenant_id == tenant_id
            {
                let task = runtime.get_task(&namespace).ok_or_else(|| {
                    RegistryError::RegistrationFailed {
                        message: format!(
                            "Task {} vanished from runtime registry between enumeration and lookup",
                            namespace
                        ),
                    }
                })?;
                workflow
                    .add_task(task)
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!(
                            "Failed to add task {} to workflow: {:?}",
                            namespace.task_id, e
                        ),
                    })?;
                found_tasks += 1;
            }
        }

        debug!(
            "Created workflow '{}' with {} tasks from runtime registry",
            workflow_name, found_tasks
        );

        // Validate and finalize the workflow
        workflow
            .validate()
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Workflow validation failed: {:?}", e),
            })?;

        Ok(workflow.finalize())
    }

    /// Verify and track triggers declared in a package's `CloacinaMetadata`.
    ///
    /// The package's trigger implementations land in the Runtime via
    /// `Runtime::seed_from_inventory()`, called immediately after the cdylib
    /// is dlopened. This method checks that each declared trigger actually
    /// appeared in the Runtime and tracks its name so it can be removed when
    /// the package is unloaded.
    /// Validate that every trigger named in `#[workflow(triggers = [...])]`
    /// is registered in the runtime. Hard-errors if any name is missing —
    /// the package's workflow won't fire from those triggers and the user
    /// almost certainly typo'd a name.
    pub(super) async fn validate_workflow_trigger_subscriptions(
        &self,
        metadata: &WorkflowMetadata,
        package_data: &[u8],
    ) -> Result<(), RegistryError> {
        let package_metadata = self
            .package_loader
            .extract_metadata(package_data)
            .await
            .map_err(RegistryError::Loader)?;

        if package_metadata.workflow_triggers.is_empty() {
            return Ok(());
        }

        let Some(runtime) = self.runtime.as_ref() else {
            warn!(
                "Package {} v{} declares workflow trigger subscriptions but the reconciler has no \
                 Runtime attached",
                metadata.package_name, metadata.version
            );
            return Ok(());
        };

        let mut missing: Vec<String> = Vec::new();
        for trigger_name in &package_metadata.workflow_triggers {
            if runtime.get_trigger(trigger_name).is_none() {
                missing.push(trigger_name.clone());
            }
        }

        if !missing.is_empty() {
            return Err(RegistryError::RegistrationFailed {
                message: format!(
                    "Package {} v{} declares `#[workflow(triggers = [...])]` referencing \
                     trigger(s) not registered in this runtime: {:?}. Ensure the package \
                     declaring those triggers is loaded first, or remove the typo.",
                    metadata.package_name, metadata.version, missing
                ),
            });
        }

        info!(
            "Package {} v{}: validated {} workflow trigger subscription(s)",
            metadata.package_name,
            metadata.version,
            package_metadata.workflow_triggers.len()
        );
        Ok(())
    }

    /// T-E / I-0102: legacy `[[triggers]]` manifest path is removed.
    /// Workflow → trigger subscriptions now flow through
    /// `#[workflow(triggers = [...])]` and arrive in the FFI metadata as
    /// `PackageTasksMetadata.triggers` (consumed by
    /// `validate_workflow_trigger_subscriptions`). This shim returns the
    /// empty Vec for callers still wired in until a follow-up cleans them
    /// up.
    pub(super) fn register_package_triggers(
        &self,
        _metadata: &WorkflowMetadata,
        _cloacina_metadata: &cloacina_workflow_plugin::CloacinaMetadata,
    ) -> Result<Vec<String>, RegistryError> {
        Ok(Vec::new())
    }

    // ========================================================================
    // T-0554 — Precedence-ordered load pipeline.
    //
    // Six steps in fixed order: cron triggers → custom triggers → reactors
    // → trigger-less CGs → reactor-bound CGs → workflows. Each helper is
    // language-agnostic: it consumes a `PackageLoadView` produced by
    // a per-language metadata-extraction adapter. Today only the Rust
    // adapter is wired; Python parity is a follow-up.
    // ========================================================================

    /// Extract a `PackageLoadView` from a Python scoped Runtime, given
    /// pre-load snapshots of the runtime's reactor / trigger / graph
    /// names. The diff identifies primitives this package introduced;
    /// the helper walks the runtime to materialize wire-format metadata
    /// matching what `build_view_rust` produces from FFI extraction.
    ///
    /// Tasks (the `PackageMetadata` field) intentionally come back empty
    /// — the Python load path's `register_package_tasks`-equivalent runs
    /// before this adapter is called, and the unified pipeline doesn't
    /// re-register tasks from the view. (Tasks live on the `Runtime`
    /// directly for Python; only the metadata view needs the shape
    /// match.) (T-0554 Phase 2)
    pub(super) fn build_view_python(
        &self,
        package_name: &str,
        pre_reactor_names: &std::collections::HashSet<String>,
        pre_trigger_names: &std::collections::HashSet<String>,
        pre_graph_names: &std::collections::HashSet<String>,
        accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    ) -> PackageLoadView {
        use cloacina_workflow_plugin::{
            types::AccumulatorDeclarationEntry, GraphPackageMetadata, ReactorPackageMetadata,
            TriggerPackageMetadata,
        };

        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => {
                return PackageLoadView {
                    tasks: crate::registry::loader::package_loader::PackageMetadata {
                        package_name: package_name.to_string(),
                        version: String::new(),
                        description: None,
                        author: None,
                        tasks: vec![],
                        graph_data: None,
                        architecture: String::new(),
                        symbols: vec![],
                        workflow_triggers: vec![],
                    },
                    triggers: vec![],
                    reactors: vec![],
                    graph: None,
                };
            }
        };

        // Reactors: diff vs. pre-snapshot, then walk each runtime
        // ReactorRegistration into wire-format with manifest accumulator
        // overrides folded in.
        let mut reactors: Vec<ReactorPackageMetadata> = Vec::new();
        for name in runtime.reactor_names() {
            if pre_reactor_names.contains(&name) {
                continue;
            }
            let Some(reg) = runtime.get_reactor(&name) else {
                continue;
            };
            let accumulators: Vec<AccumulatorDeclarationEntry> = reg
                .accumulator_names
                .iter()
                .map(|acc_name| {
                    let (accumulator_type, config) = accumulator_overrides
                        .iter()
                        .find(|cfg| &cfg.name == acc_name)
                        .map(|cfg| (cfg.accumulator_type.clone(), cfg.config.clone()))
                        .unwrap_or_else(|| ("passthrough".to_string(), Default::default()));
                    AccumulatorDeclarationEntry {
                        name: acc_name.clone(),
                        accumulator_type,
                        config,
                    }
                })
                .collect();
            let reaction_mode = match reg.reaction_mode {
                cloacina_computation_graph::ReactionMode::WhenAll => "when_all".to_string(),
                _ => "when_any".to_string(),
            };
            reactors.push(ReactorPackageMetadata {
                name: reg.name,
                package_name: package_name.to_string(),
                reaction_mode,
                accumulators,
            });
        }

        // Custom-poll triggers: the Python load path doesn't currently
        // register cron triggers via this surface (cron runs through the
        // daemon's separate path). Walk each runtime trigger and emit
        // wire-format with cron_expression carried through.
        let mut triggers: Vec<TriggerPackageMetadata> = Vec::new();
        for name in runtime.trigger_names() {
            if pre_trigger_names.contains(&name) {
                continue;
            }
            let Some(impl_) = runtime.get_trigger(&name) else {
                continue;
            };
            triggers.push(TriggerPackageMetadata {
                name: impl_.name().to_string(),
                package_name: package_name.to_string(),
                poll_interval: format!("{}s", impl_.poll_interval().as_secs()),
                cron_expression: impl_.cron_expression(),
                allow_concurrent: impl_.allow_concurrent(),
            });
        }

        // Computation graph: Python packages declare at most one CG per
        // package today. Pull the first non-pre-snapshot graph name and
        // shape it into wire-format. trigger_reactor / accumulators come
        // from the runtime registration directly.
        let graph: Option<GraphPackageMetadata> = runtime
            .computation_graph_names()
            .into_iter()
            .find(|n| !pre_graph_names.contains(n))
            .and_then(|graph_name| {
                let reg = runtime.get_computation_graph(&graph_name)?;
                let accumulators: Vec<AccumulatorDeclarationEntry> = reg
                    .accumulator_names
                    .iter()
                    .map(|acc_name| {
                        let (accumulator_type, config) = accumulator_overrides
                            .iter()
                            .find(|cfg| &cfg.name == acc_name)
                            .map(|cfg| (cfg.accumulator_type.clone(), cfg.config.clone()))
                            .unwrap_or_else(|| ("passthrough".to_string(), Default::default()));
                        AccumulatorDeclarationEntry {
                            name: acc_name.clone(),
                            accumulator_type,
                            config,
                        }
                    })
                    .collect();
                Some(GraphPackageMetadata {
                    graph_name,
                    package_name: package_name.to_string(),
                    reaction_mode: reg.reaction_mode.clone(),
                    input_strategy: "latest".to_string(),
                    accumulators,
                    trigger_reactor: reg.trigger_reactor.clone(),
                })
            });

        PackageLoadView {
            tasks: crate::registry::loader::package_loader::PackageMetadata {
                package_name: package_name.to_string(),
                version: String::new(),
                description: None,
                author: None,
                tasks: vec![],
                graph_data: None,
                architecture: String::new(),
                symbols: vec![],
                workflow_triggers: vec![],
            },
            triggers,
            reactors,
            graph,
        }
    }

    /// Extract a `PackageLoadView` from a Rust cdylib via fidius FFI.
    pub(super) async fn build_view_rust(
        &self,
        library_data: &[u8],
    ) -> Result<PackageLoadView, RegistryError> {
        let tasks = self
            .package_loader
            .extract_metadata(library_data)
            .await
            .map_err(RegistryError::Loader)?;

        let triggers = self
            .package_loader
            .extract_trigger_metadata(library_data)
            .await
            .map_err(RegistryError::Loader)
            .unwrap_or_default();

        let reactors = self
            .package_loader
            .extract_reactor_metadata(library_data)
            .await
            .map_err(RegistryError::Loader)
            .unwrap_or_default();

        let graph = match self
            .package_loader
            .extract_graph_metadata(library_data)
            .await
        {
            Ok(opt) => opt,
            Err(_) => None,
        };

        Ok(PackageLoadView {
            tasks,
            triggers,
            reactors,
            graph,
        })
    }

    /// Pipeline step 1: cron triggers (entries with `cron_expression.is_some()`).
    /// Today's path: cron schedules are registered by the daemon (T-0553),
    /// not the reconciler. Step is a no-op for now; logs the count.
    pub(super) fn step_load_cron_triggers(
        &self,
        metadata: &WorkflowMetadata,
        view: &PackageLoadView,
    ) -> Result<(), RegistryError> {
        let cron_count = view
            .triggers
            .iter()
            .filter(|t| t.cron_expression.is_some())
            .count();
        if cron_count > 0 {
            debug!(
                "Package {} v{}: {} cron trigger(s) declared (registration pending T-0553)",
                metadata.package_name, metadata.version, cron_count
            );
        }
        Ok(())
    }

    /// Pipeline step 2: custom-poll triggers (entries with
    /// `cron_expression.is_none()`). Today's path: triggers register
    /// themselves into the runtime via `inventory::submit!` at dlopen. The
    /// reconciler validates each declared trigger has a runtime impl.
    pub(super) fn step_load_custom_triggers(
        &self,
        metadata: &WorkflowMetadata,
        view: &PackageLoadView,
    ) -> Result<Vec<String>, RegistryError> {
        let custom: Vec<&cloacina_workflow_plugin::TriggerPackageMetadata> = view
            .triggers
            .iter()
            .filter(|t| t.cron_expression.is_none())
            .collect();
        if custom.is_empty() {
            return Ok(Vec::new());
        }
        let Some(runtime) = self.runtime.as_ref() else {
            warn!(
                "Package {} v{}: {} custom trigger(s) declared but no Runtime is attached",
                metadata.package_name,
                metadata.version,
                custom.len()
            );
            return Ok(Vec::new());
        };
        let mut tracked = Vec::new();
        for t in &custom {
            if runtime.get_trigger(&t.name).is_some() {
                tracked.push(t.name.clone());
            } else {
                warn!(
                    "Package {} v{}: trigger '{}' declared in metadata but no Trigger impl in runtime",
                    metadata.package_name, metadata.version, t.name,
                );
            }
        }
        Ok(tracked)
    }

    /// Pipeline step 3: reactors. Dispatches each reactor entry into the
    /// graph scheduler via `dispatch_package_reactors_into_scheduler`.
    pub(super) async fn step_load_reactors(
        &self,
        metadata: &WorkflowMetadata,
        view: &PackageLoadView,
        manifest: &cloacina_workflow_plugin::CloacinaMetadata,
    ) -> Result<(), RegistryError> {
        if view.reactors.is_empty() {
            return Ok(());
        }
        let scheduler_guard = self.graph_scheduler.read().await;
        let Some(scheduler) = scheduler_guard.as_ref() else {
            warn!(
                "Package {} v{}: {} reactor(s) declared but no ComputationGraphScheduler is configured",
                metadata.package_name, metadata.version, view.reactors.len()
            );
            return Ok(());
        };
        if let Err(e) =
            crate::computation_graph::packaging_bridge::dispatch_package_reactors_into_scheduler(
                &view.reactors,
                scheduler,
                &manifest.accumulators,
                Some(self.config.default_tenant_id.clone()),
            )
            .await
        {
            warn!(
                "Failed to dispatch package-declared reactors from {}: {}",
                metadata.package_name, e
            );
        }
        Ok(())
    }

    /// Pipeline step 4: trigger-less CGs. Today's path: the `#[task(invokes
    /// = computation_graph("name"))]` runtime walk pulls these from the
    /// `TriggerlessGraphEntry` inventory at execute time. The reconciler
    /// step is a no-op; logs the trigger-less graph names if any.
    pub(super) fn step_load_triggerless_cgs(
        &self,
        _metadata: &WorkflowMetadata,
        _view: &PackageLoadView,
    ) -> Result<(), RegistryError> {
        // Trigger-less CG inventory submission happens at dlopen via the
        // `#[computation_graph]` macro's emission; runtime walks it at
        // task-invocation time. No reconciler-side action needed today.
        Ok(())
    }

    /// Pipeline step 5: reactor-bound CGs. Dispatches the (single)
    /// computation graph from `view.graph` into the scheduler, merging
    /// manifest accumulator config overrides.
    pub(super) async fn step_load_reactor_bound_cgs(
        &self,
        metadata: &WorkflowMetadata,
        view: &PackageLoadView,
        manifest: &cloacina_workflow_plugin::CloacinaMetadata,
        library_data: &[u8],
    ) -> Result<Option<String>, RegistryError> {
        let Some(graph_meta) = view.graph.clone() else {
            return Ok(None);
        };
        info!(
            "Computation graph detected: {} (accumulators: {:?})",
            graph_meta.graph_name,
            graph_meta
                .accumulators
                .iter()
                .map(|a| &a.name)
                .collect::<Vec<_>>()
        );
        // Merge manifest accumulator configs into FFI defaults.
        let mut graph_meta = graph_meta;
        for manifest_acc in &manifest.accumulators {
            if let Some(ffi_acc) = graph_meta
                .accumulators
                .iter_mut()
                .find(|a| a.name == manifest_acc.name)
            {
                ffi_acc.accumulator_type = manifest_acc.accumulator_type.clone();
                ffi_acc.config = manifest_acc.config.clone();
            }
        }

        let scheduler_guard = self.graph_scheduler.read().await;
        let Some(scheduler) = scheduler_guard.as_ref() else {
            warn!(
                "Computation graph '{}' detected but no ComputationGraphScheduler configured",
                graph_meta.graph_name
            );
            return Ok(Some(graph_meta.graph_name));
        };

        // T-0554 Phase 2: cross-package contract validation. When the
        // subscriber binds to a reactor it does not own (publisher loaded
        // by a previous package), validate the subscriber's declared
        // accumulator names match the upstream reactor's contract before
        // we hand the declaration to the scheduler. The error path here
        // names the offending package + the missing accumulators, which
        // is more actionable than the generic "different contract"
        // message that surfaces from `load_reactor`'s idempotent guard.
        if let Some(upstream_reactor_name) = graph_meta.trigger_reactor.as_deref() {
            let publisher_in_same_package = view
                .reactors
                .iter()
                .any(|r| r.name == upstream_reactor_name);
            if !publisher_in_same_package {
                if let Some(upstream_acc_names) = scheduler
                    .reactor_accumulator_names(upstream_reactor_name)
                    .await
                {
                    let upstream_set: std::collections::HashSet<&str> =
                        upstream_acc_names.iter().map(|s| s.as_str()).collect();
                    let missing: Vec<String> = graph_meta
                        .accumulators
                        .iter()
                        .filter(|a| !upstream_set.contains(a.name.as_str()))
                        .map(|a| a.name.clone())
                        .collect();
                    if !missing.is_empty() {
                        return Err(RegistryError::RegistrationFailed {
                            message: format!(
                                "package '{}' subscribes to reactor '{}' but declares accumulator(s) {:?} \
                                 that are not part of the upstream reactor's contract (upstream declares {:?})",
                                metadata.package_name,
                                upstream_reactor_name,
                                missing,
                                upstream_acc_names,
                            ),
                        });
                    }
                } else {
                    return Err(RegistryError::RegistrationFailed {
                        message: format!(
                            "package '{}' subscribes to reactor '{}' but no such reactor is loaded; \
                             the publishing package must load before its subscribers",
                            metadata.package_name, upstream_reactor_name,
                        ),
                    });
                }
            }
        }

        let mut decl = crate::computation_graph::packaging_bridge::build_declaration_from_ffi(
            &graph_meta,
            library_data.to_vec(),
        );
        decl.tenant_id = Some(self.config.default_tenant_id.clone());

        // Mirror the CG into the scoped Runtime so executors can look it
        // up without going through the global registry.
        if let Some(runtime) = &self.runtime {
            let graph_fn = decl.reactor.graph_fn.clone();
            let accumulator_names: Vec<String> =
                decl.accumulators.iter().map(|a| a.name.clone()).collect();
            let reaction_mode = graph_meta.reaction_mode.clone();
            runtime.register_computation_graph(graph_meta.graph_name.clone(), move || {
                crate::ComputationGraphRegistration {
                    graph_fn: graph_fn.clone(),
                    entry_accumulators: accumulator_names.clone(),
                    trigger_reactor: None,
                    accumulator_names: accumulator_names.clone(),
                    reaction_mode: reaction_mode.clone(),
                }
            });
        }

        if let Err(e) = scheduler.load_graph(decl).await {
            warn!(
                "Failed to load computation graph '{}' for package {}: {}",
                graph_meta.graph_name, metadata.package_name, e
            );
        } else {
            info!(
                "Computation graph '{}' loaded into ComputationGraphScheduler",
                graph_meta.graph_name
            );
        }

        Ok(Some(graph_meta.graph_name))
    }

    /// Pipeline step 6: workflows. Registers tasks + workflow + validates
    /// `#[workflow(triggers = [...])]` subscriptions against the runtime.
    pub(super) async fn step_load_workflows(
        &self,
        metadata: &WorkflowMetadata,
        library_data: &[u8],
    ) -> Result<(Vec<TaskNamespace>, Option<String>), RegistryError> {
        let task_namespaces = self.register_package_tasks(metadata, library_data).await?;
        let workflow_name = self
            .register_package_workflows(metadata, library_data)
            .await?;
        // T-0554 Phase 2 e2e fix: seed inventory BEFORE the trigger
        // subscription validator runs. `register_package_tasks` above
        // dlopened the cdylib, which triggered the package's
        // `inventory::submit!` blocks. Without this seed call, the
        // submitted entries sit in inventory but aren't reflected in
        // the runtime's trigger registry yet, so a workflow that
        // declares `triggers = [...]` against a trigger from the same
        // cdylib would always fail validation. The end-of-load seed
        // call elsewhere covers re-seeds for late lookups; this one
        // makes the in-package case work.
        if let Some(runtime) = &self.runtime {
            runtime.seed_from_inventory();
        }
        self.validate_workflow_trigger_subscriptions(metadata, library_data)
            .await?;
        Ok((task_namespaces, workflow_name))
    }
}

/// T-0554 — Unified package metadata view fed into the precedence
/// pipeline. Wire-format types from `cloacina-workflow-plugin`. Both the
/// Rust FFI extraction path and (future) Python scoped-Runtime adapter
/// produce values of this shape.
pub(super) struct PackageLoadView {
    pub(super) tasks: crate::registry::loader::package_loader::PackageMetadata,
    pub(super) triggers: Vec<cloacina_workflow_plugin::TriggerPackageMetadata>,
    pub(super) reactors: Vec<cloacina_workflow_plugin::ReactorPackageMetadata>,
    pub(super) graph: Option<cloacina_workflow_plugin::GraphPackageMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::reconciler::ReconcilerConfig;
    use crate::registry::workflow_registry::filesystem::FilesystemWorkflowRegistry;
    use crate::Runtime;
    use serial_test::serial;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Create a minimal RegistryReconciler for testing, wired up to a scoped
    /// empty Runtime so trigger tracking doesn't depend on ambient globals.
    fn make_test_reconciler() -> RegistryReconciler {
        let registry = Arc::new(FilesystemWorkflowRegistry::new(vec![]));
        let config = ReconcilerConfig::default();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let reconciler = RegistryReconciler::new(registry, config, rx)
            .expect("Failed to create test reconciler");
        reconciler.with_runtime(Arc::new(Runtime::empty()))
    }

    fn runtime_of(r: &RegistryReconciler) -> Arc<Runtime> {
        r.runtime.clone().expect("reconciler must have a runtime")
    }

    fn make_test_metadata() -> WorkflowMetadata {
        WorkflowMetadata {
            id: Uuid::new_v4(),
            registry_id: Uuid::new_v4(),
            package_name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            author: None,
            tasks: vec![],
            schedules: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_cloacina_metadata_with_triggers(
        _triggers: Vec<cloacina_workflow_plugin::TriggerDefinition>,
    ) -> cloacina_workflow_plugin::CloacinaMetadata {
        // T-E / I-0102: `[[triggers]]` / `package_type` removed from
        // CloacinaMetadata. Trigger registration now flows through FFI
        // metadata; the manifest-side path is gone.
        cloacina_workflow_plugin::CloacinaMetadata {
            workflow_name: Some("test-workflow".to_string()),
            graph_name: None,
            language: "python".to_string(),
            description: Some("Test".to_string()),
            author: None,
            requires_python: None,
            entry_module: Some("test.tasks".to_string()),
            reaction_mode: None,
            input_strategy: None,
            accumulators: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // register_package_triggers tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn register_triggers_with_no_triggers_returns_empty() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();
        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        assert!(result.is_empty());
    }

    // T-E / I-0102: register_triggers_tracks_registered_triggers and
    // register_triggers_mixed_registered_and_missing tests deleted — they
    // asserted manifest-side `[[triggers]]` parsing, which is gone. The
    // function is now a no-op shim returning empty Vec; the remaining
    // tests in this section verify that contract.

    #[tokio::test]
    #[serial]
    async fn register_triggers_skips_unregistered_triggers() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();

        let trigger_name = format!("nonexistent-trigger-{}", Uuid::new_v4());
        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![
            cloacina_workflow_plugin::TriggerDefinition {
                name: trigger_name.clone(),
                workflow: "test-workflow".to_string(),
                poll_interval: "10s".to_string(),
                cron_expression: None,
                allow_concurrent: false,
            },
        ]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        // Should be empty because the trigger is not present in the runtime
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // T-0554 Phase 2: build_view_python adapter tests
    // -----------------------------------------------------------------------

    fn empty_pre_snapshots() -> (
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
    ) {
        (
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
        )
    }

    #[tokio::test]
    #[serial]
    async fn build_view_python_returns_empty_view_for_unloaded_runtime() {
        let reconciler = make_test_reconciler();
        let (pre_r, pre_t, pre_g) = empty_pre_snapshots();
        let view = reconciler.build_view_python("empty-pkg", &pre_r, &pre_t, &pre_g, &[]);
        assert!(view.reactors.is_empty());
        assert!(view.triggers.is_empty());
        assert!(view.graph.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn build_view_python_emits_wire_format_for_runtime_reactor() {
        let reconciler = make_test_reconciler();
        let runtime = runtime_of(&reconciler);
        runtime.register_reactor("py_reactor".to_string(), || {
            cloacina_computation_graph::ReactorRegistration {
                name: "py_reactor".to_string(),
                accumulator_names: vec!["src_a".to_string(), "src_b".to_string()],
                reaction_mode: cloacina_computation_graph::ReactionMode::WhenAny,
            }
        });

        let (pre_r, pre_t, pre_g) = empty_pre_snapshots();
        let view = reconciler.build_view_python("py-pkg", &pre_r, &pre_t, &pre_g, &[]);
        assert_eq!(view.reactors.len(), 1);
        let r = &view.reactors[0];
        assert_eq!(r.name, "py_reactor");
        assert_eq!(r.package_name, "py-pkg");
        assert_eq!(r.reaction_mode, "when_any");
        let acc_names: Vec<&str> = r.accumulators.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(acc_names, vec!["src_a", "src_b"]);
        assert!(r
            .accumulators
            .iter()
            .all(|a| a.accumulator_type == "passthrough"));
    }

    #[tokio::test]
    #[serial]
    async fn build_view_python_skips_pre_snapshot_entries() {
        let reconciler = make_test_reconciler();
        let runtime = runtime_of(&reconciler);
        runtime.register_reactor("preexisting".to_string(), || {
            cloacina_computation_graph::ReactorRegistration {
                name: "preexisting".to_string(),
                accumulator_names: vec!["x".to_string()],
                reaction_mode: cloacina_computation_graph::ReactionMode::WhenAny,
            }
        });

        let mut pre_r = std::collections::HashSet::new();
        pre_r.insert("preexisting".to_string());
        let (_, pre_t, pre_g) = empty_pre_snapshots();
        let view = reconciler.build_view_python("py-pkg", &pre_r, &pre_t, &pre_g, &[]);
        assert!(
            view.reactors.is_empty(),
            "pre-snapshot reactors must not appear in the diff view"
        );
    }

    #[tokio::test]
    #[serial]
    async fn build_view_python_folds_accumulator_overrides() {
        let reconciler = make_test_reconciler();
        let runtime = runtime_of(&reconciler);
        runtime.register_reactor("py_reactor".to_string(), || {
            cloacina_computation_graph::ReactorRegistration {
                name: "py_reactor".to_string(),
                accumulator_names: vec!["topic_a".to_string()],
                reaction_mode: cloacina_computation_graph::ReactionMode::WhenAll,
            }
        });

        let mut config = std::collections::HashMap::new();
        config.insert("topic".to_string(), "events".to_string());
        let overrides = vec![cloacina_workflow_plugin::types::AccumulatorConfig {
            name: "topic_a".to_string(),
            accumulator_type: "stream".to_string(),
            config,
        }];

        let (pre_r, pre_t, pre_g) = empty_pre_snapshots();
        let view = reconciler.build_view_python("py-pkg", &pre_r, &pre_t, &pre_g, &overrides);
        assert_eq!(view.reactors.len(), 1);
        let acc = &view.reactors[0].accumulators[0];
        assert_eq!(acc.name, "topic_a");
        assert_eq!(acc.accumulator_type, "stream");
        assert_eq!(acc.config.get("topic").map(|s| s.as_str()), Some("events"));
        assert_eq!(view.reactors[0].reaction_mode, "when_all");
    }

    // -----------------------------------------------------------------------
    // Dummy trigger for testing
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone)]
    struct DummyTrigger {
        name: String,
    }

    #[async_trait::async_trait]
    impl crate::trigger::Trigger for DummyTrigger {
        fn name(&self) -> &str {
            &self.name
        }

        fn poll_interval(&self) -> std::time::Duration {
            std::time::Duration::from_secs(60)
        }

        fn allow_concurrent(&self) -> bool {
            false
        }

        async fn poll(
            &self,
        ) -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError> {
            Ok(cloacina_workflow::TriggerResult::Skip)
        }
    }

    // -----------------------------------------------------------------------
    // T-0554 Phase 2 + T-0553 deferred AC: cross-package contract validation
    // and reverse-order unload pipeline e2e (in-crate scaffolding).
    // -----------------------------------------------------------------------

    use crate::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use crate::computation_graph::registry::EndpointRegistry;
    use crate::computation_graph::scheduler::{AccumulatorDeclaration, ComputationGraphScheduler};

    fn make_test_view_with_subscriber_graph(
        package_name: &str,
        upstream_reactor: &str,
        subscriber_accumulators: Vec<&str>,
    ) -> PackageLoadView {
        use cloacina_workflow_plugin::{types::AccumulatorDeclarationEntry, GraphPackageMetadata};
        let accumulators: Vec<AccumulatorDeclarationEntry> = subscriber_accumulators
            .into_iter()
            .map(|n| AccumulatorDeclarationEntry {
                name: n.to_string(),
                accumulator_type: "passthrough".to_string(),
                config: Default::default(),
            })
            .collect();
        PackageLoadView {
            tasks: crate::registry::loader::package_loader::PackageMetadata {
                package_name: package_name.to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                tasks: vec![],
                graph_data: None,
                architecture: String::new(),
                symbols: vec![],
                workflow_triggers: vec![],
            },
            triggers: vec![],
            reactors: vec![],
            graph: Some(GraphPackageMetadata {
                graph_name: format!("{}_graph", package_name),
                package_name: package_name.to_string(),
                reaction_mode: "when_any".to_string(),
                input_strategy: "latest".to_string(),
                accumulators,
                trigger_reactor: Some(upstream_reactor.to_string()),
            }),
        }
    }

    async fn load_publishing_reactor_into_scheduler(
        scheduler: &Arc<ComputationGraphScheduler>,
        reactor_name: &str,
        accumulators: &[&str],
    ) {
        use crate::computation_graph::packaging_bridge::PassthroughAccumulatorFactory;
        let acc_decls: Vec<AccumulatorDeclaration> = accumulators
            .iter()
            .map(|n| AccumulatorDeclaration {
                name: n.to_string(),
                factory: Arc::new(PassthroughAccumulatorFactory),
            })
            .collect();
        scheduler
            .load_reactor(
                reactor_name.to_string(),
                acc_decls,
                ReactionCriteria::WhenAny,
                InputStrategy::Latest,
                Some("public".to_string()),
                vec![],
            )
            .await
            .expect("publishing reactor should load");
    }

    fn make_reconciler_with_scheduler(
        scheduler: Arc<ComputationGraphScheduler>,
    ) -> RegistryReconciler {
        let registry = Arc::new(FilesystemWorkflowRegistry::new(vec![]));
        let config = ReconcilerConfig::default();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let reconciler = RegistryReconciler::new(registry, config, rx)
            .expect("Failed to create test reconciler")
            .with_runtime(Arc::new(Runtime::empty()));
        reconciler.with_graph_scheduler(scheduler)
    }

    #[tokio::test]
    #[serial]
    async fn cross_package_contract_mismatch_rejects_with_named_accumulators() {
        let endpoint_registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
        load_publishing_reactor_into_scheduler(&scheduler, "shared_rx", &["alpha", "beta"]).await;

        let reconciler = make_reconciler_with_scheduler(scheduler.clone());
        let metadata = WorkflowMetadata {
            package_name: "subscriber-mismatched".to_string(),
            ..make_test_metadata()
        };
        let view = make_test_view_with_subscriber_graph(
            "subscriber-mismatched",
            "shared_rx",
            vec!["alpha", "gamma"],
        );
        let manifest = make_cloacina_metadata_with_triggers(vec![]);

        let err = reconciler
            .step_load_reactor_bound_cgs(&metadata, &view, &manifest, b"")
            .await
            .expect_err("subscriber declaring accumulator outside upstream contract must error");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("subscriber-mismatched"),
            "error must name the offending package: {}",
            msg
        );
        assert!(
            msg.contains("shared_rx"),
            "error must name the upstream reactor: {}",
            msg
        );
        assert!(
            msg.contains("gamma"),
            "error must name the missing accumulator: {}",
            msg
        );

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    #[serial]
    async fn cross_package_subscriber_before_publisher_rejects_with_clear_error() {
        let endpoint_registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
        // No publisher loaded; subscriber arrives first.

        let reconciler = make_reconciler_with_scheduler(scheduler.clone());
        let metadata = WorkflowMetadata {
            package_name: "subscriber-orphan".to_string(),
            ..make_test_metadata()
        };
        let view =
            make_test_view_with_subscriber_graph("subscriber-orphan", "missing_rx", vec!["alpha"]);
        let manifest = make_cloacina_metadata_with_triggers(vec![]);

        let err = reconciler
            .step_load_reactor_bound_cgs(&metadata, &view, &manifest, b"")
            .await
            .expect_err("subscriber-before-publisher must error fast, no pending bindings");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("subscriber-orphan"),
            "error names package: {}",
            msg
        );
        assert!(
            msg.contains("missing_rx"),
            "error names the missing reactor: {}",
            msg
        );
        assert!(
            msg.contains("publishing package must load before"),
            "error suggests load-order remediation: {}",
            msg
        );

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    #[serial]
    async fn cross_package_subscriber_in_same_package_skips_validation() {
        // When the subscriber declares its own reactor (publisher is in the
        // same package), the cross-package pre-validation must NOT fire —
        // the reactor has not been loaded yet at the moment step_load_
        // reactor_bound_cgs runs (step 3 ran earlier in the pipeline, but
        // the test path here does not include step 3). The validation must
        // detect "publisher_in_same_package" via view.reactors and skip.
        use cloacina_workflow_plugin::{
            types::AccumulatorDeclarationEntry, GraphPackageMetadata, ReactorPackageMetadata,
        };
        let endpoint_registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
        let reconciler = make_reconciler_with_scheduler(scheduler.clone());

        let metadata = WorkflowMetadata {
            package_name: "self-publisher".to_string(),
            ..make_test_metadata()
        };
        let view = PackageLoadView {
            tasks: crate::registry::loader::package_loader::PackageMetadata {
                package_name: "self-publisher".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                tasks: vec![],
                graph_data: None,
                architecture: String::new(),
                symbols: vec![],
                workflow_triggers: vec![],
            },
            triggers: vec![],
            reactors: vec![ReactorPackageMetadata {
                name: "self_rx".to_string(),
                package_name: "self-publisher".to_string(),
                reaction_mode: "when_any".to_string(),
                accumulators: vec![],
            }],
            graph: Some(GraphPackageMetadata {
                graph_name: "self_graph".to_string(),
                package_name: "self-publisher".to_string(),
                reaction_mode: "when_any".to_string(),
                input_strategy: "latest".to_string(),
                accumulators: vec![AccumulatorDeclarationEntry {
                    name: "any_acc".to_string(),
                    accumulator_type: "passthrough".to_string(),
                    config: Default::default(),
                }],
                trigger_reactor: Some("self_rx".to_string()),
            }),
        };
        let manifest = make_cloacina_metadata_with_triggers(vec![]);

        // Even though the upstream "self_rx" has not been loaded into
        // the scheduler, the validation must skip because the publisher
        // appears in view.reactors. The downstream `load_graph` call
        // will spin its own reactor instance via the bundled-form path.
        let result = reconciler
            .step_load_reactor_bound_cgs(&metadata, &view, &manifest, b"")
            .await;
        assert!(
            result.is_ok(),
            "self-publishing graph must skip cross-package validation; got {:?}",
            result
        );

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    #[serial]
    async fn unload_package_rejects_when_subscribers_remain_bound() {
        // Reverse-order unload pipeline with T-0544 M4 reject-with-bound-
        // subscribers guard: a publisher package owns reactor R; a
        // subscriber from a different package binds to R. Attempting to
        // unload the publisher first must surface a clean
        // RegistrationFailed naming the publisher + R.
        use crate::registry::reconciler::PackageState;
        use crate::registry::types::WorkflowPackageId;

        let endpoint_registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
        load_publishing_reactor_into_scheduler(&scheduler, "owned_rx", &["alpha"]).await;

        // Bind a subscriber graph from another package to the reactor
        // (simulating the cross-package fan-out path).
        let graph_fn: crate::computation_graph::reactor::CompiledGraphFn = Arc::new(|_cache| {
            Box::pin(async { cloacina_computation_graph::GraphResult::completed(vec![]) })
        });
        scheduler
            .bind_graph_to_reactor(
                "subscriber_graph".to_string(),
                "owned_rx".to_string(),
                graph_fn,
            )
            .await
            .expect("subscriber should bind to publisher's reactor");

        let reconciler = make_reconciler_with_scheduler(scheduler.clone());

        // Manually insert publisher PackageState (the package owns
        // owned_rx via reactor_names).
        let publisher_id: WorkflowPackageId = uuid::Uuid::new_v4();
        let publisher_state = PackageState {
            metadata: WorkflowMetadata {
                package_name: "publisher-pkg".to_string(),
                ..make_test_metadata()
            },
            task_namespaces: vec![],
            workflow_name: None,
            trigger_names: vec![],
            graph_name: None,
            reactor_names: vec!["owned_rx".to_string()],
        };
        reconciler
            .loaded_packages
            .write()
            .await
            .insert(publisher_id, publisher_state);

        let err = reconciler
            .unload_package(publisher_id)
            .await
            .expect_err("unload must reject while subscribers remain bound");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("publisher-pkg"),
            "error must name the publisher package: {}",
            msg
        );
        assert!(
            msg.contains("owned_rx"),
            "error must name the reactor with bound subscribers: {}",
            msg
        );
        assert!(
            msg.contains("unload subscribers first"),
            "error must instruct operator on unload order: {}",
            msg
        );

        // Publisher PackageState was already removed even though the
        // unload errored — the registrar drop is a separate concern and
        // running the rest of the unload steps is fine. Sanity-check
        // that the reactor is still loaded.
        assert!(
            scheduler
                .reactor_accumulator_names("owned_rx")
                .await
                .is_some(),
            "reactor must still exist after rejected unload"
        );

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    #[serial]
    async fn unload_package_succeeds_after_subscribers_unbound() {
        // Companion to the rejection test: once subscribers are unbound,
        // the publisher's unload completes cleanly and the reactor is
        // torn down.
        use crate::registry::reconciler::PackageState;
        use crate::registry::types::WorkflowPackageId;

        let endpoint_registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
        load_publishing_reactor_into_scheduler(&scheduler, "lone_rx", &["alpha"]).await;

        let reconciler = make_reconciler_with_scheduler(scheduler.clone());

        let publisher_id: WorkflowPackageId = uuid::Uuid::new_v4();
        reconciler.loaded_packages.write().await.insert(
            publisher_id,
            PackageState {
                metadata: WorkflowMetadata {
                    package_name: "publisher-lone".to_string(),
                    ..make_test_metadata()
                },
                task_namespaces: vec![],
                workflow_name: None,
                trigger_names: vec![],
                graph_name: None,
                reactor_names: vec!["lone_rx".to_string()],
            },
        );

        reconciler
            .unload_package(publisher_id)
            .await
            .expect("unload with no subscribers must succeed");

        assert!(
            scheduler
                .reactor_accumulator_names("lone_rx")
                .await
                .is_none(),
            "reactor must be torn down after publisher unload"
        );

        scheduler.shutdown_all().await;
    }
}
