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
use crate::executor::types::ClaimedTask;
use crate::task::TaskNamespace;

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
}

impl TaskContextBuilder {
    pub fn new(dal: DAL) -> Self {
        Self {
            dal,
            secret_resolver: None,
        }
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

        // Attach the secret resolution side channel (T-0858) to whichever
        // context we return below. This is the runtime-only, never-serialized
        // handle — resolved secrets are returned to the task, never written into
        // `data`.
        if let Some(resolver) = &self.secret_resolver {
            context.set_secret_resolver(resolver.clone());
        }

        // Load initial workflow context if task has no dependencies.
        if dependencies.is_empty() {
            if let Ok(workflow_execution) = self
                .dal
                .workflow_execution()
                .get_by_id(claimed_task.workflow_execution_id)
                .await
            {
                if let Some(context_id) = workflow_execution.context_id {
                    if let Ok(initial_context) = self
                        .dal
                        .context()
                        .read::<serde_json::Value>(context_id)
                        .await
                    {
                        for (key, value) in initial_context.data() {
                            let _ = context.insert(key, value.clone());
                        }
                        debug!(
                            "Loaded initial workflow context with {} keys",
                            initial_context.data().len()
                        );
                    }
                }
            }
            return Ok(context);
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
        Ok(context)
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
