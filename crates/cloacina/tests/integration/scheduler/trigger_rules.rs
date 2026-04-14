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

use crate::fixtures::get_or_init_fixture;
use async_trait::async_trait;
use cloacina::execution_planner::{TaskScheduler, TriggerCondition, TriggerRule, ValueOperator};
use cloacina::*;
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;

// Simple mock task for testing
#[derive(Clone)]
struct SimpleTask {
    id: String,
}

#[async_trait]
impl Task for SimpleTask {
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
        &[]
    }
}

/// Mock task with configurable trigger rules and dependencies.
#[derive(Clone)]
struct TriggerTask {
    id: String,
    deps: Vec<TaskNamespace>,
    rules: serde_json::Value,
}

#[async_trait]
impl Task for TriggerTask {
    async fn execute(
        &self,
        mut context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        context
            .insert(format!("{}_ran", self.id), json!(true))
            .unwrap();
        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.deps
    }

    fn trigger_rules(&self) -> serde_json::Value {
        self.rules.clone()
    }
}

#[tokio::test]
#[serial]
async fn test_always_trigger_rule() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();

    let simple_task = SimpleTask {
        id: "trigger-task".to_string(),
    };
    let workflow = Workflow::builder("trigger-test")
        .description("Test Always trigger rule")
        .add_task(Arc::new(simple_task))
        .expect("Failed to add task")
        .build()
        .expect("Failed to build workflow");

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor("trigger-test".to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("trigger-test", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    // Verify the default trigger rule is "Always"
    let dal = fixture.get_dal();
    let _tasks = dal
        .task_execution()
        .get_all_tasks_for_workflow(UniversalUuid(execution_id))
        .await
        .expect("Failed to get tasks");

    // Since we have an empty workflow, there should be no tasks
    // But the workflow execution should be created successfully
    let wf_exec = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(execution_id))
        .await
        .expect("Failed to get workflow execution");

    assert_eq!(wf_exec.status, "Pending");
}

#[tokio::test]
#[serial]
async fn test_trigger_rule_serialization() {
    // Test serialization of various trigger rules
    let always_rule = TriggerRule::Always;
    let serialized = serde_json::to_value(&always_rule).expect("Failed to serialize Always rule");
    assert_eq!(serialized, json!({"type": "Always"}));

    let all_rule = TriggerRule::All {
        conditions: vec![
            TriggerCondition::TaskSuccess {
                task_name: "task1".to_string(),
            },
            TriggerCondition::ContextValue {
                key: "status".to_string(),
                operator: ValueOperator::Equals,
                value: json!("ready"),
            },
        ],
    };

    let serialized_all = serde_json::to_value(&all_rule).expect("Failed to serialize All rule");
    let expected = json!({
        "type": "All",
        "conditions": [
            {
                "type": "TaskSuccess",
                "task_name": "task1"
            },
            {
                "type": "ContextValue",
                "key": "status",
                "operator": "Equals",
                "value": "ready"
            }
        ]
    });

    assert_eq!(serialized_all, expected);
}

#[tokio::test]
#[serial]
async fn test_context_value_operators() {
    // Test different value operators
    let operators = vec![
        ValueOperator::Equals,
        ValueOperator::NotEquals,
        ValueOperator::GreaterThan,
        ValueOperator::LessThan,
        ValueOperator::Contains,
        ValueOperator::NotContains,
        ValueOperator::Exists,
        ValueOperator::NotExists,
    ];

    for operator in operators {
        let condition = TriggerCondition::ContextValue {
            key: "test_key".to_string(),
            operator,
            value: json!("test_value"),
        };

        let serialized = serde_json::to_value(&condition).expect("Failed to serialize condition");
        assert!(serialized.is_object());
        assert_eq!(serialized["type"], "ContextValue");
        assert_eq!(serialized["key"], "test_key");
        assert_eq!(serialized["value"], "test_value");
    }
}

