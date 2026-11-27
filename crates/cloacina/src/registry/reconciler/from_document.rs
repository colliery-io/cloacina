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

//! Load a package from its metadata document (CLOACINA-T-1075).
//!
//! On a server, tenant code runs only on the tenant's agents and in the
//! tenant's compiler (CLOACINA-A-0019). Opening a package's library runs its
//! load-time code, and importing a Python package runs the module, so the
//! server must not do either to learn what a package holds. The tenant's
//! compiler describes each package in a child process and stores the
//! document (CLOACINA-T-1074); this module registers the package from it.
//!
//! What is registered, from the document alone:
//! - each task, as a [`DescribedTask`]: its id, dependencies, trigger rules,
//!   retry policy, time limit and placement, and no code. An executor that
//!   sends work to agents reads these and sends the task to an agent, which
//!   loads the package and runs it;
//! - the workflow, built from those tasks;
//! - each cron schedule (an expression, a timezone and a policy: data);
//! - each reactor, whose accumulators are code of this crate;
//! - the graph of a Rust package, with a graph function that opens nothing
//!   and fails a firing it is asked to run: the graph executor sends each
//!   firing to an agent.
//!
//! A package whose code would have to run in this process is refused, with
//! the reason: a poll trigger (its code runs at each poll), a native provider
//! (it is a library opened in this process), and Python reactive code (the
//! accumulators, reactors and graph of a Python package are Python functions).

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{info, warn};

use super::loading::PackageLoadView;
use super::{PackageState, RegistryReconciler};
use crate::context::Context;
use crate::error::TaskError;
use crate::registry::error::RegistryError;
use crate::registry::package_description::{PackageDescription, TaskDescription};
use crate::registry::types::WorkflowMetadata;
use crate::registry::workflow_registry::StoredDescription;
use crate::retry::RetryPolicy;
use crate::task::{Task, TaskNamespace};

/// The error a graph firing gets when this process is asked to run it
/// (CLOACINA-T-1075): a server that loads from metadata has no code of the
/// graph.
pub(super) fn graph_runs_on_agents_reason(package_name: &str, graph_name: &str) -> String {
    format!(
        "graph '{graph_name}' of package '{package_name}' runs only on the tenant's agents: \
         this server loads packages from their metadata and runs no tenant code; the firing \
         could not be sent to an agent"
    )
}

/// A task of a package that was loaded from its metadata document
/// (CLOACINA-T-1075). It holds what the scheduler and an executor that sends
/// work to agents read from a task, and no code: [`Task::execute`] fails
/// with a reason that says where the task runs.
#[derive(Debug, Clone)]
pub struct DescribedTask {
    id: String,
    package_name: String,
    dependencies: Vec<TaskNamespace>,
    trigger_rules: serde_json::Value,
    requires_handle: bool,
    retry_policy: RetryPolicy,
    timeout: Option<Duration>,
    placement: cloacina_workflow::TaskPlacement,
    code_fingerprint: Option<String>,
}

impl DescribedTask {
    /// The task of `description`, with its dependencies already made into
    /// namespaces.
    pub fn from_description(
        package_name: &str,
        description: &TaskDescription,
        dependencies: Vec<TaskNamespace>,
    ) -> Self {
        Self {
            id: description.id.clone(),
            package_name: package_name.to_string(),
            dependencies,
            trigger_rules: description.trigger_rules.clone(),
            requires_handle: description.requires_handle,
            retry_policy: description.retry_policy.clone(),
            timeout: description.timeout_seconds.map(Duration::from_secs),
            placement: description.placement(),
            code_fingerprint: description.code_fingerprint.clone(),
        }
    }

    /// A constructor node of a package: a task with the default policies.
    pub fn constructor_node(
        package_name: &str,
        id: &str,
        dependencies: Vec<TaskNamespace>,
    ) -> Self {
        Self {
            id: id.to_string(),
            package_name: package_name.to_string(),
            dependencies,
            trigger_rules: serde_json::json!({ "type": "Always" }),
            requires_handle: false,
            retry_policy: RetryPolicy::default(),
            timeout: None,
            placement: cloacina_workflow::TaskPlacement::default(),
            code_fingerprint: None,
        }
    }
}

