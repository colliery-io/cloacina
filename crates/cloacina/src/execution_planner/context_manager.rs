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

//! Context management for task execution.
//!
//! This module handles loading and merging contexts for tasks based on
//! their dependencies.

use tracing::debug;

use std::sync::Arc;

use crate::dal::DAL;
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use crate::Context;
use crate::Runtime;

use super::trigger_rules::ValueOperator;

/// Context management operations for the scheduler.
pub struct ContextManager<'a> {
    dal: &'a DAL,
    runtime: Arc<Runtime>,
}

impl<'a> ContextManager<'a> {
    /// Creates a new ContextManager.
    pub fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self {
        Self { dal, runtime }
    }

    /// Loads the context for a specific task based on its dependencies.
    pub async fn load_context_for_task(
        &self,
        task_execution: &TaskExecution,
    ) -> Result<Context<serde_json::Value>, ValidationError> {
        // Get the workflow to find task dependencies
        let pipeline = self
            .dal
            .workflow_execution()
            .get_by_id(task_execution.pipeline_execution_id)
            .await?;
        let workflow = match self.runtime.get_workflow(&pipeline.pipeline_name) {
            Some(wf) => wf,
            None => {
                return Err(ValidationError::WorkflowNotFound(
                    pipeline.pipeline_name.clone(),
                ));
            }
        };

        // Parse the task name string to TaskNamespace
        let task_namespace = crate::task::TaskNamespace::from_string(&task_execution.task_name)
            .map_err(ValidationError::InvalidTaskName)?;

        let dependencies = workflow
            .get_dependencies(&task_namespace)
            .map_err(|e| ValidationError::InvalidTaskName(e.to_string()))?;

        if dependencies.is_empty() {
            // No dependencies: read initial pipeline context
            if let Some(context_id) = pipeline.context_id {
                let context = self.dal.context().read(context_id).await.map_err(|_e| {
                    ValidationError::ContextEvaluationFailed {
                        key: format!("context_id:{}", context_id),
                    }
                })?;
                debug!(
                    "Context loaded: initial pipeline context ({} keys)",
                    context.data().len()
                );
                Ok(context)
            } else {
                debug!("Context loaded: empty initial context");
                Ok(Context::new())
            }
        } else if dependencies.len() == 1 {
            // Single dependency: read that task's saved context
            let dep_task_namespace = &dependencies[0];
            let dep_task_name = dep_task_namespace.to_string();
            match self
                .dal
                .task_execution_metadata()
                .get_by_pipeline_and_task(task_execution.pipeline_execution_id, dep_task_namespace)
                .await
            {
                Ok(task_metadata) => {
                    if let Some(context_id) = task_metadata.context_id {
                        match self
                            .dal
                            .context()
                            .read::<serde_json::Value>(context_id)
                            .await
                        {
                            Ok(context) => {
                                debug!(
                                    "Context loaded: from dependency '{}' ({} keys)",
                                    dep_task_name,
                                    context.data().len()
                                );
                                Ok(context)
                            }
                            Err(e) => Err(ValidationError::ContextEvaluationFailed {
                                key: format!("context_read_error:{}", e),
                            }),
                        }
                    } else {
                        // Task completed but has no output context
                        debug!(
                            "Context loaded: empty (dependency '{}' has no output context)",
                            dep_task_name
                        );
                        Ok(Context::new())
                    }
                }
                Err(_) => {
                    // Dependency task hasn't completed yet or no metadata saved
                    debug!(
                        "Context loaded: empty (dependency '{}' not found)",
                        dep_task_name
                    );
                    Ok(Context::new())
                }
            }
        } else {
            // Multiple dependencies: merge their saved contexts
            self.merge_dependency_contexts(task_execution, dependencies)
                .await
        }
    }

