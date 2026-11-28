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

//! A run with a priority through the router (CLOACINA-T-1123): the limit of
//! the key, the notice in the answer, the execution detail and the rows the
//! planner writes. They need Postgres, like the other router tests of this
//! crate.

use super::*;

use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalUuid;

/// A task that does nothing.
struct Noop;

#[async_trait::async_trait]
impl cloacina::Task for Noop {
    async fn execute(
        &self,
        context: cloacina::Context<serde_json::Value>,
    ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
        Ok(context)
    }

    fn id(&self) -> &str {
        "work"
    }

    fn dependencies(&self) -> &[cloacina::TaskNamespace] {
        &[]
    }
}

/// Puts a workflow with one task in the runtime of the `public` tenant's
/// runner, as a loaded package does. Returns its name.
fn load_public_workflow(state: &AppState) -> String {
    let name = format!("t1123_prio_{}", uuid::Uuid::new_v4().simple());
    let task: Arc<dyn cloacina::Task> = Arc::new(Noop);
    let workflow = cloacina::Workflow::builder(&name)
        .tenant("public")
        .add_task(task.clone())
        .expect("add task")
        .build()
        .expect("build workflow");
    let namespace = cloacina::TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "work",
    );
    let runtime = state.runner.runtime();
    runtime.register_task(namespace, move || task.clone());
    runtime.register_workflow(name.clone(), move || workflow.clone());
    name
}

fn run_request(
    workflow: &str,
    token: &str,
    priority: serde_json::Value,
) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(format!("/v1/tenants/public/workflows/{workflow}/execute"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({ "context": {}, "priority": priority }).to_string(),
        ))
        .unwrap()
}

fn get(uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// The stored execution and the task row of a run.
async fn rows(
    state: &AppState,
    execution_id: &str,
) -> (
    cloacina::models::workflow_execution::WorkflowExecutionRecord,
    Vec<cloacina::models::task_execution::TaskExecution>,
) {
    let dal = DAL::new(state.database.clone());
    let id = UniversalUuid(uuid::Uuid::parse_str(execution_id).expect("execution id"));
    let execution = dal
        .workflow_execution()
        .get_by_id(id)
        .await
        .expect("execution row");
    let tasks = dal
        .task_execution()
        .get_all_tasks_for_workflow(id)
        .await
        .expect("task rows");
    (execution, tasks)
}

/// A `write` key may set 5 at most. It asks for 1: the run is not refused,
/// it runs at 5, and the answer, the execution detail and the rows say asked
/// 1 and given 5.
#[tokio::test]
#[serial]
async fn a_write_key_that_asks_above_its_limit_is_lowered_with_a_notice() {
    let state = test_state().await;
    let workflow = load_public_workflow(&state);
    let write = create_tenant_api_key(&state, "public", "write").await;

    let (status, body) = send_request(
        build_router(state.clone()),
        run_request(&workflow, &write, serde_json::json!(1)),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body["priority"]["asked"], 1, "{body}");
    assert_eq!(body["priority"]["given"], 5, "{body}");
    let notice = body["priority_notice"].as_str().expect("a notice");
    assert!(notice.contains('1') && notice.contains('5'), "{notice}");

    let id = body["execution_id"]
        .as_str()
        .expect("execution id")
        .to_string();
    let (status, detail) = send_request(
        build_router(state.clone()),
        get(&format!("/v1/tenants/public/executions/{id}"), &write),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["priority"]["asked"], 1, "{detail}");
    assert_eq!(detail["priority"]["given"], 5, "{detail}");

    let (execution, tasks) = rows(&state, &id).await;
    assert_eq!((execution.priority_asked, execution.priority), (Some(1), 5));
    assert_eq!(tasks.len(), 1);
    assert_eq!((tasks[0].workflow_priority, tasks[0].task_priority), (5, 5));
    assert_eq!(tasks[0].task_priority_asked, None, "the task declares none");
}

/// A platform admin key may set 0: it gets what it asks for, with no notice.
/// A run that asks for nothing gets the default and no notice.
#[tokio::test]
#[serial]
async fn an_admin_key_gets_the_priority_it_asks_for() {
    let state = test_state().await;
    let workflow = load_public_workflow(&state);
    let admin = create_test_api_key(&state).await;

    let (status, body) = send_request(
        build_router(state.clone()),
        run_request(&workflow, &admin, serde_json::json!(1)),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body["priority"]["asked"], 1);
    assert_eq!(body["priority"]["given"], 1);
    assert!(body["priority_notice"].is_null(), "{body}");
    let id = body["execution_id"]
        .as_str()
        .expect("execution id")
        .to_string();
    let (execution, tasks) = rows(&state, &id).await;
    assert_eq!((execution.priority_asked, execution.priority), (Some(1), 1));
    assert_eq!((tasks[0].workflow_priority, tasks[0].task_priority), (1, 1));

    let (status, body) = send_request(
        build_router(state.clone()),
        run_request(&workflow, &admin, serde_json::Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body["priority"]["given"], 5, "{body}");
    assert!(body["priority_notice"].is_null(), "{body}");
}

/// A priority outside 0 to 10 is a bad request, and no run starts.
#[tokio::test]
#[serial]
async fn a_priority_outside_the_scale_is_refused() {
    let state = test_state().await;
    let workflow = load_public_workflow(&state);
    let write = create_tenant_api_key(&state, "public", "write").await;

    let (status, body) = send_request(
        build_router(state.clone()),
        run_request(&workflow, &write, serde_json::json!(11)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_priority", "{body}");
}
