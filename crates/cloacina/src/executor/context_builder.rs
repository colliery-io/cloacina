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

//! Shared dependency-context builder (CLOACI-I-0114 / task T-0633).
//!
//! Extracted verbatim from `ThreadTaskExecutor::build_task_context` +
//! `merge_context_values` so the upcoming `FleetExecutor` resolves the merged
//! dependency context the **exact** same way the thread executor does. Without
//! this seam the two executors would inevitably drift on what context a task
//! sees — mirroring the same drift risk `TaskResultHandler` (T-0630) closed
//! for the post-execution path.
//!
//! The builder is intentionally backend-agnostic and holds only a `DAL`; the
//! caller supplies the task's dependency namespaces (the thread executor gets
//! them from the locally-loaded `Task::dependencies()`; the fleet executor
//! from the same server-side `Runtime`).

use std::sync::Arc;
use tracing::{debug, error};

use cloacina_workflow::secret::SecretResolver;

use crate::context::Context;
use crate::dal::DAL;
use crate::error::ExecutorError;
use crate::executor::context_size::{ContextKind, ContextSizeLimits, check_context_size};
use crate::executor::types::ClaimedTask;
use crate::security::secret_grant::{
    DeclaredSecretResolver, SecretGrantMode, declared_secret_grant, secret_bindings,
};
use crate::task::TaskNamespace;
use cloacina_workflow::secret::SECRET_REFS_KEY;

/// Builds a task's input context by loading + merging its dependency contexts
/// (or the workflow's initial context when the task has no dependencies).
#[derive(Clone)]
pub struct TaskContextBuilder {
    dal: DAL,
    /// Optional secret resolution side channel (CLOACI-T-0858). When set, every
    /// context this builder produces carries the resolver so a task body can
    /// call `context.secret(...)`. It is attached to the runtime-only,
    /// never-serialized handle on `Context`, so it cannot leak into the durable
    /// context. `None` on paths that don't configure secrets.
    secret_resolver: Option<Arc<dyn SecretResolver>>,
    /// CLOACINA-T-1047: the warning and the hard limit on the size of the
    /// input context this builder gives a task.
    context_limits: ContextSizeLimits,
    /// CLOACINA-T-1081: with [`SecretGrantMode::Declared`] the resolver a
    /// task gets answers only the secrets its package declares.
    secret_grants: SecretGrantMode,
}

impl TaskContextBuilder {
    pub fn new(dal: DAL) -> Self {
        Self {
            dal,
            secret_resolver: None,
            context_limits: ContextSizeLimits::default(),
            secret_grants: SecretGrantMode::Unrestricted,
        }
    }

    /// The same builder with this rule for the secrets a task reads
    /// (CLOACINA-T-1081). A new builder is [`SecretGrantMode::Unrestricted`].
    pub fn with_secret_grants(mut self, mode: SecretGrantMode) -> Self {
        self.secret_grants = mode;
        self
    }

    /// The same builder with these limits on the size of an input context
    /// (CLOACINA-T-1047). A new builder has the default limits.
    pub fn with_context_limits(mut self, context_limits: ContextSizeLimits) -> Self {
        self.context_limits = context_limits;
        self
    }

    /// Measure the input context of `claimed_task` and apply the limits. Over
    /// the hard limit this is [`ExecutorError::ContextTooLarge`]: the task
    /// must fail before it runs or is put in a work packet.
    fn check_input_size(
        &self,
        claimed_task: &ClaimedTask,
        context: &Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        check_context_size(
            &self.context_limits,
            self.dal.tenant_label(),
            &claimed_task.task_name,
            ContextKind::Input,
            context.data(),
        )
        .map(|_| ())
    }

    /// Attach the secret resolver every built context should carry (T-0858).
    pub fn with_secret_resolver(mut self, resolver: Option<Arc<dyn SecretResolver>>) -> Self {
        self.secret_resolver = resolver;
        self
    }

