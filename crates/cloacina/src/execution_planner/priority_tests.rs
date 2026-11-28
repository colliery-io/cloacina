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

//! CLOACINA-T-1045: priority from the declaration to the order of dispatch,
//! on SQLite in memory. The planner writes the rows; one pass of the
//! scheduler loop hands the waiting tasks to a dispatcher that records the
//! order and takes none of them (every slot is busy).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use super::*;
use crate::database::schema::unified::task_executions;
use crate::dispatcher::{DispatchError, TaskExecutor, TaskReadyEvent};
use crate::{Task, TaskError};
use cloacina_api_types::PriorityGrant;
use cloacina_workflow::TaskPlacement;

struct Stub {
    id: String,
    priority: Option<u8>,
}

#[async_trait]
impl Task for Stub {
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
    fn placement(&self) -> TaskPlacement {
        TaskPlacement {
            priority: self.priority,
            ..Default::default()
        }
    }
}

fn stub(id: &str, priority: Option<u8>) -> Arc<dyn Task> {
    Arc::new(Stub {
        id: id.to_string(),
        priority,
    })
}

/// A dispatcher with no free slot: it records each task it is offered, in
/// the order of the offers, and takes none. The tasks stay Ready.
#[derive(Default)]
struct FullDispatcher {
    offered: Mutex<Vec<String>>,
}

impl FullDispatcher {
    /// The local task ids offered since the last call, in order.
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.offered.lock().unwrap())
    }
}

#[async_trait]
impl Dispatcher for FullDispatcher {
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
        let local = event
            .task_name
            .rsplit("::")
            .next()
            .unwrap_or_default()
            .to_string();
        self.offered.lock().unwrap().push(local);
        Err(DispatchError::NoCapacity("test".into()))
    }
    fn register_executor(&self, _key: &str, _executor: Arc<dyn TaskExecutor>) {}
    fn has_capacity(&self) -> bool {
        false
    }
    fn resolve_executor_key(&self, _task_name: &str) -> String {
        "test".into()
    }
    fn has_executor(&self, _key: &str) -> bool {
        true
    }
}

/// One tenant: its own database, runtime, scheduler and dispatcher.
struct Tenant {
    database: Database,
    runtime: Arc<Runtime>,
    scheduler: TaskScheduler,
    dispatcher: Arc<FullDispatcher>,
}

impl Tenant {
    async fn new() -> Self {
        let url = format!(
            "file:priority_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let database = Database::new(&url, "", 5);
        database.run_migrations().await.expect("migrations");
        let runtime = Arc::new(Runtime::empty());
        let dispatcher = Arc::new(FullDispatcher::default());
        let scheduler =
            TaskScheduler::with_poll_interval_sync(database.clone(), Duration::from_millis(100))
                .with_runtime(runtime.clone())
                .with_dispatcher(dispatcher.clone());
        Self {
            database,
            runtime,
            scheduler,
            dispatcher,
        }
    }

    /// Register a workflow `name` with one task of the same name, and
    /// schedule one execution of it.
    async fn run(&self, name: &str, workflow: Option<u8>, task: Option<u8>) -> uuid::Uuid {
        self.run_as(name, workflow, task, RunPriority::default())
            .await
            .0
    }

    async fn run_as(
        &self,
        name: &str,
        workflow: Option<u8>,
        task: Option<u8>,
        run: RunPriority,
    ) -> (uuid::Uuid, PriorityGrant) {
        let workflow_name = name.to_string();
        let task_name = name.to_string();
        self.runtime
            .register_workflow(workflow_name.clone(), move || {
                let mut builder = Workflow::builder(&workflow_name);
                if let Some(priority) = workflow {
                    builder = builder.priority(priority);
                }
                builder
                    .add_task(stub(&task_name, task))
                    .expect("add task")
                    .build()
                    .expect("build")
            });
        self.scheduler
            .schedule_workflow_execution_with_priority(name, Context::new(), run)
            .await
            .expect("schedule")
    }

    /// One pass of the scheduler loop. Returns the tasks offered to the
    /// dispatcher, in order.
    async fn pass(&self) -> Vec<String> {
        self.scheduler
            .process_active_executions()
            .await
            .expect("pass");
        self.dispatcher.take()
    }

    /// Make the task `name` wait since `seconds` ago.
    async fn waited(&self, name: &str, seconds: i64) {
        let since = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::seconds(seconds));
        let pattern = format!("%::{name}");
        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .expect("connection");
        let rows = conn
            .interact(move |conn| {
                diesel::update(
                    task_executions::table.filter(task_executions::task_name.like(pattern)),
                )
                .set(task_executions::waiting_since.eq(Some(since)))
                .execute(conn)
            })
            .await
            .expect("interact")
            .expect("update");
        assert_eq!(rows, 1, "one row for {name}");
    }