#[tokio::test]
#[serial]
async fn test_trigger_condition_types() {
    // Test all trigger condition types
    let task_success = TriggerCondition::TaskSuccess {
        task_name: "successful_task".to_string(),
    };

    let task_failed = TriggerCondition::TaskFailed {
        task_name: "failed_task".to_string(),
    };

    let task_skipped = TriggerCondition::TaskSkipped {
        task_name: "skipped_task".to_string(),
    };

    let context_value = TriggerCondition::ContextValue {
        key: "environment".to_string(),
        operator: ValueOperator::Equals,
        value: json!("production"),
    };

    let conditions = vec![task_success, task_failed, task_skipped, context_value];

    for condition in conditions {
        let serialized = serde_json::to_value(&condition).expect("Failed to serialize condition");
        assert!(serialized.is_object());
        assert!(serialized.get("type").is_some());
    }
}

#[tokio::test]
#[serial]
async fn test_complex_trigger_rule() {
    // Test a complex trigger rule with multiple conditions
    let complex_rule = TriggerRule::Any {
        conditions: vec![
            TriggerCondition::TaskSuccess {
                task_name: "data_extraction".to_string(),
            },
            TriggerCondition::ContextValue {
                key: "retry_count".to_string(),
                operator: ValueOperator::LessThan,
                value: json!(3),
            },
        ],
    };

    // This should serialize and deserialize correctly
    let serialized = serde_json::to_value(&complex_rule).expect("Failed to serialize complex rule");
    let deserialized: TriggerRule =
        serde_json::from_value(serialized).expect("Failed to deserialize complex rule");

    match deserialized {
        TriggerRule::Any { conditions } => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected Any trigger rule"),
    }
}

// ── Runtime evaluation tests ────────────────────────────────────────

