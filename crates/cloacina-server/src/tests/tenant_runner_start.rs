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

//! A tenant's schedules fire with no call for that tenant (CLOACINA-T-1031):
//! after the tenant gets a schedule, after a restart, and with more tenants
//! than the warning limit. They need Postgres, like the other router tests.
//!
//! The tests give the server the names of their own tenants
//! (`ensure_runners`), not the list of every schema (`run_pass`): the test
//! database is shared, and a runner on another test's schema would run that
//! test's work.

use super::*;

use crate::tenant_runner_cache::{TenantRunnerCache, ensure_runners};
use cloacina::dal::DAL;
use cloacina::models::schedule::{NewSchedule, Schedule};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// The workflow that the test schedules run.
const WORKFLOW: &str = "t1031_report";

/// A task that counts its runs.
struct CountTask {
    runs: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl cloacina::Task for CountTask {
    async fn execute(
        &self,
        context: cloacina::Context<serde_json::Value>,
    ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(context)
    }

    fn id(&self) -> &str {
        "work"
    }

    fn dependencies(&self) -> &[cloacina::TaskNamespace] {
        &[]
    }
}

/// Makes a tenant schema with the admin API, not with a route: no route is
/// called for the tenant in these tests unless the test says so.
async fn make_tenant(state: &AppState, label: &str) -> String {
    let name = format!("t1031_{}_{}", label, uuid::Uuid::new_v4().simple());
    cloacina::database::DatabaseAdmin::new(state.database.clone())
        .create_tenant(cloacina::database::TenantConfig {
            schema_name: name.clone(),
            username: name.clone(),
            password: "testpass123".to_string(),
        })
        .await
        .expect("create tenant");
    name
}

/// A database of the tenant that no server state of the test owns, so the
/// test can still read after a state has shut its runners down.
fn tenant_dal(tenant: &str) -> DAL {
    DAL::new(
        cloacina::Database::try_new_with_schema(TEST_DB_URL, "cloacina", 2, Some(tenant))
            .expect("tenant database"),
    )
}

/// Puts the workflow of the tenant in the runtime that the tenant runners of
/// `state` share, as a loaded package does. Returns the run counter of its
/// task.
fn load_workflow(state: &AppState, tenant: &str) -> Arc<AtomicUsize> {
    let runs = Arc::new(AtomicUsize::new(0));
    let task: Arc<dyn cloacina::Task> = Arc::new(CountTask { runs: runs.clone() });
    let workflow = cloacina::Workflow::builder(WORKFLOW)
        .tenant(tenant)
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
    let runtime = state
        .tenant_runners
        .shared_runtime()
        .scoped_to_tenant(tenant);
    runtime.register_task(namespace, move || task.clone());
    runtime.register_workflow(WORKFLOW.to_string(), move || workflow.clone());
    runs
}

/// Fires once a year: after the fire that a test waits for, no other fire
/// comes due while the test runs.
const YEARLY: &str = "0 0 1 1 *";

/// Fires at each hour.
const HOURLY: &str = "0 * * * *";

/// A cron schedule row that is already due, as the reconciler writes one for
/// a package and time then passes.
async fn due_schedule(dal: &DAL, cron_expression: &str, catchup_policy: &str) -> Schedule {
    let due = chrono::Utc::now() - chrono::Duration::seconds(30);
    schedule_row(dal, cron_expression, catchup_policy, due).await
}

/// A cron schedule row with its next fire at `next_run`.
async fn schedule_row(
    dal: &DAL,
    cron_expression: &str,
    catchup_policy: &str,
    next_run: chrono::DateTime<chrono::Utc>,
) -> Schedule {
    let mut schedule = NewSchedule::cron(
        WORKFLOW,
        cron_expression,
        cloacina::database::universal_types::UniversalTimestamp(next_run),
    );
    schedule.catchup_policy = Some(catchup_policy.to_string());
    dal.schedule()
        .create(schedule)
        .await
        .expect("create schedule")
}

/// The number of fires recorded for the schedule.
async fn fires(dal: &DAL, schedule: &Schedule) -> usize {
    dal.schedule_execution()
        .list_by_schedule(schedule.id, 1000, 0)
        .await
        .expect("list schedule executions")
        .len()
}

/// Polls until the schedule has at least `at_least` fires. Returns the count
/// seen last.
async fn wait_for_fires(dal: &DAL, schedule: &Schedule, at_least: usize) -> usize {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let seen = fires(dal, schedule).await;
        if seen >= at_least || tokio::time::Instant::now() >= deadline {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Polls until `counter` is at least `at_least`. Returns the value seen last.
async fn wait_for_runs(counter: &AtomicUsize, at_least: usize) -> usize {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let seen = counter.load(Ordering::SeqCst);
        if seen >= at_least || tokio::time::Instant::now() >= deadline {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Stops the runners of each state and removes the tenants.
async fn clean_up(states: &[&AppState], tenants: &[&String]) {
    for state in states {
        let _ = state.tenant_runners.shutdown_all().await;
        for tenant in tenants {
            state.tenant_databases.evict(tenant).await;
        }
    }
    let admin = cloacina::database::DatabaseAdmin::new(states[0].database.clone());
    for tenant in tenants {
        let _ = admin.remove_tenant(tenant, tenant).await;
    }
}

/// A named tenant has a due cron schedule and no route was ever called for
/// it. The pass over the tenants makes its runner, and the schedule fires
/// and runs the workflow.
#[tokio::test]
#[serial]
async fn a_tenant_schedule_fires_with_no_call_for_the_tenant() {
    let state = test_state().await;
    let tenant = make_tenant(&state, "nocall").await;
    let dal = tenant_dal(&tenant);
    let runs = load_workflow(&state, &tenant);
    let schedule = due_schedule(&dal, YEARLY, "skip").await;

    // With no runner, nothing fires: this is the fault of CLOACINA-T-1031.
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!state.tenant_runners.contains(&tenant).await);
    assert_eq!(fires(&dal, &schedule).await, 0, "no runner, no fire");

    let made = ensure_runners(&state, std::slice::from_ref(&tenant)).await;
    assert_eq!(made, 1, "the pass makes the tenant's runner");

    assert_eq!(
        wait_for_fires(&dal, &schedule, 1).await,
        1,
        "the due schedule fires once (skip policy) with no call for the tenant"
    );
    assert_eq!(
        wait_for_runs(&runs, 1).await,
        1,
        "the fire ran the workflow"
    );
    let after = dal.schedule().get_by_id(schedule.id).await.expect("read");
    assert!(after.last_run_at.is_some());
    assert!(
        after.next_run_at.expect("next run").0 > chrono::Utc::now(),
        "the schedule moved on to its next fire"
    );

    // A second pass makes nothing new.
    assert_eq!(
        ensure_runners(&state, std::slice::from_ref(&tenant)).await,
        0
    );
    clean_up(&[&state], &[&tenant]).await;
}

/// A restart: the first server state stops, fire times pass, and a second
/// state over the same database starts. With no call for the tenant, the
/// second state's pass makes the runner, and the fires missed in between
/// follow each schedule's catch-up policy: one fire for `skip`, each missed
/// fire for `run_all`.
#[tokio::test]
#[serial]
async fn after_a_restart_schedules_fire_and_missed_fires_follow_the_policy() {
    let first = test_state().await;
    let tenant = make_tenant(&first, "restart").await;
    let dal = tenant_dal(&tenant);
    load_workflow(&first, &tenant);
    let skip = due_schedule(&dal, YEARLY, "skip").await;
    // Not due while the first server runs.
    let in_a_day = chrono::Utc::now() + chrono::Duration::days(1);
    let run_all = schedule_row(&dal, HOURLY, "run_all", in_a_day).await;

    assert_eq!(
        ensure_runners(&first, std::slice::from_ref(&tenant)).await,
        1
    );
    assert_eq!(wait_for_fires(&dal, &skip, 1).await, 1);

    // The server stops.
    let stopped = first.tenant_runners.shutdown_all().await;
    assert!(stopped.values().all(|r| r.is_ok()), "{stopped:?}");
    let skip_before = fires(&dal, &skip).await;
    let run_all_before = fires(&dal, &run_all).await;

    // It stays down over three fire times of the hourly schedule: each
    // schedule last ran 3 hours and 30 minutes ago, and its next fire was
    // due 2 hours and 30 minutes ago.
    let last_run = chrono::Utc::now() - chrono::Duration::minutes(210);
    for schedule in [&skip, &run_all] {
        dal.schedule()
            .update_schedule_times(
                schedule.id,
                last_run,
                last_run + chrono::Duration::minutes(60),
            )
            .await
            .expect("set the schedule back");
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(fires(&dal, &skip).await, skip_before, "down: no fire");

    // The server starts again: a new state, new caches, the same database.
    let second = test_state().await;
    let runs = load_workflow(&second, &tenant);
    assert!(!second.tenant_runners.contains(&tenant).await);
    assert_eq!(
        ensure_runners(&second, std::slice::from_ref(&tenant)).await,
        1
    );

    let skip_after = wait_for_fires(&dal, &skip, skip_before + 1).await;
    let run_all_after = wait_for_fires(&dal, &run_all, run_all_before + 3).await;
    // Let a wrong extra fire show itself.
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(
        fires(&dal, &skip).await,
        skip_before + 1,
        "skip: one fire for the missed fire times (saw {skip_after})"
    );
    let run_all_new = fires(&dal, &run_all).await - run_all_before;
    // The hours that began between the last run and now: 3 or 4.
    let missed = (chrono::Utc::now().timestamp().div_euclid(3600)
        - last_run.timestamp().div_euclid(3600)) as usize;
    assert!(
        run_all_new >= 3 && run_all_new <= missed,
        "run_all: each missed fire time fires (saw {run_all_new} of {missed}, \
         first read {run_all_after})"
    );
    assert!(
        wait_for_runs(&runs, 1 + run_all_new).await > run_all_new,
        "the fires ran the workflow"
    );

    clean_up(&[&second, &first], &[&tenant]).await;
}

/// More tenants than the warning limit. Each gets a runner, none is stopped,
/// and the schedule of the tenant that came first still fires afterwards.
#[tokio::test]
#[serial]
async fn tenants_above_the_limit_keep_their_runners_and_schedules() {
    let mut state = test_state().await;
    state.tenant_runners = Arc::new(TenantRunnerCache::new(
        std::num::NonZeroUsize::new(1).expect("limit 1"),
        runner_config_for_tenant_cache(None, "default", false, None, Default::default()),
    ));
    let first = make_tenant(&state, "limit_a").await;
    let second = make_tenant(&state, "limit_b").await;
    let third = make_tenant(&state, "limit_c").await;
    let dal = tenant_dal(&first);
    let runs = load_workflow(&state, &first);

    let tenants = [first.clone(), second.clone(), third.clone()];
    assert_eq!(ensure_runners(&state, &tenants).await, 3);
    assert_eq!(
        state.tenant_runners.len().await,
        3,
        "limit 1, three runners"
    );
    for tenant in &tenants {
        assert!(state.tenant_runners.contains(tenant).await);
    }

    // The first tenant gets a due schedule only now, after the other two
    // runners were made. Before CLOACINA-T-1031 its runner was stopped and
    // its connection pool closed at this point.
    let schedule = due_schedule(&dal, YEARLY, "skip").await;
    let first_database = state
        .tenant_databases
        .resolve(&first, &state.database)
        .await
        .expect("resolve the first tenant");
    state
        .tenant_runners
        .get_or_create(&first, first_database.clone())
        .await
        .expect("the first tenant's runner")
        .notify_cron_change();
    assert_eq!(wait_for_fires(&dal, &schedule, 1).await, 1);
    assert_eq!(wait_for_runs(&runs, 1).await, 1);
    DAL::new(first_database)
        .schedule()
        .get_by_id(schedule.id)
        .await
        .expect("the pool the tenant's routes use is open");

    clean_up(&[&state], &[&first, &second, &third]).await;
}

/// Two servers hold a runner for the same tenant, as two replicas do. A due
/// schedule fires once, not once for each runner.
#[tokio::test]
#[serial]
async fn two_runners_for_one_tenant_fire_a_schedule_once() {
    let one = test_state().await;
    let two = test_state().await;
    let tenant = make_tenant(&one, "replicas").await;
    let dal = tenant_dal(&tenant);
    let runs_one = load_workflow(&one, &tenant);
    let runs_two = load_workflow(&two, &tenant);
    assert_eq!(ensure_runners(&one, std::slice::from_ref(&tenant)).await, 1);
    assert_eq!(ensure_runners(&two, std::slice::from_ref(&tenant)).await, 1);

    // Both schedulers are asleep with no schedule. Make one that is due and
    // wake both at the same time.
    let schedule = due_schedule(&dal, YEARLY, "skip").await;
    for state in [&one, &two] {
        let database = state
            .tenant_databases
            .resolve(&tenant, &state.database)
            .await
            .expect("resolve");
        state
            .tenant_runners
            .get_or_create(&tenant, database)
            .await
            .expect("runner")
            .notify_cron_change();
    }

    assert_eq!(wait_for_fires(&dal, &schedule, 1).await, 1);
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(fires(&dal, &schedule).await, 1, "one fire, two runners");
    assert_eq!(
        runs_one.load(Ordering::SeqCst) + runs_two.load(Ordering::SeqCst),
        1,
        "the workflow ran once"
    );

    clean_up(&[&one, &two], &[&tenant]).await;
}

/// The pass is given the list of tenant schemas. A tenant in the list with
/// no runner gets one. A tenant that had a runner and is no longer in the
/// list (it was deleted through another replica) loses its runner and its
/// cached database; the other tenant keeps its runner.
#[tokio::test]
#[serial]
async fn the_pass_follows_the_list_of_tenant_schemas() {
    use crate::tenant_runner_cache::apply_tenant_list;

    let state = test_state().await;
    let stays = make_tenant(&state, "stays").await;
    let goes = make_tenant(&state, "goes").await;
    let both = [stays.clone(), goes.clone()];

    assert_eq!(apply_tenant_list(&state, &[], &both).await, (2, 0));
    assert_eq!(apply_tenant_list(&state, &both, &both).await, (0, 0));

    let held = state.tenant_runners.tenant_ids().await;
    let listed = [stays.clone()];
    assert_eq!(apply_tenant_list(&state, &held, &listed).await, (0, 1));
    assert!(state.tenant_runners.contains(&stays).await);
    assert!(!state.tenant_runners.contains(&goes).await);
    assert!(
        !state.tenant_databases.evict(&goes).await,
        "the cached database of the tenant that is gone was dropped"
    );

    clean_up(&[&state], &[&stays, &goes]).await;
}

/// The tenant create route and the package upload route make the tenant's
/// runner, so a new tenant and a publish need no other call.
#[tokio::test]
#[serial]
async fn tenant_create_and_upload_start_the_tenant_runner() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1031_routes_{}", uuid::Uuid::new_v4().simple());

    let create = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/tenants")
        .header("Authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "name": tenant, "password": "testpass123" }).to_string(),
        ))
        .unwrap();
    let (status, _) = send_request(build_router(state.clone()), create).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(
        state.tenant_runners.contains(&tenant).await,
        "tenant create starts the runner"
    );

    // A restart, then a publish as the first call for the tenant.
    let restarted = test_state().await;
    assert!(!restarted.tenant_runners.contains(&tenant).await);
    let package = std::fs::read(fixture_path("python-workflow.cloacina")).expect("fixture");
    let (boundary, body_bytes) = multipart_file_body(&package);
    let upload = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/v1/tenants/{tenant}/workflows"))
        .header("Authorization", format!("Bearer {token}"))
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bytes))
        .unwrap();
    let (status, body) = send_request(build_router(restarted.clone()), upload).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    assert!(
        restarted.tenant_runners.contains(&tenant).await,
        "a publish starts the tenant's runner"
    );

    clean_up(&[&restarted, &state], &[&tenant]).await;
}