#[async_trait::async_trait]
impl Task for DescribedTask {
    async fn execute(
        &self,
        _context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        Err(TaskError::ExecutionFailed {
            task_id: self.id.clone(),
            message: format!(
                "task '{}' of package '{}' runs only on the tenant's agents: this server \
                 loads packages from their metadata and runs no tenant code. Send work to \
                 the agent fleet (--default-executor fleet)",
                self.id, self.package_name
            ),
            timestamp: Utc::now(),
        })
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }

    fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy.clone()
    }

    fn trigger_rules(&self) -> serde_json::Value {
        self.trigger_rules.clone()
    }

    fn code_fingerprint(&self) -> Option<String> {
        self.code_fingerprint.clone()
    }

    fn requires_handle(&self) -> bool {
        self.requires_handle
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    fn placement(&self) -> cloacina_workflow::TaskPlacement {
        self.placement.clone()
    }
}

/// What a package uses that would run its code in this process, each named
/// (CLOACINA-T-1075). Empty: the package loads from its document.
///
/// - a poll trigger: a trigger with no cron expression;
/// - a native provider: a bundled provider whose `provider.json` declares
///   `runtime = "native"` (`native_providers`);
/// - Python reactive code: a reactor, a graph or an accumulator of a Python
///   package.
pub fn code_needed_in_process(
    document: &PackageDescription,
    manifest: &cloacina_workflow_plugin::CloacinaMetadata,
    native_providers: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();

    let mut poll: Vec<&str> = document
        .triggers
        .iter()
        .filter(|t| t.cron_expression.is_none())
        .map(|t| t.name.as_str())
        .collect();
    poll.sort_unstable();
    if !poll.is_empty() {
        reasons.push(format!(
            "it declares the poll trigger(s) {}, whose code runs at each poll",
            quoted(&poll)
        ));
    }

    if !native_providers.is_empty() {
        let names: Vec<&str> = native_providers.iter().map(String::as_str).collect();
        reasons.push(format!(
            "it bundles the native provider(s) {}, which are libraries opened in the \
             process that uses them",
            quoted(&names)
        ));
    }

    if document.language == "python" {
        let mut parts = Vec::new();
        let reactors: Vec<&str> = document.reactors.iter().map(|r| r.name.as_str()).collect();
        if !reactors.is_empty() {
            parts.push(format!("the reactor(s) {}", quoted(&reactors)));
        }
        let graph = document
            .graph
            .as_ref()
            .map(|g| g.graph_name.as_str())
            .or(manifest.graph_name.as_deref());
        if let Some(graph) = graph {
            parts.push(format!("the graph {}", quoted(&[graph])));
        }
        let accumulators: Vec<&str> = manifest
            .accumulators
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        if parts.is_empty() && !accumulators.is_empty() {
            parts.push(format!("the accumulator(s) {}", quoted(&accumulators)));
        }
        if !parts.is_empty() {
            reasons.push(format!(
                "it holds Python reactive code ({}), which runs in the process that loads it",
                parts.join(", ")
            ));
        }
    }

    reasons
}

fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The namespace of a dependency of a packaged task, read the way the task
/// registrar reads it: a local id, or a namespace whose `{tenant}` is filled
/// in.
fn dependency_namespace(
    dependency: &str,
    tenant: &str,
    package_name: &str,
    workflow_name: &str,
) -> Result<TaskNamespace, RegistryError> {
    if dependency.contains("::") {
        let full = dependency.replace("{tenant}", tenant);
        crate::parse_namespace(&full).map_err(|e| RegistryError::RegistrationFailed {
            message: format!("invalid dependency namespace '{full}': {e}"),
        })
    } else {
        Ok(TaskNamespace::new(
            tenant,
            package_name,
            workflow_name,
            dependency,
        ))
    }
}