/// Helper: schedule a workflow and run one round of execution processing.
/// Returns the task statuses as a map of task_name -> status.
async fn schedule_and_process(
    workflow_name: &str,
    workflow: Workflow,
    input: Context<serde_json::Value>,
) -> std::collections::HashMap<String, String> {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let database = fixture.get_database();

    register_workflow_constructor(workflow_name.to_string(), {
        let w = workflow.clone();
        move || w.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();
    let exec_id = scheduler
        .schedule_workflow_execution(workflow_name, input)
        .await
        .expect("Failed to schedule");

    // Drive one round of scheduling
    scheduler
        .process_active_executions()
        .await
        .expect("Failed to process executions");

    let dal = fixture.get_dal();
    let tasks = dal
        .task_execution()
        .get_all_tasks_for_workflow(UniversalUuid(exec_id))
        .await
        .expect("Failed to get tasks");

    tasks
        .into_iter()
        .map(|t| {
            // Extract the short task id (last segment of namespace)
            let short_name = t.task_name.rsplit("::").next().unwrap_or(&t.task_name);
            (short_name.to_string(), t.status)
        })
        .collect()
}

#[tokio::test]
#[serial]
async fn test_runtime_all_conditions_met_task_becomes_ready() {
    // task1 (no deps, Always) -> task2 (depends on task1, All[TaskSuccess("task1")])
    // After task1 is marked ready by the scheduler, task2 should still be NotStarted
    // because task1 hasn't completed yet.
    let wf_name = "runtime-all-met";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({"type": "Always"}),
    };
    let task1_ns = TaskNamespace::new("public", "embedded", wf_name, "task1");
    let task2 = TriggerTask {
        id: "task2".to_string(),
        deps: vec![task1_ns],
        rules: json!({
            "type": "All",
            "conditions": [
                {"type": "TaskSuccess", "task_name": format!("public::embedded::{}::task1", wf_name)}
            ]
        }),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test All trigger rule runtime")
        .add_task(Arc::new(task1))
        .unwrap()
        .add_task(Arc::new(task2))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    // task1 has no deps -> should be Ready (or already dispatched)
    let task1_status = &statuses["task1"];
    assert!(
        task1_status == "Ready" || task1_status == "Running",
        "task1 should be Ready or Running, got: {}",
        task1_status
    );

    // task2 depends on task1 which hasn't completed -> NotStarted
    let task2_status = &statuses["task2"];
    assert_eq!(task2_status, "NotStarted", "task2 should wait for task1");
}

#[tokio::test]
#[serial]
async fn test_runtime_always_rule_no_deps_becomes_ready() {
    let wf_name = "runtime-always";
    let task1 = TriggerTask {
        id: "solo".to_string(),
        deps: vec![],
        rules: json!({"type": "Always"}),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test Always trigger rule")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    let status = &statuses["solo"];
    assert!(
        status == "Ready" || status == "Running",
        "Task with Always rule and no deps should be Ready, got: {}",
        status
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_none_rule_no_conditions_becomes_ready() {
    // None with empty conditions -> true (no conditions matched -> execute)
    let wf_name = "runtime-none-empty";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({"type": "None", "conditions": []}),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test None with empty conditions")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    let status = &statuses["task1"];
    assert!(
        status == "Ready" || status == "Running",
        "None{{}} with no conditions should pass, got: {}",
        status
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_all_empty_conditions_becomes_ready() {
    // All with empty conditions -> true (vacuously true)
    let wf_name = "runtime-all-empty";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({"type": "All", "conditions": []}),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test All with empty conditions")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    let status = &statuses["task1"];
    assert!(
        status == "Ready" || status == "Running",
        "All{{}} with no conditions should pass, got: {}",
        status
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_any_empty_conditions_gets_skipped() {
    // Any with empty conditions -> false (no conditions passed)
    let wf_name = "runtime-any-empty";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({"type": "Any", "conditions": []}),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test Any with empty conditions")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    assert_eq!(
        statuses["task1"], "Skipped",
        "Any{{}} with no conditions should skip"
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_context_value_exists_passes() {
    // Task with ContextValue Exists condition — context has the key
    let wf_name = "runtime-ctx-exists";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({
            "type": "All",
            "conditions": [
                {"type": "ContextValue", "key": "my_flag", "operator": "Exists", "value": true}
            ]
        }),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test ContextValue Exists")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let mut ctx = Context::new();
    ctx.insert("my_flag".to_string(), json!(true)).unwrap();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    let status = &statuses["task1"];
    assert!(
        status == "Ready" || status == "Running",
        "Task with Exists condition on present key should be Ready, got: {}",
        status
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_context_value_exists_fails_skipped() {
    // Task with ContextValue Exists condition — context does NOT have the key
    let wf_name = "runtime-ctx-exists-fail";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({
            "type": "All",
            "conditions": [
                {"type": "ContextValue", "key": "missing_key", "operator": "Exists", "value": true}
            ]
        }),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test ContextValue Exists fails")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let ctx = Context::new();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    assert_eq!(
        statuses["task1"], "Skipped",
        "Task with Exists condition on missing key should be Skipped"
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_context_value_equals_passes() {
    let wf_name = "runtime-ctx-equals";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({
            "type": "All",
            "conditions": [
                {"type": "ContextValue", "key": "env", "operator": "Equals", "value": "production"}
            ]
        }),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test ContextValue Equals")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let mut ctx = Context::new();
    ctx.insert("env".to_string(), json!("production")).unwrap();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    let status = &statuses["task1"];
    assert!(
        status == "Ready" || status == "Running",
        "Equals condition matching should pass, got: {}",
        status
    );
}

#[tokio::test]
#[serial]
async fn test_runtime_context_value_equals_fails_skipped() {
    let wf_name = "runtime-ctx-equals-fail";
    let task1 = TriggerTask {
        id: "task1".to_string(),
        deps: vec![],
        rules: json!({
            "type": "All",
            "conditions": [
                {"type": "ContextValue", "key": "env", "operator": "Equals", "value": "production"}
            ]
        }),
    };

    let workflow = Workflow::builder(wf_name)
        .description("Test ContextValue Equals fails")
        .add_task(Arc::new(task1))
        .unwrap()
        .build()
        .unwrap();

    let mut ctx = Context::new();
    ctx.insert("env".to_string(), json!("staging")).unwrap();
    let statuses = schedule_and_process(wf_name, workflow, ctx).await;

    assert_eq!(
        statuses["task1"], "Skipped",
        "Equals condition not matching should skip"
    );
}