    async fn task_row(&self, name: &str) -> crate::models::task_execution::TaskExecution {
        let dal = DAL::new(self.database.clone());
        let executions = dal
            .workflow_execution()
            .get_active_executions()
            .await
            .expect("executions");
        for execution in executions {
            let rows = dal
                .task_execution()
                .get_all_tasks_for_workflow(execution.id)
                .await
                .expect("rows");
            if let Some(row) = rows
                .into_iter()
                .find(|t| t.task_name.ends_with(&format!("::{name}")))
            {
                return row;
            }
        }
        panic!("no task row for {name}");
    }
}

#[tokio::test]
async fn the_planner_writes_asked_and_given_on_the_execution_and_both_priorities_on_the_task() {
    let tenant = Tenant::new().await;

    // A workflow at 2 with a task at 0, and a task that declares none.
    let id = tenant.run("urgent", Some(2), Some(0)).await;
    let dal = DAL::new(tenant.database.clone());
    let execution = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(id))
        .await
        .expect("execution");
    assert_eq!(execution.priority_asked, Some(2));
    assert_eq!(execution.priority, 2);
    let row = tenant.task_row("urgent").await;
    assert_eq!((row.workflow_priority, row.task_priority), (2, 0));

    tenant.run("inherits", Some(7), None).await;
    let row = tenant.task_row("inherits").await;
    assert_eq!((row.workflow_priority, row.task_priority), (7, 7));

    // Nothing declared: the default, 5.
    let id = tenant.run("plain", None, None).await;
    let execution = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(id))
        .await
        .expect("execution");
    assert_eq!((execution.priority_asked, execution.priority), (Some(5), 5));
    let row = tenant.task_row("plain").await;
    assert_eq!((row.workflow_priority, row.task_priority), (5, 5));
}

#[tokio::test]
async fn a_run_that_asks_for_more_than_its_principal_may_set_is_lowered_not_refused() {
    let tenant = Tenant::new().await;
    // A principal that may set 5 at most asks for 1.
    let (id, grant) = tenant
        .run_as("asked", None, None, RunPriority::asked_by(1, Some(5)))
        .await;
    assert_eq!((grant.asked, grant.given), (1, 5));
    assert!(grant.notice().is_some());

    let dal = DAL::new(tenant.database.clone());
    let execution = dal
        .workflow_execution()
        .get_by_id(UniversalUuid(id))
        .await
        .expect("execution");
    assert_eq!((execution.priority_asked, execution.priority), (Some(1), 5));
    let row = tenant.task_row("asked").await;
    assert_eq!((row.workflow_priority, row.task_priority), (5, 5));

    // A principal that may set 0 gets what it asks for.
    let (_, grant) = tenant
        .run_as("admin", None, None, RunPriority::asked_by(1, Some(0)))
        .await;
    assert_eq!((grant.asked, grant.given), (1, 1));
    assert!(grant.notice().is_none());
}

/// The publisher of every package may set 5 at most.
struct PublisherLimitedTo(
    std::sync::atomic::AtomicU8,
    Mutex<Vec<(String, PriorityGrant)>>,
);

