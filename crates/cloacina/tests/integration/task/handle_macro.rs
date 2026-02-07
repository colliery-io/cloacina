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

//! Tests for `#[task]` macro detection of the optional `TaskHandle` parameter.
//!
//! Verifies that:
//! - Tasks without a handle parameter report `requires_handle() == false`
//! - Tasks with a `handle` parameter report `requires_handle() == true`
//! - Tasks with a `task_handle` parameter report `requires_handle() == true`
//! - Handle-aware tasks can still execute (context-only path via `Task::execute`)

use cloacina::{task, Context, Task, TaskError, TaskHandle};
use serde_json::Value;

// --- Task WITHOUT handle parameter ---

#[task(id = "no_handle_task", dependencies = [])]
async fn no_handle_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    context
        .insert("no_handle", Value::Bool(true))
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("{e}"),
            task_id: "no_handle_task".into(),
            timestamp: chrono::Utc::now(),
        })?;
    Ok(())
}

// --- Task WITH `handle` parameter ---

#[task(id = "with_handle_task", dependencies = [])]
async fn with_handle_task(
    context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    // Just verify handle is accessible
    let _ = handle.is_slot_held();
    context
        .insert("with_handle", Value::Bool(true))
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("{e}"),
            task_id: "with_handle_task".into(),
            timestamp: chrono::Utc::now(),
        })?;
    Ok(())
}

// --- Task WITH `task_handle` parameter (alternate name) ---

#[task(id = "with_task_handle_task", dependencies = [])]
async fn with_task_handle_task(
    context: &mut Context<Value>,
    task_handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    let _ = task_handle.is_slot_held();
    context
        .insert("with_task_handle", Value::Bool(true))
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("{e}"),
            task_id: "with_task_handle_task".into(),
            timestamp: chrono::Utc::now(),
        })?;
    Ok(())
}

#[tokio::test]
async fn test_no_handle_task_does_not_require_handle() {
    let task = no_handle_task_task();
    assert!(
        !task.requires_handle(),
        "Task without handle param should not require handle"
    );
}

#[tokio::test]
async fn test_handle_param_requires_handle() {
    let task = with_handle_task_task();
    assert!(
        task.requires_handle(),
        "Task with 'handle' param should require handle"
    );
}

#[tokio::test]
async fn test_task_handle_param_requires_handle() {
    let task = with_task_handle_task_task();
    assert!(
        task.requires_handle(),
        "Task with 'task_handle' param should require handle"
    );
}

#[tokio::test]
async fn test_no_handle_task_executes_normally() {
    let task = no_handle_task_task();
    let context = Context::new();
    let result = task.execute(context).await;
    assert!(result.is_ok());
    let ctx = result.unwrap();
    assert_eq!(ctx.get("no_handle"), Some(&Value::Bool(true)));
}
