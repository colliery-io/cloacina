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

use cloacina::{task, CheckpointError, Context, Task, TaskError, TaskNamespace};
use std::sync::{Arc, Mutex};

// Test task that implements custom checkpointing
struct CheckpointableTask {
    id: String,
    dependencies: Vec<TaskNamespace>,
    checkpoint_data: Arc<Mutex<Option<String>>>,
}

impl CheckpointableTask {
    fn new(id: &str, dependencies: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            dependencies: dependencies
                .into_iter()
                .map(|s| TaskNamespace::from_string(s).unwrap())
                .collect(),
            checkpoint_data: Arc::new(Mutex::new(None)),
        }
    }

    fn get_checkpoint_data(&self) -> Option<String> {
        self.checkpoint_data.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Task for CheckpointableTask {
    async fn execute(
        &self,
        mut context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Simulate some work
        context
            .insert("processed_by", serde_json::json!(self.id))
            .map_err(|e| TaskError::ContextError {
                task_id: self.id.clone(),
                error: e,
            })?;
        context
            .insert(
                "timestamp",
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            )
            .map_err(|e| TaskError::ContextError {
                task_id: self.id.clone(),
                error: e,
            })?;
        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }

    fn checkpoint(&self, context: &Context<serde_json::Value>) -> Result<(), CheckpointError> {
        // Save checkpoint data
        let checkpoint_json = context.to_json().map_err(|e| CheckpointError::SaveFailed {
            task_id: self.id.clone(),
            message: format!("Failed to serialize context: {:?}", e),
        })?;

        *self.checkpoint_data.lock().unwrap() = Some(checkpoint_json);

        Ok(())
    }
}

#[test]
fn test_default_checkpoint_implementation() {
    // Test the default Task::checkpoint implementation
    #[task(id = "simple-task", dependencies = [])]
    async fn simple_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    let task = simple_task_task();
    let context = Context::new();

    // Default implementation should succeed without doing anything
    let result = task.checkpoint(&context);
    assert!(result.is_ok());
}

#[test]
fn test_custom_checkpoint_save() {
    let task = CheckpointableTask::new("checkpoint-task", vec![]);
    let mut context = Context::new();

    // Add some data to context
    context
        .insert("test_data", serde_json::json!("checkpoint_test"))
        .unwrap();
    context.insert("number", serde_json::json!(42)).unwrap();

    // Save checkpoint
    let result = task.checkpoint(&context);
    assert!(result.is_ok());

    // Verify checkpoint was saved
    let checkpoint_data = task.get_checkpoint_data();
    assert!(checkpoint_data.is_some());

    let saved_data = checkpoint_data.unwrap();
    assert!(saved_data.contains("checkpoint_test"));
    assert!(saved_data.contains("42"));
}

#[test]
fn test_checkpoint_restore() {
    let task = CheckpointableTask::new("restore-task", vec![]);
    let mut original_context = Context::new();

    // Create original context with data
    original_context
        .insert("original_data", serde_json::json!("test_value"))
        .unwrap();
    original_context
        .insert("count", serde_json::json!(100))
        .unwrap();

    // Save checkpoint
    task.checkpoint(&original_context).unwrap();

    // Simulate restoration by parsing saved checkpoint
    let checkpoint_data = task.get_checkpoint_data().unwrap();
    let restored_context: Context<serde_json::Value> = Context::from_json(checkpoint_data).unwrap();

    // Verify restored data matches original
    assert_eq!(
        restored_context.get("original_data").unwrap(),
        &serde_json::json!("test_value")
    );
    assert_eq!(
        restored_context.get("count").unwrap(),
        &serde_json::json!(100)
    );
}

#[test]
fn test_checkpoint_serialization_error() {
    // Test checkpoint failure due to serialization issues
    struct FailingCheckpointTask;

    #[async_trait::async_trait]
    impl Task for FailingCheckpointTask {
        async fn execute(
            &self,
            context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            Ok(context)
        }

        fn id(&self) -> &str {
            "failing-checkpoint"
        }

        fn dependencies(&self) -> &[TaskNamespace] {
            &[]
        }

        fn checkpoint(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError> {
            // Simulate a serialization error
            Err(CheckpointError::SaveFailed {
                task_id: self.id().to_string(),
                message: "Simulated checkpoint failure".to_string(),
            })
        }
    }

    let task = FailingCheckpointTask;
    let context = Context::new();

    let result = task.checkpoint(&context);
    assert!(result.is_err());

    match result.unwrap_err() {
        CheckpointError::SaveFailed { task_id, message } => {
            assert_eq!(task_id, "failing-checkpoint");
            assert!(message.contains("Simulated checkpoint failure"));
        }
        _ => panic!("Expected SaveFailed error"),
    }
}

#[test]
fn test_checkpoint_validation() {
    let task = CheckpointableTask::new("validation-task", vec![]);
    let mut context = Context::new();

    // Add valid data
    context
        .insert("valid_field", serde_json::json!("valid_value"))
        .unwrap();

    // Checkpoint should succeed with valid data
    let result = task.checkpoint(&context);
    assert!(result.is_ok());

    // Test with empty context (should still work)
    let empty_context = Context::new();
    let empty_result = task.checkpoint(&empty_context);
    assert!(empty_result.is_ok());
}
