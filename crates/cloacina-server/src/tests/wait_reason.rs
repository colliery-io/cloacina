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

//! The task rows of an execution say why a task waits (CLOACINA-T-1148).
//! Needs Postgres, like the other router tests.

use super::*;

use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use cloacina_api_types::TaskWaitReason;

async fn tasks_of(state: &AppState, token: &str, wf: uuid::Uuid) -> serde_json::Value {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/v1/tenants/public/executions/{wf}/tasks"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), request).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    body
}

/// A task put off with a reason has it in the tasks route; once it starts
/// the reason is gone.
#[tokio::test]
#[serial]
async fn the_tasks_route_gives_why_a_task_waits_and_clears_it_when_it_starts() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let wf = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: format!("t1148_{}", uuid::Uuid::new_v4().simple()),
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
            task_name: "public::embedded::t1148::load".into(),
            status: "Ready".into(),
            attempt: 1,
            max_attempts: 1,
            trigger_rules: r#"{"type":"Always"}"#.into(),
            task_configuration: "{}".into(),
        })
        .await
        .expect("task execution")
        .id;

    let body = tasks_of(&state, &token, wf.0).await;
    assert!(body["tasks"][0]["wait_reason"].is_null(), "body: {body:?}");

    // Put off for an hour, so the test's runner does not take it.
    let later = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::hours(1));
    let reason = TaskWaitReason::new(
        TaskWaitReason::NO_AGENT,
        "no live agent of tenant public, pool default; the task runs when one registers",
    );
    assert!(
        dal.task_execution()
            .defer_dispatch_because(task, later, Some(&reason))
            .await
            .expect("defer")
    );
    let body = tasks_of(&state, &token, wf.0).await;
    let wait = &body["tasks"][0]["wait_reason"];
    assert_eq!(wait["code"], "no_agent", "body: {body:?}");
    assert_eq!(wait["message"], reason.message.as_str());

    dal.task_execution()
        .mark_started(task)
        .await
        .expect("start");
    let body = tasks_of(&state, &token, wf.0).await;
    assert!(body["tasks"][0]["wait_reason"].is_null(), "body: {body:?}");
}