impl RegistryReconciler {
    /// Register a package from its metadata document (CLOACINA-T-1075). See
    /// the module docs. Schedule ids it creates go into
    /// `created_schedule_ids`, as for the in-process load, so a failure
    /// later in the load rolls them back.
    pub(super) async fn load_package_from_document(
        &self,
        metadata: &WorkflowMetadata,
        manifest: &cloacina_workflow_plugin::CloacinaMetadata,
        created_schedule_ids: &mut Vec<String>,
    ) -> Result<(), RegistryError> {
        let document = self.stored_document(metadata).await?;

        // The bundled providers are unpacked to read their manifests. Nothing
        // in them is opened.
        let provider_root = self.stage_bundled_providers(metadata).await?;
        let native_providers = native_providers_in(provider_root.as_deref())?;
        let reasons = code_needed_in_process(&document, manifest, &native_providers);
        if !reasons.is_empty() {
            return Err(self.refuse_for_code(metadata, &reasons).await);
        }

        let runtime = self
            .runtime
            .clone()
            .ok_or_else(|| RegistryError::RegistrationFailed {
                message: format!(
                    "package {} received but the reconciler has no Runtime attached",
                    metadata.package_name
                ),
            })?;

        let view = PackageLoadView {
            triggers: document.triggers.clone(),
            reactors: document.reactors.clone(),
            graph: document.graph.clone(),
        };

        // Cron schedules: data.
        created_schedule_ids.extend(self.step_load_cron_triggers(metadata, &view).await?);
        // Reactors: their accumulators are code of this crate.
        self.step_load_reactors(metadata, &view, manifest, provider_root.as_deref())
            .await?;
        // The graph of a Rust package, with no code behind it. A Python
        // package with a graph was refused above.
        let graph_name = if document.language == "rust" {
            self.step_load_reactor_bound_cgs(
                metadata,
                &view,
                manifest,
                None,
                provider_root.as_deref(),
            )
            .await?
        } else {
            None
        };

        // Tasks and the workflow.
        let (task_namespaces, workflow_name) =
            self.register_described_tasks(metadata, &document, &runtime)?;
        let unregister = |runtime: &Arc<crate::Runtime>| {
            for ns in &task_namespaces {
                runtime.unregister_task(ns);
            }
        };
        if let Err(refused) = self.check_task_placement(metadata, &task_namespaces).await {
            unregister(&runtime);
            return Err(refused);
        }
        if let Some(workflow_name) = workflow_name.as_deref()
            && let Err(e) = self.register_workflow_of_package(metadata, workflow_name)
        {
            unregister(&runtime);
            return Err(e);
        }
        if let Err(e) = self.validate_subscriptions_from_document(metadata, &document, &runtime) {
            if let Some(workflow_name) = workflow_name.as_deref() {
                runtime.unregister_workflow(workflow_name);
            }
            unregister(&runtime);
            return Err(e);
        }

        // The task list the API and the UI show. A Rust package's list was
        // written with its build; a Python package's was written at its load
        // before the document existed. Best effort, as there.
        if document.language == "python" && !document.tasks.is_empty() {
            let graph: Vec<(String, Vec<String>)> = document
                .tasks
                .iter()
                .map(|t| (t.id.clone(), t.dependencies.clone()))
                .collect();
            if let Err(e) = self.registry.persist_task_graph(metadata.id, graph).await {
                warn!(
                    package = %metadata.package_name,
                    error = %e,
                    "failed to persist the task graph of a package loaded from its document"
                );
            }
        }

        info!(
            "Package {} v{} loaded from its metadata document: {} task(s), workflow {:?}, \
             {} cron schedule(s), {} reactor(s), graph {:?}; no tenant code was opened",
            metadata.package_name,
            metadata.version,
            task_namespaces.len(),
            workflow_name,
            created_schedule_ids.len(),
            document.reactors.len(),
            graph_name,
        );

        let state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
            trigger_names: Vec::new(),
            graph_name,
            reactor_names: document.reactors.iter().map(|r| r.name.clone()).collect(),
            cron_schedule_ids: created_schedule_ids.clone(),
            triggerless_graph_names: Vec::new(),
        };
        self.loaded_packages
            .write()
            .await
            .insert(metadata.id, state);
        Ok(())
    }

    /// The document of a package, or the reason it is not loaded. A package
    /// with no document is not changed: the compiler describes it, or has
    /// recorded why it could not.
    async fn stored_document(
        &self,
        metadata: &WorkflowMetadata,
    ) -> Result<PackageDescription, RegistryError> {
        let stored = self.registry.get_package_description(metadata.id).await?;
        match stored {
            Some(StoredDescription {
                document: Some(document),
                ..
            }) => Ok(document),
            Some(StoredDescription {
                document: None,
                describe_error: Some(reason),
                ..
            }) => Err(RegistryError::ValidationError {
                reason: format!(
                    "package {} v{} is not loaded: its compiler could not describe it: {}. \
                     This server loads packages from their metadata only; build the package \
                     again, or run the server with --single-trust",
                    metadata.package_name, metadata.version, reason
                ),
            }),
            _ => Err(RegistryError::ValidationError {
                reason: format!(
                    "package {} v{} is not loaded: it has no metadata document yet. This \
                     server loads packages from their metadata only; the tenant's compiler \
                     describes the package on its next pass",
                    metadata.package_name, metadata.version
                ),
            }),
        }
    }

    /// Refuse a package whose code would run in this process: the reason
    /// goes where a failed build is read (build status `failed`,
    /// `build_error`), as for a placement refusal (CLOACINA-T-1027).
    async fn refuse_for_code(
        &self,
        metadata: &WorkflowMetadata,
        reasons: &[String],
    ) -> RegistryError {
        let message = format!(
            "package {} v{} is refused: {}. This server loads packages from their metadata \
             and runs no tenant code (CLOACINA-A-0019); the package loads when this code runs \
             on agents, or on a server with --single-trust",
            metadata.package_name,
            metadata.version,
            reasons.join("; ")
        );
        warn!("{}", message);
        match self.registry.refuse_package(metadata.id, &message).await {
            Ok(_) => {}
            Err(e) => warn!(
                "could not record the refusal of package {} v{}: {}",
                metadata.package_name, metadata.version, e
            ),
        }
        RegistryError::ValidationError { reason: message }
    }

    /// Register a [`DescribedTask`] for each task and constructor node of
    /// the document. Returns the namespaces and the workflow name (`None`
    /// for a package with no task).
    fn register_described_tasks(
        &self,
        metadata: &WorkflowMetadata,
        document: &PackageDescription,
        runtime: &Arc<crate::Runtime>,
    ) -> Result<(Vec<TaskNamespace>, Option<String>), RegistryError> {
        if document.tasks.is_empty() && document.constructors.is_empty() {
            return Ok((Vec::new(), None));
        }
        let tenant = self.config.default_tenant_id.as_str();
        let package_name = metadata.package_name.as_str();
        let workflow_name = if document.workflow_name.is_empty() {
            package_name.to_string()
        } else {
            document.workflow_name.clone()
        };

        let mut entries: Vec<(TaskNamespace, DescribedTask)> = Vec::new();
        for task in &document.tasks {
            let dependencies = task
                .dependencies
                .iter()
                .map(|d| dependency_namespace(d, tenant, package_name, &workflow_name))
                .collect::<Result<Vec<_>, _>>()?;
            let ns = TaskNamespace::new(tenant, package_name, &workflow_name, &task.id);
            entries.push((
                ns,
                DescribedTask::from_description(package_name, task, dependencies),
            ));
        }
        for node in &document.constructors {
            let workflow = if node.workflow.is_empty() {
                workflow_name.as_str()
            } else {
                node.workflow.as_str()
            };
            let dependencies = node
                .dependencies
                .iter()
                .map(|d| TaskNamespace::new(tenant, package_name, workflow, d))
                .collect();
            let ns = TaskNamespace::new(tenant, package_name, workflow, &node.id);
            entries.push((
                ns,
                DescribedTask::constructor_node(package_name, &node.id, dependencies),
            ));
        }

        let mut namespaces = Vec::with_capacity(entries.len());
        for (ns, task) in entries {
            let task = Arc::new(task);
            runtime.register_task(ns.clone(), move || task.clone() as Arc<dyn Task>);
            namespaces.push(ns);
        }
        Ok((namespaces, Some(workflow_name)))
    }

    /// Each trigger the workflow subscribes to must be a trigger of the
    /// document or one in the runtime, as the in-process load checks.
    fn validate_subscriptions_from_document(
        &self,
        metadata: &WorkflowMetadata,
        document: &PackageDescription,
        runtime: &Arc<crate::Runtime>,
    ) -> Result<(), RegistryError> {
        let missing: Vec<&String> = document
            .workflow_triggers
            .iter()
            .filter(|name| {
                runtime.get_trigger(name).is_none()
                    && !document.triggers.iter().any(|t| &t.name == *name)
            })
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(RegistryError::RegistrationFailed {
            message: format!(
                "Package {} v{} declares `#[workflow(triggers = [...])]` referencing \
                 trigger(s) not registered in this runtime: {:?}. Ensure the package \
                 declaring those triggers is loaded first, or remove the typo.",
                metadata.package_name, metadata.version, missing
            ),
        })
    }
}

