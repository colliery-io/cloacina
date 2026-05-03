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

        // Track the loaded package state
        let package_state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
            trigger_names,
            graph_name,
        };

        let mut loaded_packages = self.loaded_packages.write().await;
        loaded_packages.insert(metadata.id, package_state);
        drop(loaded_packages);

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

        Ok(())
    }

    /// Unload a package from the global registries
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

        // Tell the task registrar (which owns dlopen handles) to drop the
        // package so the cdylib is unloaded and any cached state is released.
        let package_id_str = package_id.to_string();
        self.task_registrar
            .unregister_package_tasks(&package_id_str)
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to unregister package tasks: {}", e),
            })?;

        // Mirror removals through the runtime so executors drop the stale
        // entries immediately.
        if let Some(runtime) = &self.runtime {
            for ns in &package_state.task_namespaces {
                runtime.unregister_task(ns);
            }
            if let Some(workflow_name) = &package_state.workflow_name {
                runtime.unregister_workflow(workflow_name);
            }
            for trigger_name in &package_state.trigger_names {
                runtime.unregister_trigger(trigger_name);
            }
            if let Some(graph_name) = &package_state.graph_name {
                runtime.unregister_computation_graph(graph_name);
            }
        }

        // Unload computation graph from graph scheduler
        if let Some(graph_name) = &package_state.graph_name {
            let scheduler_guard = self.graph_scheduler.read().await;
            if let Some(ref scheduler) = *scheduler_guard {
                if let Err(e) = scheduler.unload_graph(graph_name).await {
                    warn!("Failed to unload computation graph '{}': {}", graph_name, e);
                }
            }
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
}