#[async_trait]
impl PriorityAuthority for PublisherLimitedTo {
    async fn declared_limit(&self, _workflow_name: &str) -> Option<u8> {
        Some(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
    async fn run_held(&self, workflow_name: &str, grant: PriorityGrant) {
        self.1
            .lock()
            .unwrap()
            .push((workflow_name.to_string(), grant));
    }
}

#[tokio::test]
async fn a_declared_priority_is_held_to_the_publishers_limit_as_it_is_when_the_run_starts() {
    let tenant = Tenant::new().await;
    let authority = Arc::new(PublisherLimitedTo(
        std::sync::atomic::AtomicU8::new(5),
        Mutex::new(Vec::new()),
    ));
    tenant.scheduler.set_priority_authority(authority.clone());

    // The package asks for 2. Its publisher may set 5: the run is at 5.
    let (_, grant) = tenant
        .run_as("declared", Some(2), Some(1), RunPriority::default())
        .await;
    assert_eq!((grant.asked, grant.given), (2, 5));
    let row = tenant.task_row("declared").await;
    assert_eq!((row.workflow_priority, row.task_priority), (5, 5));
    assert_eq!(authority.1.lock().unwrap().len(), 1, "the held run is told");

    // The publisher's limit is raised to 0. The next run is at 2, with no
    // new publish.
    authority.0.store(0, std::sync::atomic::Ordering::SeqCst);
    let (_, grant) = tenant
        .scheduler
        .schedule_workflow_execution_with_priority(
            "declared",
            Context::new(),
            RunPriority::default(),
        )
        .await
        .expect("schedule");
    assert_eq!((grant.asked, grant.given), (2, 2));
    assert_eq!(authority.1.lock().unwrap().len(), 1, "not held: not told");

    // Lowered again: the next run is held to the new limit.
    authority.0.store(7, std::sync::atomic::Ordering::SeqCst);
    let (_, grant) = tenant
        .scheduler
        .schedule_workflow_execution_with_priority(
            "declared",
            Context::new(),
            RunPriority::default(),
        )
        .await
        .expect("schedule");
    assert_eq!((grant.asked, grant.given), (2, 7));
}

/// The limit of each principal, by `set_by`, as a server reads it at the
/// run start; the changes it is told of.
#[derive(Default)]
struct ByPrincipal {
    limits: Mutex<std::collections::HashMap<String, u8>>,
    changes: Mutex<Vec<super::PriorityChange>>,
}

impl ByPrincipal {
    fn set(&self, by: &str, limit: u8) {
        self.limits.lock().unwrap().insert(by.to_string(), limit);
    }
    fn changes(&self) -> Vec<(String, u8, u8)> {
        self.changes
            .lock()
            .unwrap()
            .iter()
            .map(|c| (c.kind.clone(), c.grant.asked, c.grant.given))
            .collect()
    }
}

#[async_trait]
impl PriorityAuthority for ByPrincipal {
    async fn declared_limit(&self, _workflow_name: &str) -> Option<u8> {
        Some(5)
    }
    async fn limit_of(&self, setter: &crate::dal::unified::PrioritySetter) -> Option<u8> {
        self.limits.lock().unwrap().get(&setter.by).copied()
    }
    async fn given_changed(&self, change: &super::PriorityChange) {
        self.changes.lock().unwrap().push(change.clone());
    }
}

impl Tenant {
    /// Register `name` as the workflow of the package `package`.
    fn register_packaged(&self, name: &str, package: &str, workflow_priority: u8) {
        let (workflow_name, task_name, package) =
            (name.to_string(), name.to_string(), package.to_string());
        self.runtime
            .register_workflow(workflow_name.clone(), move || {
                let mut workflow = Workflow::builder(&workflow_name)
                    .priority(workflow_priority)
                    .add_task(stub(&task_name, None))
                    .expect("add task")
                    .build()
                    .expect("build");
                workflow.set_package(&package);
                workflow
            });
    }

    async fn start(&self, name: &str, run: RunPriority) -> PriorityGrant {
        self.scheduler
            .schedule_workflow_execution_with_priority(name, Context::new(), run)
            .await
            .expect("schedule")
            .1
    }
}

/// CLOACINA-T-1123: a package keeps who published it. At each run start its
/// declared priority is held to that publisher's limit as it is then; the
/// change of the given priority is told once, not at each run.
#[tokio::test]
async fn a_package_run_is_held_to_its_publishers_limit_now_and_a_change_is_told_once() {
    use crate::dal::unified::PrioritySetter;
    use crate::dal::unified::priority_grants::KIND_PACKAGE;

    let tenant = Tenant::new().await;
    let authority = Arc::new(ByPrincipal::default());
    tenant.scheduler.set_priority_authority(authority.clone());
    tenant.register_packaged("orders", "orders-pkg", 2);
    let dal = DAL::new(tenant.database.clone());

    // No record of the publisher (a package from before): the old default.
    assert_eq!(
        tenant.start("orders", RunPriority::default()).await.given,
        5
    );
    assert!(
        authority.changes().is_empty(),
        "nothing is recorded to change"
    );

    // Published by a key limited to 5: asks 2, runs at 5.
    dal.priority_grants()
        .record(
            KIND_PACKAGE,
            "orders-pkg",
            &PrioritySetter {
                by: "key:k1".into(),
                role: "write".into(),
            },
            None,
        )
        .await
        .unwrap();
    authority.set("key:k1", 5);
    let grant = tenant.start("orders", RunPriority::default()).await;
    assert_eq!((grant.asked, grant.given), (2, 5));
    assert_eq!(authority.changes(), vec![("package".to_string(), 2, 5)]);
    // The same again: no second report.
    tenant.start("orders", RunPriority::default()).await;
    assert_eq!(authority.changes().len(), 1);

    // The limit is raised to 0: the next run is at 2, with no new publish.
    authority.set("key:k1", 0);
    assert_eq!(
        tenant.start("orders", RunPriority::default()).await.given,
        2
    );
    // Lowered again: held to it.
    authority.set("key:k1", 7);
    assert_eq!(
        tenant.start("orders", RunPriority::default()).await.given,
        7
    );
    assert_eq!(
        authority.changes(),
        vec![
            ("package".to_string(), 2, 5),
            ("package".to_string(), 2, 2),
            ("package".to_string(), 2, 7)
        ]
    );
    let row = dal
        .priority_grants()
        .get(KIND_PACKAGE, "orders-pkg")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.last_given(), Some(7), "the state is in the database");

    // A run that asks for a priority itself is held to who asked.
    let grant = tenant
        .start("orders", RunPriority::asked_by(1, Some(0)))
        .await;
    assert_eq!(grant.given, 1);
    assert_eq!(authority.changes().len(), 3, "a request is not tracked");
}

/// CLOACINA-T-1123: a schedule created with a priority gives each fire that
/// priority, held to its creator's limit at the fire.
#[tokio::test]
async fn a_schedule_fire_is_held_to_its_creators_limit_now() {
    use crate::dal::unified::PrioritySetter;
    use crate::dal::unified::priority_grants::KIND_SCHEDULE;

    let tenant = Tenant::new().await;
    let authority = Arc::new(ByPrincipal::default());
    tenant.scheduler.set_priority_authority(authority.clone());
    tenant.register_packaged("nightly", "nightly-pkg", 8);
    let dal = DAL::new(tenant.database.clone());
    let schedule = uuid::Uuid::new_v4();

    // A schedule with no record fires at what the workflow declares.
    let grant = tenant
        .start("nightly", RunPriority::fired_by(schedule))
        .await;
    assert_eq!((grant.asked, grant.given), (8, 8));

    dal.priority_grants()
        .record(
            KIND_SCHEDULE,
            &schedule.to_string(),
            &PrioritySetter {
                by: "local:u1".into(),
                role: "write".into(),
            },
            Some(1),
        )
        .await
        .unwrap();
    authority.set("local:u1", 5);
    let grant = tenant
        .start("nightly", RunPriority::fired_by(schedule))
        .await;
    assert_eq!((grant.asked, grant.given), (1, 5));
    authority.set("local:u1", 0);
    let grant = tenant
        .start("nightly", RunPriority::fired_by(schedule))
        .await;
    assert_eq!((grant.asked, grant.given), (1, 1));
    assert_eq!(
        authority.changes(),
        vec![
            ("schedule".to_string(), 1, 5),
            ("schedule".to_string(), 1, 1)
        ]
    );

    // With no authority (the embedded use) a schedule's record is not read.
    let embedded = Tenant::new().await;
    embedded.register_packaged("nightly", "nightly-pkg", 8);
    assert_eq!(
        embedded
            .start("nightly", RunPriority::fired_by(schedule))
            .await
            .given,
        8
    );
}

/// CLOACINA-T-1123: the task row keeps the priority its task asked for
/// beside the one it was given; a task that declares none asks nothing.
#[tokio::test]
async fn the_task_row_keeps_the_asked_task_priority_beside_the_given_one() {
    let tenant = Tenant::new().await;
    tenant
        .scheduler
        .set_priority_authority(Arc::new(PublisherLimitedTo(
            std::sync::atomic::AtomicU8::new(5),
            Mutex::new(Vec::new()),
        )));

    tenant.run("held", Some(8), Some(1)).await;
    let row = tenant.task_row("held").await;
    assert_eq!(
        (row.task_priority_asked, row.task_priority),
        (Some(1), 5),
        "asked 1, held to the publisher's 5"
    );

    tenant.run("lower", Some(8), Some(9)).await;
    let row = tenant.task_row("lower").await;
    assert_eq!((row.task_priority_asked, row.task_priority), (Some(9), 9));

    tenant.run("inherits", Some(8), None).await;
    let row = tenant.task_row("inherits").await;
    assert_eq!((row.task_priority_asked, row.task_priority), (None, 8));
}

#[tokio::test]
async fn one_tenant_dispatches_by_workflow_priority_then_task_priority_then_time_waited() {
    let tenant = Tenant::new().await;
    tenant.run("low", Some(8), None).await;
    tenant.run("default_a", None, None).await;
    tenant.run("default_b", None, None).await;
    tenant.run("default_urgent_task", None, Some(1)).await;
    tenant.run("high", Some(2), None).await;

    // The first pass makes the tasks Ready. Then each gets a known wait.
    tenant.pass().await;
    tenant.waited("low", 100).await;
    tenant.waited("default_a", 50).await;
    tenant.waited("default_b", 40).await;
    tenant.waited("default_urgent_task", 5).await;
    tenant.waited("high", 1).await;

    assert_eq!(
        tenant.pass().await,
        vec![
            "high",
            "default_urgent_task",
            "default_a",
            "default_b",
            "low"
        ]
    );
}

#[tokio::test]
async fn the_default_priority_keeps_todays_order_oldest_first() {
    let tenant = Tenant::new().await;
    tenant.run("c", None, None).await;
    tenant.run("a", None, None).await;
    tenant.run("b", None, None).await;
    tenant.pass().await;
    tenant.waited("a", 90).await;
    tenant.waited("b", 60).await;
    tenant.waited("c", 30).await;

    assert_eq!(tenant.pass().await, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn ageing_lifts_old_low_priority_work_past_newer_higher_priority_work() {
    let tenant = Tenant::new().await;
    assert_eq!(
        cloacina_api_types::DEFAULT_PRIORITY_AGING_INTERVAL,
        Duration::from_secs(120),
        "the default interval is two minutes"
    );
    tenant.run("low", Some(10), None).await;
    tenant.run("fresh", None, None).await;
    tenant.pass().await;

    // Priority 10 against a fresh priority 5: five intervals of two minutes
    // close the gap. One second before that, the fresh task still goes first.
    tenant.waited("low", 10 * 60 - 1).await;
    tenant.waited("fresh", 0).await;
    assert_eq!(tenant.pass().await, vec!["fresh", "low"]);

    // After ten minutes the two are level, and the older one goes first.
    tenant.waited("low", 10 * 60 + 1).await;
    tenant.waited("fresh", 0).await;
    assert_eq!(tenant.pass().await, vec!["low", "fresh"]);

    // The stored priority did not change.
    let row = tenant.task_row("low").await;
    assert_eq!((row.workflow_priority, row.task_priority), (10, 10));
}

#[tokio::test]
async fn two_tenants_each_have_their_own_queue() {
    let acme = Tenant::new().await;
    let globex = Tenant::new().await;

    // Acme has work at the highest priority. Globex has default work only.
    acme.run("acme_top", Some(0), Some(0)).await;
    globex.run("globex_old", None, None).await;
    globex.run("globex_new", None, None).await;
    acme.pass().await;
    globex.pass().await;
    globex.waited("globex_old", 60).await;
    globex.waited("globex_new", 10).await;

    // Each pass offers only the tenant's own tasks, in the tenant's own
    // order. Acme's priority 0 is not in Globex's queue.
    assert_eq!(globex.pass().await, vec!["globex_old", "globex_new"]);
    assert_eq!(acme.pass().await, vec!["acme_top"]);
}

#[tokio::test]
async fn a_running_task_is_not_stopped_for_more_important_work() {
    let tenant = Tenant::new().await;
    tenant.run("running_low", Some(10), None).await;
    tenant.pass().await;

    // The low priority task takes the one slot and starts.
    let dal = DAL::new(tenant.database.clone());
    let low = tenant.task_row("running_low").await;
    let runner = UniversalUuid::new_v4();
    dal.task_execution()
        .claim_for_runner(low.id, runner)
        .await
        .expect("claim");
    dal.task_execution()
        .mark_started(low.id)
        .await
        .expect("start");

    // Work at the highest priority arrives. It is the only task offered:
    // the running task is not in the queue and nothing touches it.
    tenant.run("top", Some(0), Some(0)).await;
    assert_eq!(tenant.pass().await, vec!["top"]);
    let low = tenant.task_row("running_low").await;
    assert_eq!(low.status, "Running");
    assert_eq!(low.claimed_by, Some(runner));
    assert!(low.started_at.is_some());
}

#[tokio::test]
async fn the_in_process_executor_leaves_a_task_in_the_queue_when_it_has_no_free_slot() {
    use crate::executor::ThreadTaskExecutor;
    use crate::executor::types::ExecutorConfig;

    let tenant = Tenant::new().await;
    tenant.run("waits", None, None).await;
    tenant.pass().await;
    let row = tenant.task_row("waits").await;

    let executor = ThreadTaskExecutor::with_runtime_and_registry(
        tenant.database.clone(),
        Arc::new(crate::TaskRegistry::new()),
        tenant.runtime.clone(),
        ExecutorConfig {
            max_concurrent_tasks: 1,
            enable_claiming: true,
            ..Default::default()
        },
    );
    // The one slot is busy.
    let busy = executor
        .semaphore()
        .clone()
        .try_acquire_owned()
        .expect("the slot");

    let event = TaskReadyEvent::new(
        row.id,
        row.workflow_execution_id,
        row.task_name.clone(),
        row.attempt,
    );
    let outcome = TaskExecutor::execute(&executor, event).await;
    assert!(
        matches!(outcome, Err(DispatchError::NoCapacity(_))),
        "{outcome:?}"
    );

    // The task was not claimed: it is still in the waiting set, where the
    // next pass orders it again.
    let row = tenant.task_row("waits").await;
    assert_eq!(row.status, "Ready");
    assert_eq!(row.claimed_by, None);
    assert_eq!(tenant.pass().await, vec!["waits"]);
    drop(busy);
}