/// The native providers of a staged bundle. Without `constructors-wasm` no
/// bundle can be staged (staging a package that bundles any provider fails
/// closed), so there is none.
fn native_providers_in(root: Option<&std::path::Path>) -> Result<Vec<String>, RegistryError> {
    let Some(root) = root else {
        return Ok(Vec::new());
    };
    #[cfg(feature = "constructors-wasm")]
    {
        crate::registry::loader::constructor_loader::native_provider_names(root)
            .map_err(RegistryError::Loader)
    }
    #[cfg(not(feature = "constructors-wasm"))]
    {
        let _ = root;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::reconciler::{PackageLoadMode, ReconcilerConfig};
    use crate::registry::traits::WorkflowRegistry;
    use crate::registry::types::{LoadedWorkflow, WorkflowPackageId};
    use async_trait::async_trait;
    use cloacina_workflow_plugin::{ReactorPackageMetadata, TriggerPackageMetadata};
    use std::sync::Mutex;

    /// A task list as `persist_task_graph` takes it.
    type TaskGraph = Vec<(String, Vec<String>)>;

    /// A registry that holds one stored document and records refusals and
    /// the task graph a load writes.
    #[derive(Default)]
    struct DocumentRegistry {
        stored: Mutex<Option<StoredDescription>>,
        refused: Mutex<Vec<(WorkflowPackageId, String)>>,
        task_graphs: Mutex<Vec<(WorkflowPackageId, TaskGraph)>>,
    }

    #[async_trait]
    impl WorkflowRegistry for DocumentRegistry {
        async fn register_workflow(
            &mut self,
            _package_data: Vec<u8>,
        ) -> Result<WorkflowPackageId, RegistryError> {
            unimplemented!("stub")
        }
        async fn get_workflow(
            &self,
            _package_name: &str,
            _version: &str,
        ) -> Result<Option<LoadedWorkflow>, RegistryError> {
            Ok(None)
        }
        async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
            Ok(vec![])
        }
        async fn unregister_workflow(
            &mut self,
            _package_name: &str,
            _version: &str,
        ) -> Result<(), RegistryError> {
            Ok(())
        }
        async fn refuse_package(
            &self,
            package_id: WorkflowPackageId,
            reason: &str,
        ) -> Result<bool, RegistryError> {
            self.refused
                .lock()
                .unwrap()
                .push((package_id, reason.to_string()));
            Ok(true)
        }
        async fn persist_task_graph(
            &self,
            package_id: WorkflowPackageId,
            tasks: Vec<(String, Vec<String>)>,
        ) -> Result<(), RegistryError> {
            self.task_graphs.lock().unwrap().push((package_id, tasks));
            Ok(())
        }
        async fn get_package_description(
            &self,
            _package_id: WorkflowPackageId,
        ) -> Result<Option<StoredDescription>, RegistryError> {
            Ok(self.stored.lock().unwrap().clone())
        }
    }

    /// A cron registrar that records each schedule.
    #[derive(Default)]
    struct Schedules {
        cron: Mutex<Vec<(String, String, String)>>,
    }

    #[async_trait]
    impl super::super::CronWorkflowRegistrar for Schedules {
        async fn register_cron_workflow(
            &self,
            workflow_name: &str,
            cron_expression: &str,
            timezone: &str,
        ) -> Result<String, String> {
            let mut cron = self.cron.lock().unwrap();
            cron.push((
                workflow_name.to_string(),
                cron_expression.to_string(),
                timezone.to_string(),
            ));
            Ok(format!("schedule-{}", cron.len()))
        }
        async fn unregister_cron_workflow(&self, _schedule_id: &str) -> Result<(), String> {
            Ok(())
        }
    }

    fn reconciler(
        tenant: &str,
        registry: Arc<DocumentRegistry>,
        schedules: Arc<Schedules>,
    ) -> RegistryReconciler {
        let config = ReconcilerConfig {
            default_tenant_id: tenant.to_string(),
            package_load_mode: PackageLoadMode::MetadataOnly,
            ..ReconcilerConfig::default()
        };
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let mut reconciler = RegistryReconciler::new(registry, config, rx)
            .expect("create reconciler")
            .with_runtime(Arc::new(crate::Runtime::empty()))
            .with_cron_registrar(schedules);
        reconciler.placement_check = None;
        reconciler
    }

    fn metadata(package_name: &str) -> WorkflowMetadata {
        WorkflowMetadata {
            id: uuid::Uuid::new_v4(),
            registry_id: uuid::Uuid::new_v4(),
            workflow_name: "wf".to_string(),
            package_name: package_name.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            tasks: vec![],
            task_graph: vec![],
            schedules: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            paused: false,
            declared_params: vec![],
            declared_surfaces: vec![],
            workflow_triggers: vec![],
        }
    }

    fn manifest(language: &str) -> cloacina_workflow_plugin::CloacinaMetadata {
        cloacina_workflow_plugin::CloacinaMetadata {
            workflow_name: None,
            graph_name: None,
            language: language.to_string(),
            description: None,
            author: None,
            requires_python: None,
            entry_module: None,
            reaction_mode: None,
            input_strategy: None,
            accumulators: Vec::new(),
            providers: Default::default(),
            vendored: None,
        }
    }

    fn task(id: &str, deps: &[&str]) -> TaskDescription {
        TaskDescription {
            index: 0,
            id: id.to_string(),
            namespaced_id_template: format!("{{tenant}}::pkg::wf::{id}"),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            description: String::new(),
            source_location: String::new(),
            trigger_rules: serde_json::json!({ "type": "Always" }),
            requires_handle: false,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: None,
            pool: None,
            cpu: None,
            memory: None,
            inline: false,
            code_fingerprint: None,
            doc_what: None,
            doc_why: None,
            priority: None,
            workflow_priority: None,
        }
    }

    fn trigger(name: &str, cron: Option<&str>) -> TriggerPackageMetadata {
        TriggerPackageMetadata {
            name: name.to_string(),
            workflow_name: "wf".to_string(),
            package_name: "pkg".to_string(),
            poll_interval: "60s".to_string(),
            cron_expression: cron.map(str::to_string),
            allow_concurrent: false,
            timezone: Some("Europe/Paris".to_string()),
            catchup_policy: None,
            max_active_runs: None,
        }
    }

    fn document(language: &str, tasks: Vec<TaskDescription>) -> PackageDescription {
        let mut value = serde_json::json!({
            "format_version": crate::registry::PACKAGE_DESCRIPTION_VERSION,
            "language": language,
            "described_by": "test",
            "package_name": "pkg",
            "workflow_name": "wf",
        });
        value["tasks"] = serde_json::to_value(tasks).unwrap();
        serde_json::from_value(value).unwrap()
    }

    fn stored(document: PackageDescription) -> StoredDescription {
        StoredDescription {
            version: Some(document.format_version),
            document: Some(document),
            describe_error: None,
        }
    }

    async fn load(
        reconciler: &RegistryReconciler,
        metadata: &WorkflowMetadata,
        language: &str,
    ) -> Result<Vec<String>, RegistryError> {
        let mut ids = Vec::new();
        reconciler
            .load_package_from_document(metadata, &manifest(language), &mut ids)
            .await
            .map(|()| ids)
    }

    /// The tasks, the workflow and the cron schedule of a package exist after
    /// a load from the document alone, with each task's own policies.
    #[tokio::test]
    async fn tasks_workflow_and_schedules_are_registered_from_the_document() {
        let registry = Arc::new(DocumentRegistry::default());
        let schedules = Arc::new(Schedules::default());
        let reconciler = reconciler("acme", registry.clone(), schedules.clone());
        let meta = metadata("pkg");

        let mut load_task = task("load", &["extract"]);
        load_task.retry_policy = RetryPolicy::builder().max_attempts(0).build();
        load_task.timeout_seconds = Some(600);
        load_task.pool = Some("large".to_string());
        load_task.memory = Some("8Gi".to_string());
        load_task.trigger_rules = serde_json::json!({ "type": "All", "conditions": [] });
        load_task.requires_handle = true;
        let mut doc = document("rust", vec![task("extract", &[]), load_task]);
        doc.triggers = vec![trigger("nightly", Some("0 2 * * *"))];
        *registry.stored.lock().unwrap() = Some(stored(doc));

        let ids = load(&reconciler, &meta, "rust").await.expect("loads");
        assert_eq!(ids, vec!["schedule-1".to_string()]);
        assert_eq!(
            schedules.cron.lock().unwrap().as_slice(),
            &[(
                "wf".to_string(),
                "0 2 * * *".to_string(),
                "Europe/Paris".to_string()
            )]
        );

        let runtime = reconciler.runtime.clone().unwrap();
        let ns = TaskNamespace::new("acme", "pkg", "wf", "load");
        let loaded = runtime.get_task(&ns).expect("task registered");
        assert_eq!(loaded.retry_policy().max_attempts, 0);
        assert_eq!(loaded.timeout(), Some(Duration::from_secs(600)));
        assert_eq!(loaded.placement().pool_name(), "large");
        assert_eq!(loaded.trigger_rules()["type"], "All");
        assert!(loaded.requires_handle());
        assert_eq!(
            loaded.dependencies(),
            &[TaskNamespace::new("acme", "pkg", "wf", "extract")]
        );

        let workflow = runtime.get_workflow("wf").expect("workflow registered");
        assert_eq!(workflow.topological_sort().unwrap().len(), 2);

        // A task with no code fails with where it runs.
        let err = loaded
            .execute(Context::new())
            .await
            .expect_err("no code in this process");
        assert!(err.to_string().contains("runs only on the tenant's agents"));

        let loaded_packages = reconciler.loaded_packages.read().await;
        let state = loaded_packages.get(&meta.id).expect("tracked");
        assert_eq!(state.task_namespaces.len(), 2);
        assert_eq!(state.workflow_name.as_deref(), Some("wf"));
        assert!(registry.refused.lock().unwrap().is_empty());
    }

    /// A package of `public` is loaded the same way as one of a named tenant.
    #[tokio::test]
    async fn a_package_of_public_loads_the_same_way() {
        for tenant in ["public", "acme"] {
            let registry = Arc::new(DocumentRegistry::default());
            let reconciler = reconciler(tenant, registry.clone(), Arc::default());
            let meta = metadata("pkg");
            *registry.stored.lock().unwrap() =
                Some(stored(document("rust", vec![task("only", &[])])));
            load(&reconciler, &meta, "rust").await.expect("loads");
            let runtime = reconciler.runtime.clone().unwrap();
            let loaded = runtime
                .get_task(&TaskNamespace::new(tenant, "pkg", "wf", "only"))
                .expect("task registered");
            assert!(
                loaded
                    .execute(Context::new())
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("runs only on the tenant's agents"),
                "tenant {tenant}"
            );
        }
    }

    /// A package with no document, or one its compiler could not describe,
    /// does not load, and its row is not changed.
    #[tokio::test]
    async fn a_package_with_no_document_does_not_load() {
        let registry = Arc::new(DocumentRegistry::default());
        let reconciler = reconciler("acme", registry.clone(), Arc::default());
        let meta = metadata("pkg");

        let err = load(&reconciler, &meta, "rust").await.unwrap_err();
        assert!(
            err.to_string().contains("has no metadata document yet"),
            "{err}"
        );

        *registry.stored.lock().unwrap() = Some(StoredDescription {
            document: None,
            version: Some(1),
            describe_error: Some("the library does not load".to_string()),
        });
        let err = load(&reconciler, &meta, "rust").await.unwrap_err();
        let text = err.to_string();
        assert!(text.contains("could not describe it"), "{text}");
        assert!(text.contains("the library does not load"), "{text}");

        assert!(registry.refused.lock().unwrap().is_empty());
        assert!(reconciler.loaded_packages.read().await.is_empty());
        assert!(
            reconciler
                .runtime
                .clone()
                .unwrap()
                .task_namespaces()
                .is_empty()
        );
    }

    /// A package with a poll trigger is refused, with the trigger named, and
    /// the refusal is recorded; nothing is registered.
    #[tokio::test]
    async fn a_poll_trigger_refuses_the_package() {
        let registry = Arc::new(DocumentRegistry::default());
        let schedules = Arc::new(Schedules::default());
        let reconciler = reconciler("acme", registry.clone(), schedules.clone());
        let meta = metadata("pkg");
        let mut doc = document("rust", vec![task("t", &[])]);
        doc.triggers = vec![
            trigger("nightly", Some("0 2 * * *")),
            trigger("watch_inbox", None),
        ];
        *registry.stored.lock().unwrap() = Some(stored(doc));

        let err = load(&reconciler, &meta, "rust").await.unwrap_err();
        let text = err.to_string();
        assert!(text.contains("is refused"), "{text}");
        assert!(text.contains("poll trigger(s) 'watch_inbox'"), "{text}");
        assert!(text.contains("--single-trust"), "{text}");

        let refused = registry.refused.lock().unwrap();
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].0, meta.id);
        assert!(refused[0].1.contains("'watch_inbox'"));
        assert!(schedules.cron.lock().unwrap().is_empty());
        assert!(
            reconciler
                .runtime
                .clone()
                .unwrap()
                .task_namespaces()
                .is_empty()
        );
    }

    /// Python reactive code and native providers are named in the reasons.
    #[test]
    fn python_reactive_code_and_native_providers_are_named() {
        let mut doc = document("python", vec![]);
        doc.reactors = vec![ReactorPackageMetadata {
            name: "prices".to_string(),
            package_name: "pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            accumulators: vec![],
        }];
        let mut py_manifest = manifest("python");
        py_manifest.graph_name = Some("pricing".to_string());
        let reasons = code_needed_in_process(&doc, &py_manifest, &[]);
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].contains("Python reactive code"));
        assert!(reasons[0].contains("'prices'"));
        assert!(reasons[0].contains("'pricing'"));

        // A Rust package's reactors are not Python code; its native
        // provider is named.
        let mut rust_doc = document("rust", vec![]);
        rust_doc.reactors = doc.reactors.clone();
        let reasons = code_needed_in_process(
            &rust_doc,
            &manifest("rust"),
            &["cloacina-provider-kafka".to_string()],
        );
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].contains("native provider(s) 'cloacina-provider-kafka'"));

        // A plain Python workflow needs nothing in this process.
        let plain = document("python", vec![task("a", &[])]);
        assert!(code_needed_in_process(&plain, &manifest("python"), &[]).is_empty());
    }

    /// A Python package with a reactor is refused through the load, and a
    /// plain Python workflow loads with its task graph written.
    #[tokio::test]
    async fn python_packages_from_the_document() {
        let registry = Arc::new(DocumentRegistry::default());
        let reconciler = reconciler("acme", registry.clone(), Arc::default());

        let reactive = metadata("reactive");
        let mut doc = document("python", vec![]);
        doc.reactors = vec![ReactorPackageMetadata {
            name: "prices".to_string(),
            package_name: "reactive".to_string(),
            reaction_mode: "when_any".to_string(),
            accumulators: vec![],
        }];
        *registry.stored.lock().unwrap() = Some(stored(doc));
        let err = load(&reconciler, &reactive, "python").await.unwrap_err();
        assert!(err.to_string().contains("Python reactive code"), "{err}");
        assert_eq!(registry.refused.lock().unwrap().len(), 1);

        let plain = metadata("plain");
        *registry.stored.lock().unwrap() = Some(stored(document(
            "python",
            vec![task("a", &[]), task("b", &["a"])],
        )));
        load(&reconciler, &plain, "python").await.expect("loads");
        let graphs = registry.task_graphs.lock().unwrap();
        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].0, plain.id);
        assert_eq!(graphs[0].1[1], ("b".to_string(), vec!["a".to_string()]));
        let runtime = reconciler.runtime.clone().unwrap();
        assert!(runtime.get_workflow("wf").is_some());
    }
}
