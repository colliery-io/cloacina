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

//! The output route gives the end of each attempt's output (CLOACINA-T-1091).
//! Needs Postgres, like the other router tests.

use super::*;

use cloacina::fleet::AttemptOutput;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;

/// A run with one task, in `public`.
async fn one_task(state: &AppState) -> (cloacina::UniversalUuid, cloacina::UniversalUuid) {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let wf = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: format!("t1091_{}", uuid::Uuid::new_v4().simple()),
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
            task_name: "public::embedded::t1091::load".into(),
            status: "Completed".into(),
            attempt: 2,
            max_attempts: 2,
            trigger_rules: r#"{"type":"Always"}"#.into(),
            task_configuration: "{}".into(),
        })
        .await
        .expect("task execution")
        .id;
    (wf, task)
}

async fn get(state: &AppState, token: &str, uri: String) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

/// Two attempts with output: the route gives both in attempt order, one with
/// `?attempt=`, none under another execution's id.
#[tokio::test]
#[serial]
async fn the_output_route_gives_each_attempt_in_order() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let (wf, task) = one_task(&state).await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    for (attempt, text, cut) in [(2, "second\n", false), (1, "end of first\n", true)] {
        dal.task_execution()
            .record_attempt_output(
                task,
                wf,
                attempt,
                AttemptOutput {
                    text: text.into(),
                    total_bytes: 9000,
                    cut,
                },
            )
            .await
            .expect("store the output");
    }

    let base = format!(
        "/v1/tenants/public/executions/{}/tasks/{}/output",
        wf.0, task.0
    );
    let (status, body) = get(&state, &token, base.clone()).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let attempts = body["attempts"].as_array().expect("attempts");
    assert_eq!(attempts.len(), 2, "body: {body:?}");
    assert_eq!(attempts[0]["attempt"], 1);
    assert_eq!(attempts[0]["output"], "end of first\n");
    assert_eq!(attempts[0]["cut"], true);
    assert_eq!(attempts[0]["total_bytes"], 9000);
    assert_eq!(attempts[1]["attempt"], 2);
    assert_eq!(attempts[1]["cut"], false);
    assert!(attempts[1]["recorded_at"].as_str().is_some());

    let (status, body) = get(&state, &token, format!("{base}?attempt=2")).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(body["attempts"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["attempts"][0]["output"], "second\n");

    let other = uuid::Uuid::new_v4();
    let (status, body) = get(
        &state,
        &token,
        format!(
            "/v1/tenants/public/executions/{other}/tasks/{}/output",
            task.0
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(body["attempts"].as_array().map(Vec::len), Some(0));

    let (status, _) = get(
        &state,
        &token,
        format!(
            "/v1/tenants/public/executions/{}/tasks/not-a-uuid/output",
            wf.0
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
