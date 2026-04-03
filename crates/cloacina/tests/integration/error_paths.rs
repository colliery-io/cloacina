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

//! Error path and negative tests.
//!
//! Tests that invalid inputs produce the correct errors (not panics).

use async_trait::async_trait;
use cloacina::task_scheduler::{TriggerCondition, TriggerRule, ValueOperator};
use cloacina::*;
use std::sync::Arc;

// ── Mock tasks for workflow error testing ───────────────────────────

#[derive(Clone)]
struct MockTask {
    id: String,
    deps: Vec<TaskNamespace>,
}

#[async_trait]
impl Task for MockTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.deps
    }
}

// ── Workflow validation errors ──────────────────────────────────────

#[test]
fn test_empty_workflow_returns_error() {
    let workflow = Workflow::new("empty-wf");
    let result = workflow.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ValidationError::EmptyWorkflow
    ));
}

#[test]
fn test_duplicate_task_returns_error() {
    let task1 = MockTask {
        id: "task1".to_string(),
        deps: vec![],
    };
    let task2 = MockTask {
        id: "task1".to_string(), // Same ID
        deps: vec![],
    };

    let result = Workflow::builder("dup-test")
        .add_task(Arc::new(task1))
        .expect("first add should succeed")
        .add_task(Arc::new(task2));

    let err = result.err().expect("Expected error for duplicate task");
    match err {
        WorkflowError::DuplicateTask(name) => {
            assert!(name.contains("task1"));
        }
        other => panic!("Expected DuplicateTask, got: {:?}", other),
    }
}

#[test]
fn test_missing_dependency_returns_error() {
    let nonexistent_dep = TaskNamespace::new("public", "embedded", "missing-dep-wf", "ghost");
    let task = MockTask {
        id: "task1".to_string(),
        deps: vec![nonexistent_dep],
    };

    let result = Workflow::builder("missing-dep-wf")
        .add_task(Arc::new(task))
        .expect("add should succeed")
        .build();

    // build() validates and should return the error
    let err = result
        .err()
        .expect("Expected MissingDependency error from build()");
    match err {
        ValidationError::MissingDependency { task, dependency } => {
            assert!(task.contains("task1"));
            assert!(dependency.contains("ghost"));
        }
        other => panic!("Expected MissingDependency, got: {:?}", other),
    }
}

#[test]
fn test_cyclic_dependency_returns_error() {
    let wf_name = "cycle-wf";
    let task2_ns = TaskNamespace::new("public", "embedded", wf_name, "task2");
    let task1_ns = TaskNamespace::new("public", "embedded", wf_name, "task1");

    let task1 = MockTask {
        id: "task1".to_string(),
        deps: vec![task2_ns],
    };
    let task2 = MockTask {
        id: "task2".to_string(),
        deps: vec![task1_ns],
    };

    let result = Workflow::builder(wf_name)
        .add_task(Arc::new(task1))
        .unwrap()
        .add_task(Arc::new(task2))
        .unwrap()
        .build();

    // build() validates and should detect the cycle
    let err = result
        .err()
        .expect("Expected CyclicDependency error from build()");
    assert!(matches!(err, ValidationError::CyclicDependency { .. }));
}

// ── Trigger rule deserialization errors ─────────────────────────────

#[test]
fn test_invalid_trigger_rule_json() {
    let result = serde_json::from_str::<TriggerRule>("not json at all");
    assert!(result.is_err());
}

#[test]
fn test_unknown_trigger_rule_type() {
    let result = serde_json::from_str::<TriggerRule>(r#"{"type":"UnknownRule"}"#);
    assert!(result.is_err());
}

#[test]
fn test_trigger_rule_all_missing_conditions() {
    let result = serde_json::from_str::<TriggerRule>(r#"{"type":"All"}"#);
    assert!(result.is_err());
}

#[test]
fn test_trigger_rule_conditions_wrong_type() {
    let result =
        serde_json::from_str::<TriggerRule>(r#"{"type":"All","conditions":"not_an_array"}"#);
    assert!(result.is_err());
}

#[test]
fn test_unknown_condition_type() {
    let result =
        serde_json::from_str::<TriggerCondition>(r#"{"type":"UnknownCondition","task_name":"x"}"#);
    assert!(result.is_err());
}

#[test]
fn test_context_value_condition_missing_fields() {
    let result = serde_json::from_str::<TriggerCondition>(r#"{"type":"ContextValue","key":"k"}"#);
    assert!(result.is_err());
}

#[test]
fn test_unknown_value_operator() {
    let result = serde_json::from_str::<ValueOperator>(r#""UnknownOp""#);
    assert!(result.is_err());
}

// ── Context error paths ─────────────────────────────────────────────

#[test]
fn test_context_duplicate_insert_returns_error() {
    let mut ctx = Context::<serde_json::Value>::new();
    ctx.insert("key1", serde_json::json!("val1")).unwrap();

    let result = ctx.insert("key1", serde_json::json!("val2"));
    assert!(result.is_err());
}

#[test]
fn test_context_update_missing_key_returns_error() {
    let mut ctx = Context::<serde_json::Value>::new();
    let result = ctx.update("nonexistent", serde_json::json!("val"));
    assert!(result.is_err());
}

#[test]
fn test_context_get_missing_key_returns_none() {
    let ctx = Context::<serde_json::Value>::new();
    assert!(ctx.get("missing").is_none());
}

// ── CronEvaluator error paths ───────────────────────────────────────

#[test]
fn test_cron_invalid_expression_error() {
    use cloacina::cron_evaluator::CronEvaluator;
    let result = CronEvaluator::new("invalid cron", "UTC");
    assert!(result.is_err());
}

#[test]
fn test_cron_invalid_timezone_error() {
    use cloacina::cron_evaluator::CronEvaluator;
    let result = CronEvaluator::new("0 9 * * *", "Not/A/Timezone");
    assert!(result.is_err());
}

#[test]
fn test_cron_empty_expression_error() {
    use cloacina::cron_evaluator::CronEvaluator;
    let result = CronEvaluator::new("", "UTC");
    assert!(result.is_err());
}

// ── Manifest validation errors ──────────────────────────────────────

#[test]
fn test_manifest_parse_duration_invalid() {
    use cloacina::packaging::manifest_schema::parse_duration_str;
    assert!(parse_duration_str("invalid").is_err());
    assert!(parse_duration_str("5x").is_err());
    assert!(parse_duration_str("").is_err());
    assert!(parse_duration_str("abc123").is_err());
}

#[test]
fn test_manifest_parse_duration_valid() {
    use cloacina::packaging::manifest_schema::parse_duration_str;
    assert_eq!(
        parse_duration_str("100ms").unwrap(),
        std::time::Duration::from_millis(100)
    );
    assert_eq!(
        parse_duration_str("5s").unwrap(),
        std::time::Duration::from_secs(5)
    );
    assert_eq!(
        parse_duration_str("2m").unwrap(),
        std::time::Duration::from_secs(120)
    );
    assert_eq!(
        parse_duration_str("1h").unwrap(),
        std::time::Duration::from_secs(3600)
    );
}