    /// Build the execution context for `claimed_task` given its `dependencies`.
    ///
    /// - **No dependencies**: load the workflow's initial context (if any).
    /// - **With dependencies**: batch-load each dependency's persisted context
    ///   and smart-merge (latest-wins for primitives, recursive for objects,
    ///   dedup-concat for arrays). A dependency context that fails to parse is
    ///   a hard `ContextLoadFailed` (COR-11) — never a silent partial context.
    ///
    /// The built context is measured, and one over the hard size limit is
    /// [`ExecutorError::ContextTooLarge`] (CLOACINA-T-1047).
    pub async fn build(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        debug!(
            "Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );

        let mut context = Context::new();

        // Load initial workflow context if task has no dependencies.
        if dependencies.is_empty() {
            if let Some(initial_context) = self.initial_context(claimed_task).await {
                for (key, value) in initial_context.data() {
                    let _ = context.insert(key, value.clone());
                }
                debug!(
                    "Loaded initial workflow context with {} keys",
                    initial_context.data().len()
                );
            }
            return self.finish(claimed_task, context).await;
        }

        // Batch-load dependency contexts (eager loading strategy).
        debug!(
            "Loading dependency contexts for {} dependencies: {:?}",
            dependencies.len(),
            dependencies
        );
        let dep_metadata_with_contexts = self
            .dal
            .task_execution_metadata()
            .get_dependency_metadata_with_contexts(claimed_task.workflow_execution_id, dependencies)
            .await
            .map_err(|e| {
                error!(
                    "Failed to load dependency contexts for task '{}': {}",
                    claimed_task.task_name, e
                );
                ExecutorError::ContextLoadFailed(format!(
                    "dependency context load failed for '{}': {}",
                    claimed_task.task_name, e
                ))
            })?;

        debug!(
            "Found {} dependency metadata records",
            dep_metadata_with_contexts.len()
        );
        for (_task_metadata, context_json) in dep_metadata_with_contexts {
            if let Some(json_str) = context_json {
                // COR-11: a parse failure here fails the task explicitly rather
                // than silently continuing with a partial context.
                let dep_context = match Context::<serde_json::Value>::from_json(json_str) {
                    Ok(c) => c,
                    Err(e) => {
                        metrics::counter!(
                            "cloacina_context_merge_failures_total",
                            "kind" => "parse",
                        )
                        .increment(1);
                        return Err(ExecutorError::ContextLoadFailed(format!(
                            "dependency context JSON parse failed for task '{}': {}",
                            claimed_task.task_name, e
                        )));
                    }
                };
                debug!(
                    "Merging dependency context with {} keys: {:?}",
                    dep_context.data().len(),
                    dep_context.data().keys().collect::<Vec<_>>()
                );
                for (key, value) in dep_context.data() {
                    if let Some(existing_value) = context.get(key) {
                        let merged_value = Self::merge_context_values(existing_value, value);
                        if context.update(key, merged_value).is_err() {
                            metrics::counter!(
                                "cloacina_context_merge_failures_total",
                                "kind" => "merge",
                            )
                            .increment(1);
                        }
                    } else if context.insert(key, value.clone()).is_err() {
                        metrics::counter!(
                            "cloacina_context_merge_failures_total",
                            "kind" => "merge",
                        )
                        .increment(1);
                    }
                }
            }
        }

        debug!(
            "Final context for task {} has {} keys: {:?}",
            claimed_task.task_name,
            context.data().len(),
            context.data().keys().collect::<Vec<_>>()
        );

        // CLOACINA-T-1081: the secret bindings are the caller's, from the
        // run's first context. A task's output cannot change which stored
        // secret a later task gets.
        if context.get(SECRET_REFS_KEY).is_some() {
            let bindings = self
                .initial_context(claimed_task)
                .await
                .and_then(|initial| initial.get(SECRET_REFS_KEY).cloned());
            match bindings {
                Some(value) => {
                    let _ = context.update(SECRET_REFS_KEY, value);
                }
                None => {
                    let _ = context.remove(SECRET_REFS_KEY);
                }
            }
        }
        self.finish(claimed_task, context).await
    }

    /// The run's first context, as the caller wrote it.
    async fn initial_context(
        &self,
        claimed_task: &ClaimedTask,
    ) -> Option<Context<serde_json::Value>> {
        let workflow_execution = self
            .dal
            .workflow_execution()
            .get_by_id(claimed_task.workflow_execution_id)
            .await
            .ok()?;
        let context_id = workflow_execution.context_id?;
        self.dal
            .context()
            .read::<serde_json::Value>(context_id)
            .await
            .ok()
    }

    /// Attach the secret resolver and apply the size limits.
    async fn finish(
        &self,
        claimed_task: &ClaimedTask,
        mut context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // The secret resolution side channel (T-0858): runtime only, never
        // serialized. Resolved secrets are returned to the task, never written
        // into `data`.
        if let Some(resolver) = &self.secret_resolver {
            let resolver = match self.secret_grants {
                SecretGrantMode::Unrestricted => resolver.clone(),
                SecretGrantMode::Declared => {
                    self.declared_resolver(claimed_task, &context, resolver.clone())
                        .await
                }
            };
            context.set_secret_resolver(resolver);
        }
        self.check_input_size(claimed_task, &context)?;
        Ok(context)
    }

    /// CLOACINA-T-1081: `resolver` limited to the grant of the task: the
    /// secrets its package declares, through the run's bindings.
    async fn declared_resolver(
        &self,
        claimed_task: &ClaimedTask,
        context: &Context<serde_json::Value>,
        resolver: Arc<dyn SecretResolver>,
    ) -> Arc<dyn SecretResolver> {
        let bindings = context
            .get(SECRET_REFS_KEY)
            .map(|v| secret_bindings(&serde_json::json!({ SECRET_REFS_KEY: v })))
            .unwrap_or_default();
        let grant = match crate::task::parse_namespace(&claimed_task.task_name) {
            Ok(namespace) => {
                declared_secret_grant(
                    &self.dal,
                    claimed_task.workflow_execution_id,
                    &namespace.package_name,
                    &bindings,
                )
                .await
            }
            Err(_) => crate::security::SecretGrant::no_package(&claimed_task.task_name),
        };
        DeclaredSecretResolver::new(
            resolver,
            grant,
            self.dal.tenant_label(),
            &claimed_task.task_name,
            claimed_task.workflow_execution_id.to_string(),
            claimed_task.task_execution_id.to_string(),
        )
        .into_arc()
    }

    /// Smart-merge two context values: arrays concat+dedup, objects merge
    /// recursively, everything else latest-wins.
    pub fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        use serde_json::Value;

        match (existing, new) {
            (Value::Array(existing_arr), Value::Array(new_arr)) => {
                let mut merged = existing_arr.clone();
                for item in new_arr {
                    if !merged.contains(item) {
                        merged.push(item.clone());
                    }
                }
                Value::Array(merged)
            }
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    if let Some(existing_value) = merged.get(key) {
                        merged.insert(
                            key.clone(),
                            Self::merge_context_values(existing_value, value),
                        );
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(merged)
            }
            (_, new_value) => new_value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_primitives_latest_wins() {
        let merged =
            TaskContextBuilder::merge_context_values(&serde_json::json!(1), &serde_json::json!(2));
        assert_eq!(merged, serde_json::json!(2));
    }

    #[test]
    fn merge_arrays_dedup_concat() {
        let merged = TaskContextBuilder::merge_context_values(
            &serde_json::json!([1, 2]),
            &serde_json::json!([2, 3]),
        );
        assert_eq!(merged, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn merge_objects_recursive() {
        let merged = TaskContextBuilder::merge_context_values(
            &serde_json::json!({"a": 1, "nested": {"x": 1}}),
            &serde_json::json!({"b": 2, "nested": {"y": 2}}),
        );
        assert_eq!(
            merged,
            serde_json::json!({"a": 1, "b": 2, "nested": {"x": 1, "y": 2}})
        );
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod input_size_tests {
    //! The builder measures the input context it gives a task and refuses
    //! one over the hard limit (CLOACINA-T-1047). On SQLite in memory.

    use super::*;
    use crate::database::Database;
    use crate::database::universal_types::UniversalUuid;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::task_execution_metadata::NewTaskExecutionMetadata;
    use crate::models::workflow_execution::NewWorkflowExecution;

    async fn dal() -> DAL {
        let url = format!(
            "file:input_size_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        DAL::new(db)
    }

    fn context_with(key: &str, len: usize) -> Context<serde_json::Value> {
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert(key, serde_json::json!("x".repeat(len))).unwrap();
        ctx
    }

    async fn workflow(dal: &DAL, initial: Option<&Context<serde_json::Value>>) -> UniversalUuid {
        let context_id = match initial {
            Some(ctx) => dal.context().create(ctx).await.unwrap(),
            None => None,
        };
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "orders".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id,
            })
            .await
            .unwrap()
            .id
    }

    async fn task(dal: &DAL, workflow_id: UniversalUuid, name: &str) -> ClaimedTask {
        let id = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: name.into(),
                status: "Running".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        ClaimedTask {
            task_execution_id: id,
            workflow_execution_id: workflow_id,
            task_name: name.to_string(),
            attempt: 1,
        }
    }

    /// A completed dependency: a task row and its saved context.
    async fn dependency(
        dal: &DAL,
        workflow_id: UniversalUuid,
        ns: &TaskNamespace,
        ctx: &Context<serde_json::Value>,
    ) {
        let claimed = task(dal, workflow_id, &ns.to_string()).await;
        let context_id = dal.context().create(ctx).await.unwrap();
        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: claimed.task_execution_id,
                workflow_execution_id: workflow_id,
                task_name: ns.to_string(),
                context_id,
                context_size_bytes: None,
            })
            .await
            .unwrap();
    }

    const LIMITS: ContextSizeLimits = ContextSizeLimits {
        warn_bytes: 0,
        max_bytes: 1_000,
    };

    #[tokio::test]
    async fn a_first_context_over_the_limit_is_refused() {
        let dal = dal().await;
        let initial = context_with("rows", 2_000);
        let size = initial.to_json().unwrap().len();
        let workflow_id = workflow(&dal, Some(&initial)).await;
        let claimed = task(&dal, workflow_id, "public::embedded::orders::load").await;

        let err = TaskContextBuilder::new(dal)
            .with_context_limits(LIMITS)
            .build(&claimed, &[])
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            format!(
                "Context too large: the input context of task 'public::embedded::orders::load' \
                 is {size} bytes, over the limit of 1000 bytes. Largest keys: rows (2002 \
                 bytes). Write large data to storage in the task and pass a reference in the \
                 context."
            )
        );
    }

    #[tokio::test]
    async fn two_dependency_contexts_under_the_limit_that_merge_over_it_are_refused() {
        let dal = dal().await;
        let workflow_id = workflow(&dal, None).await;
        let extract = TaskNamespace::new("public", "embedded", "orders", "extract");
        let enrich = TaskNamespace::new("public", "embedded", "orders", "enrich");
        dependency(&dal, workflow_id, &extract, &context_with("a", 600)).await;
        dependency(&dal, workflow_id, &enrich, &context_with("b", 700)).await;
        let claimed = task(&dal, workflow_id, "public::embedded::orders::load").await;

        let err = TaskContextBuilder::new(dal.clone())
            .with_context_limits(LIMITS)
            .build(&claimed, &[extract.clone(), enrich.clone()])
            .await
            .unwrap_err();
        match err {
            ExecutorError::ContextTooLarge {
                kind,
                size_bytes,
                limit_bytes,
                largest_keys,
                ..
            } => {
                assert_eq!(kind, "input");
                assert_eq!(limit_bytes, 1_000);
                assert!(size_bytes > 1_300, "size was {size_bytes}");
                assert_eq!(largest_keys, "b (702 bytes), a (602 bytes)");
            }
            other => panic!("not ContextTooLarge: {other}"),
        }

        // The same merge is accepted when the limit is off.
        let built = TaskContextBuilder::new(dal)
            .with_context_limits(ContextSizeLimits::OFF)
            .build(&claimed, &[extract, enrich])
            .await
            .unwrap();
        assert_eq!(built.data().len(), 2);
    }
}
