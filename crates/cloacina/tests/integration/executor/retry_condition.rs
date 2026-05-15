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

//! End-to-end tests for the `retry_condition` task attribute (CLOACI-T-0042).
//!
//! The retry-policy machinery (`RetryPolicy::should_retry`,
//! `RetryCondition::{AllErrors,Never,TransientOnly,ErrorPattern}`)
//! already lived in `cloacina-workflow` and was wired into the
//! executor. These tests pin the end-to-end behavior so a future
//! refactor can't silently break the retry-skip path:
//!
//! - `retry_condition = "transient"` retries on a transient-flavored
//!   error message (matches the "connection" pattern) and eventually
//!   succeeds.
//! - `retry_condition = "never"` does NOT retry: a failing task lands
//!   in `Failed` after exactly one attempt regardless of
//!   `retry_attempts`.

use cloacina::runner::DefaultRunner;
use cloacina::*;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::fixtures::get_or_init_fixture;

// Per-process attempt counters keyed by task id. Tests run serially via
// `cargo test --test-threads=1` (the angreal integration runner pins
// this), so static atomics are sufficient — no need for a Mutex<HashMap>.
static TRANSIENT_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static NEVER_ATTEMPTS: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
struct WorkflowTask {
    id: String,
    dependencies: Vec<TaskNamespace>,
}

impl WorkflowTask {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            dependencies: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl Task for WorkflowTask {
    async fn execute(&self, context: Context<Value>) -> Result<Context<Value>, TaskError> {
        Ok(context)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

// `retry_condition = "transient"` matches the "connection" substring in
// our error message, so the task should be retried. We deliberately fail
// the first two attempts (attempts 1 + 2) and succeed on the third so
// the executor exercises the should_retry → schedule retry path twice,
// then completes.
#[task(
    id = "transient_retry_task",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 50,
    retry_max_delay_ms = 200,
    retry_jitter = false,
    retry_condition = "transient"
)]
async fn transient_retry_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    let attempt = TRANSIENT_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
    if attempt < 3 {
        return Err(TaskError::ExecutionFailed {
            message: "simulated connection refused (transient)".into(),
            task_id: "transient_retry_task".into(),
            timestamp: chrono::Utc::now(),
        });
    }
    Ok(())
}

// `retry_condition = "never"` must skip retries even when
// `retry_attempts` is set. Task fails on its single attempt and stays
// failed.
#[task(
    id = "never_retry_task",
    dependencies = [],
    retry_attempts = 5,
    retry_delay_ms = 50,
    retry_max_delay_ms = 200,
    retry_jitter = false,
    retry_condition = "never"
)]
async fn never_retry_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    NEVER_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
    Err(TaskError::ExecutionFailed {
        message: "validation failed (do not retry)".into(),
        task_id: "never_retry_task".into(),
        timestamp: chrono::Utc::now(),
    })
}

async fn run_with_task(
    task_factory: impl Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    workflow_name: &str,
    task_id: &str,
) -> (uuid::Uuid, cloacina::dal::DAL, DefaultRunner) {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();

    let workflow = Workflow::builder(workflow_name)
        .description("retry_condition integration coverage")
        .add_task(Arc::new(WorkflowTask::new(task_id)))
        .unwrap()
        .build()
        .unwrap();

    let runtime = cloacina::Runtime::empty();
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        task_id,
    );
    runtime.register_task(namespace, task_factory);
    runtime.register_workflow(workflow_name.to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let schema = fixture.get_schema();
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .runtime(runtime)
        .build()
        .await
        .unwrap();

    let execution = runner
        .execute_async(workflow_name, Context::new())
        .await
        .unwrap();
    let exec_id = execution.execution_id;
    let dal = cloacina::dal::DAL::new(database.clone());
    (exec_id, dal, runner)
}

#[tokio::test]
#[serial_test::serial]
async fn test_retry_condition_transient_retries_and_succeeds() {
    TRANSIENT_ATTEMPTS.store(0, Ordering::SeqCst);

    let (exec_id, dal, runner) = run_with_task(
        || Arc::new(transient_retry_task_task()) as Arc<dyn Task>,
        "retry_pipeline_transient",
        "transient_retry_task",
    )
    .await;

    // Poll until the task finishes (Completed or Failed). With 50ms
    // base delay + max 200ms + no jitter, 3 attempts complete well
    // under the 30s ceiling.
    crate::fixtures::poll_until(
        Duration::from_secs(30),
        Duration::from_millis(50),
        "transient retry task should reach a terminal state",
        || {
            let dal = dal.clone();
            async move {
                let tasks = dal
                    .task_execution()
                    .get_all_tasks_for_workflow(UniversalUuid(exec_id))
                    .await
                    .unwrap_or_default();
                tasks
                    .iter()
                    .any(|t| t.status == "Completed" || t.status == "Failed")
            }
        },
    )
    .await;

    let tasks = dal
        .task_execution()
        .get_all_tasks_for_workflow(UniversalUuid(exec_id))
        .await
        .unwrap();
    assert_eq!(
        tasks.len(),
        1,
        "exactly one task row tracks the latest attempt"
    );
    assert_eq!(
        tasks[0].status, "Completed",
        "transient retries should let the task succeed on attempt 3"
    );
    let attempts = TRANSIENT_ATTEMPTS.load(Ordering::SeqCst);
    assert_eq!(
        attempts, 3,
        "expected exactly 3 attempts (2 transient failures + 1 success), got {}",
        attempts
    );

    runner.shutdown().await.unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn test_retry_condition_never_skips_retries() {
    NEVER_ATTEMPTS.store(0, Ordering::SeqCst);

    let (exec_id, dal, runner) = run_with_task(
        || Arc::new(never_retry_task_task()) as Arc<dyn Task>,
        "retry_pipeline_never",
        "never_retry_task",
    )
    .await;

    crate::fixtures::poll_until(
        Duration::from_secs(30),
        Duration::from_millis(50),
        "never-retry task should reach Failed",
        || {
            let dal = dal.clone();
            async move {
                let tasks = dal
                    .task_execution()
                    .get_all_tasks_for_workflow(UniversalUuid(exec_id))
                    .await
                    .unwrap_or_default();
                tasks.iter().any(|t| t.status == "Failed")
            }
        },
    )
    .await;

    let tasks = dal
        .task_execution()
        .get_all_tasks_for_workflow(UniversalUuid(exec_id))
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].status, "Failed",
        "retry_condition=never must surface the failure without retrying"
    );
    let attempts = NEVER_ATTEMPTS.load(Ordering::SeqCst);
    assert_eq!(
        attempts, 1,
        "expected exactly 1 attempt with retry_condition=never (got {})",
        attempts
    );

    runner.shutdown().await.unwrap();
}
