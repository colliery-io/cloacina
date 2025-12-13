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

use cloacina::cloacina_workflow::Task;
use cloacina::{task, Context, TaskError};
use std::sync::atomic::{AtomicU32, Ordering};

// Completely separate counters for each test scenario
static TEST1_SUCCESS_COUNT: AtomicU32 = AtomicU32::new(0);
static TEST2_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);
static TEST3_SUCCESS_COUNT: AtomicU32 = AtomicU32::new(0);
static TEST3_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);
static TEST4_SUCCESS_COUNT: AtomicU32 = AtomicU32::new(0);
static TEST4_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);

// Test 1: Callback for success-only task
async fn test1_success_callback(
    _task_id: &str,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST1_SUCCESS_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

// Test 2: Callback for failure-only task
async fn test2_failure_callback(
    _task_id: &str,
    _error: &cloacina::cloacina_workflow::TaskError,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST2_FAILURE_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

// Test 3: Callbacks for both-success scenario
async fn test3_success_callback(
    _task_id: &str,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST3_SUCCESS_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

async fn test3_failure_callback(
    _task_id: &str,
    _error: &cloacina::cloacina_workflow::TaskError,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST3_FAILURE_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

// Test 4: Callbacks for both-failure scenario
async fn test4_success_callback(
    _task_id: &str,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST4_SUCCESS_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

async fn test4_failure_callback(
    _task_id: &str,
    _error: &cloacina::cloacina_workflow::TaskError,
    _context: &Context<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    TEST4_FAILURE_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

// Test 1: Task with on_success callback only
#[task(id = "test1-success-only", dependencies = [], on_success = test1_success_callback)]
async fn test1_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

// Test 2: Task with on_failure callback only
#[task(id = "test2-failure-only", dependencies = [], on_failure = test2_failure_callback)]
async fn test2_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Err(TaskError::ExecutionFailed {
        message: "Intentional failure for testing".to_string(),
        task_id: "test2-failure-only".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

// Test 3: Task with both callbacks - succeeds
#[task(
    id = "test3-both-success",
    dependencies = [],
    on_success = test3_success_callback,
    on_failure = test3_failure_callback
)]
async fn test3_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

// Test 4: Task with both callbacks - fails
#[task(
    id = "test4-both-failure",
    dependencies = [],
    on_success = test4_success_callback,
    on_failure = test4_failure_callback
)]
async fn test4_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Err(TaskError::ExecutionFailed {
        message: "Intentional failure".to_string(),
        task_id: "test4-both-failure".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

#[tokio::test]
async fn test_on_success_callback_invoked() {
    let initial = TEST1_SUCCESS_COUNT.load(Ordering::SeqCst);

    let task = test1_task_task();
    let context = Context::new();
    let result = task.execute(context).await;

    assert!(result.is_ok(), "Task should succeed");
    assert_eq!(
        TEST1_SUCCESS_COUNT.load(Ordering::SeqCst),
        initial + 1,
        "on_success callback should have been called once"
    );
}

#[tokio::test]
async fn test_on_failure_callback_invoked() {
    let initial = TEST2_FAILURE_COUNT.load(Ordering::SeqCst);

    let task = test2_task_task();
    let context = Context::new();
    let result = task.execute(context).await;

    assert!(result.is_err(), "Task should fail");
    assert_eq!(
        TEST2_FAILURE_COUNT.load(Ordering::SeqCst),
        initial + 1,
        "on_failure callback should have been called once"
    );
}

#[tokio::test]
async fn test_both_callbacks_success_path() {
    let initial_success = TEST3_SUCCESS_COUNT.load(Ordering::SeqCst);
    let initial_failure = TEST3_FAILURE_COUNT.load(Ordering::SeqCst);

    let task = test3_task_task();
    let context = Context::new();
    let result = task.execute(context).await;

    assert!(result.is_ok(), "Task should succeed");
    assert_eq!(
        TEST3_SUCCESS_COUNT.load(Ordering::SeqCst),
        initial_success + 1,
        "on_success callback should have been called"
    );
    assert_eq!(
        TEST3_FAILURE_COUNT.load(Ordering::SeqCst),
        initial_failure,
        "on_failure callback should NOT have been called"
    );
}

#[tokio::test]
async fn test_both_callbacks_failure_path() {
    let initial_success = TEST4_SUCCESS_COUNT.load(Ordering::SeqCst);
    let initial_failure = TEST4_FAILURE_COUNT.load(Ordering::SeqCst);

    let task = test4_task_task();
    let context = Context::new();
    let result = task.execute(context).await;

    assert!(result.is_err(), "Task should fail");
    assert_eq!(
        TEST4_SUCCESS_COUNT.load(Ordering::SeqCst),
        initial_success,
        "on_success callback should NOT have been called"
    );
    assert_eq!(
        TEST4_FAILURE_COUNT.load(Ordering::SeqCst),
        initial_failure + 1,
        "on_failure callback should have been called"
    );
}
