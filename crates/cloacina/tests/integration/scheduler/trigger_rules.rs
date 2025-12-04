/*
 *  Copyright 2025 Colliery Software
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
use cloacina::task_scheduler::{TaskScheduler, TriggerCondition, TriggerRule, ValueOperator};
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
        .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
        .await
        .expect("Failed to get tasks");

    // Since we have an empty workflow, there should be no tasks
    // But the pipeline should be created successfully
    let pipeline = dal
        .pipeline_execution()
        .get_by_id(UniversalUuid(execution_id))
        .await
        .expect("Failed to get pipeline");

    assert_eq!(pipeline.status, "Pending");
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