    /// Merges contexts from multiple dependencies.
    async fn merge_dependency_contexts(
        &self,
        task_execution: &TaskExecution,
        dependencies: &[crate::task::TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ValidationError> {
        let mut merged_context = Context::new();
        let mut sources = Vec::new();

        for dep_task_namespace in dependencies {
            let dep_task_name = dep_task_namespace.to_string();
            if let Ok(task_metadata) = self
                .dal
                .task_execution_metadata()
                .get_by_pipeline_and_task(task_execution.pipeline_execution_id, dep_task_namespace)
                .await
            {
                if let Some(context_id) = task_metadata.context_id {
                    if let Ok(dep_context) = self
                        .dal
                        .context()
                        .read::<serde_json::Value>(context_id)
                        .await
                    {
                        sources.push(format!("{}({})", dep_task_name, dep_context.data().len()));
                        // Merge dependency context (later dependencies override earlier ones)
                        for (key, value) in dep_context.data() {
                            if merged_context.get(key).is_some() {
                                merged_context
                                    .update(key.clone(), value.clone())
                                    .map_err(|e| ValidationError::ContextEvaluationFailed {
                                        key: format!("merge_error:{}", e),
                                    })?;
                            } else {
                                merged_context
                                    .insert(key.clone(), value.clone())
                                    .map_err(|e| ValidationError::ContextEvaluationFailed {
                                        key: format!("merge_error:{}", e),
                                    })?;
                            }
                        }
                    }
                }
            }
        }

        debug!(
            "Context loaded: merged from {} ({} total keys)",
            sources.join(", "),
            merged_context.data().len()
        );
        Ok(merged_context)
    }

    /// Evaluates a context-based condition using the provided operator.
    pub fn evaluate_context_condition(
        context: &Context<serde_json::Value>,
        key: &str,
        operator: &ValueOperator,
        expected: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        let actual = context.get(key);

        match operator {
            ValueOperator::Exists => Ok(actual.is_some()),
            ValueOperator::NotExists => Ok(actual.is_none()),
            ValueOperator::Equals => Ok(actual == Some(expected)),
            ValueOperator::NotEquals => Ok(actual != Some(expected)),
            ValueOperator::GreaterThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::LessThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::Contains => match (actual, expected) {
                (Some(a), b) if a.is_string() && b.is_string() => {
                    Ok(a.as_str().unwrap_or("").contains(b.as_str().unwrap_or("")))
                }
                (Some(a), b) if a.is_array() => Ok(a.as_array().unwrap_or(&vec![]).contains(b)),
                _ => Ok(false),
            },
            ValueOperator::NotContains => Ok(!Self::evaluate_context_condition(
                context,
                key,
                &ValueOperator::Contains,
                expected,
            )?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx_with(pairs: Vec<(&str, serde_json::Value)>) -> Context<serde_json::Value> {
        let mut ctx = Context::new();
        for (k, v) in pairs {
            ctx.insert(k.to_string(), v).unwrap();
        }
        ctx
    }

    // ── Exists / NotExists ──────────────────────────────────────────

    #[test]
    fn exists_returns_true_when_key_present() {
        let ctx = ctx_with(vec![("name", json!("alice"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "name",
            &ValueOperator::Exists,
            &json!(true),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn exists_returns_false_when_key_missing() {
        let ctx = Context::new();
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "name",
            &ValueOperator::Exists,
            &json!(true),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn not_exists_returns_true_when_key_missing() {
        let ctx = Context::new();
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "name",
            &ValueOperator::NotExists,
            &json!(true),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn not_exists_returns_false_when_key_present() {
        let ctx = ctx_with(vec![("name", json!("alice"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "name",
            &ValueOperator::NotExists,
            &json!(true),
        )
        .unwrap();
        assert!(!result);
    }

    // ── Equals / NotEquals ──────────────────────────────────────────

    #[test]
    fn equals_string_match() {
        let ctx = ctx_with(vec![("status", json!("ready"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "status",
            &ValueOperator::Equals,
            &json!("ready"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn equals_string_mismatch() {
        let ctx = ctx_with(vec![("status", json!("pending"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "status",
            &ValueOperator::Equals,
            &json!("ready"),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn equals_number_match() {
        let ctx = ctx_with(vec![("count", json!(42))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "count",
            &ValueOperator::Equals,
            &json!(42),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn equals_boolean_match() {
        let ctx = ctx_with(vec![("flag", json!(true))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "flag",
            &ValueOperator::Equals,
            &json!(true),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn equals_missing_key_returns_false() {
        let ctx = Context::new();
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "missing",
            &ValueOperator::Equals,
            &json!("anything"),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn not_equals_different_values() {
        let ctx = ctx_with(vec![("status", json!("error"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "status",
            &ValueOperator::NotEquals,
            &json!("ok"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn not_equals_same_values() {
        let ctx = ctx_with(vec![("status", json!("ok"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "status",
            &ValueOperator::NotEquals,
            &json!("ok"),
        )
        .unwrap();
        assert!(!result);
    }

    // ── GreaterThan / LessThan ──────────────────────────────────────

    #[test]
    fn greater_than_true() {
        let ctx = ctx_with(vec![("score", json!(100))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "score",
            &ValueOperator::GreaterThan,
            &json!(50),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn greater_than_false_when_equal() {
        let ctx = ctx_with(vec![("score", json!(50))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "score",
            &ValueOperator::GreaterThan,
            &json!(50),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn greater_than_non_number_returns_false() {
        let ctx = ctx_with(vec![("name", json!("alice"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "name",
            &ValueOperator::GreaterThan,
            &json!(10),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn greater_than_missing_key_returns_false() {
        let ctx = Context::new();
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "missing",
            &ValueOperator::GreaterThan,
            &json!(0),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn less_than_true() {
        let ctx = ctx_with(vec![("score", json!(10))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "score",
            &ValueOperator::LessThan,
            &json!(50),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn less_than_float() {
        let ctx = ctx_with(vec![("ratio", json!(0.5))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "ratio",
            &ValueOperator::LessThan,
            &json!(1.0),
        )
        .unwrap();
        assert!(result);
    }

    // ── Contains / NotContains ──────────────────────────────────────

    #[test]
    fn contains_string_substring() {
        let ctx = ctx_with(vec![("message", json!("hello world"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "message",
            &ValueOperator::Contains,
            &json!("world"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn contains_string_not_found() {
        let ctx = ctx_with(vec![("message", json!("hello"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "message",
            &ValueOperator::Contains,
            &json!("world"),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn contains_array_element() {
        let ctx = ctx_with(vec![("tags", json!(["a", "b", "c"]))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "tags",
            &ValueOperator::Contains,
            &json!("b"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn contains_array_element_missing() {
        let ctx = ctx_with(vec![("tags", json!(["a", "b"]))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "tags",
            &ValueOperator::Contains,
            &json!("z"),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn contains_non_string_non_array_returns_false() {
        let ctx = ctx_with(vec![("count", json!(42))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "count",
            &ValueOperator::Contains,
            &json!(4),
        )
        .unwrap();
        assert!(!result);
    }

    #[test]
    fn not_contains_string() {
        let ctx = ctx_with(vec![("message", json!("hello"))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "message",
            &ValueOperator::NotContains,
            &json!("world"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn not_contains_array() {
        let ctx = ctx_with(vec![("tags", json!(["a", "b"]))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "tags",
            &ValueOperator::NotContains,
            &json!("z"),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn not_contains_when_present() {
        let ctx = ctx_with(vec![("tags", json!(["a", "b"]))]);
        let result = ContextManager::evaluate_context_condition(
            &ctx,
            "tags",
            &ValueOperator::NotContains,
            &json!("a"),
        )
        .unwrap();
        assert!(!result);
    }
}
