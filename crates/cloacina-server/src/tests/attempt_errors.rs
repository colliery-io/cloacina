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

//! The task rows of an execution carry the error of each failed attempt
//! (CLOACINA-T-1100). Needs Postgres, like the other router tests.

use super::*;

use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;

/// Attempt 1 is retried with its error, attempt 2 fails the task. The tasks
/// route gives both errors with their numbers, the traceback of attempt 1
/// with it.
#[tokio::test]
#[serial]
async fn the_tasks_route_gives_the_error_of_each_failed_attempt() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let workflow = format!("t1100_{}", uuid::Uuid::new_v4().simple());
    let wf = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: workflow,
            workflow_version: "1.0".into(),
            status: "Running".into(),
            context_id: None,
        })
        .await
        .expect("workflow execution")
        .id;
    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            workflow_execution_id: wf,
            task_name: "public::embedded::t1100::load".into(),
            status: "Running".into(),
            attempt: 1,
            max_attempts: 2,
            trigger_rules: r#"{"type":"Always"}"#.into(),
            task_configuration: "{}".into(),
        })
        .await
        .expect("task execution")
        .id;

    let first = cloacina::task_logs::with_error_details("first failure", "Traceback: one");
    // The retry is due in an hour, so the test's runner does not take it.
    let later = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::hours(1));
    dal.task_execution()
        .schedule_retry(task, later, 2, Some(&first))
        .await
        .expect("retry");
    dal.task_execution()
        .mark_failed(task, "second failure", None)
        .await
        .expect("fail");

    let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/v1/tenants/public/executions/{}/tasks", wf.0))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), request).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let row = &body["tasks"][0];
    assert_eq!(row["last_error"], "second failure", "body: {body:?}");
    let errors = row["attempt_errors"].as_array().expect("attempt_errors");
    assert_eq!(errors.len(), 2, "body: {body:?}");
    assert_eq!(errors[0]["attempt"], 1);
    assert_eq!(errors[0]["message"], "first failure");
    assert_eq!(errors[0]["details"], "Traceback: one");
    assert_eq!(errors[1]["attempt"], 2);
    assert_eq!(errors[1]["message"], "second failure");
    assert!(errors[1]["details"].is_null());
    assert!(errors[0]["failed_at"].as_str().is_some());
}
