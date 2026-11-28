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

//! The ordered dispatch on Postgres (CLOACINA-T-1123). On Postgres the
//! scheduler loop spawns the dispatches of a pass at the same time. Each one
//! waits for the one before it to decide about its slot (its turn), so the
//! free slots go to the tasks in priority order; and a new pass dispatches
//! nothing while the dispatches of the pass before still take their turns.
//!
//! The test runs the real scheduling loop on its own schema, with a
//! dispatcher that has no free slot. The dispatcher takes longer to decide
//! for the more important tasks: without the turns, the least important task
//! would decide first.

use async_trait::async_trait;
use cloacina::dispatcher::{DispatchError, Dispatcher, TaskExecutor, TaskReadyEvent};
use cloacina::execution_planner::{RunPriority, TaskScheduler};
use cloacina::*;
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Noop {
    id: String,
}

#[async_trait]
impl Task for Noop {
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
}

/// What the dispatcher saw, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    /// A dispatch of the task began.
    Offered(String),
    /// The dispatch decided: no free slot.
    Decided(String),
}

/// A dispatcher with no free slot. It takes `delay_of(task)` to decide, then
/// releases the turn and answers `NoCapacity`: the task stays Ready.
#[derive(Default)]
struct SlowFullDispatcher {
    seen: Mutex<Vec<Seen>>,
}

fn local_name(task_name: &str) -> String {
    task_name
        .rsplit("::")
        .next()
        .unwrap_or_default()
        .to_string()
}

fn delay_of(task: &str) -> Duration {
    match task {
        "high" => Duration::from_millis(600),
        "default" => Duration::from_millis(300),
        _ => Duration::ZERO,
    }
}

#[async_trait]
impl Dispatcher for SlowFullDispatcher {
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
        let name = local_name(&event.task_name);
        self.seen.lock().unwrap().push(Seen::Offered(name.clone()));
        tokio::time::sleep(delay_of(&name)).await;
        self.seen.lock().unwrap().push(Seen::Decided(name));
        event.admission.release();
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

/// Drop the test's schema.
async fn drop_schema(database: &Database, schema: &str) {
    use diesel::RunQueryDsl;
    let sql = format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE");
    let conn = database
        .get_postgres_connection()
        .await
        .expect("connection");
    let _ = conn
        .interact(move |conn| diesel::sql_query(sql).execute(conn))
        .await;
}

#[tokio::test]
#[serial]
async fn postgres_dispatches_a_pass_in_priority_order_and_a_new_pass_waits() {
    let schema = format!("t1123_order_{}", uuid::Uuid::new_v4().simple());
    let database = Database::new_with_schema(
        "postgres://cloacina:cloacina@localhost:15432",
        "cloacina",
        5,
        Some(&schema),
    );
    database
        .setup_schema(&schema)
        .await
        .expect("set up the test schema");

    // Three workflows of one task each: priority 8, the default, and 2.
    let runtime = Arc::new(cloacina::Runtime::empty());
    for (name, priority) in [("low", Some(8u8)), ("default", None), ("high", Some(2))] {
        let task: Arc<dyn Task> = Arc::new(Noop {
            id: name.to_string(),
        });
        let mut builder = Workflow::builder(name);
        if let Some(priority) = priority {
            builder = builder.priority(priority);
        }
        let workflow = builder
            .add_task(task)
            .expect("add task")
            .build()
            .expect("build");
        runtime.register_workflow(name.to_string(), move || workflow.clone());
    }

    let dispatcher = Arc::new(SlowFullDispatcher::default());
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let scheduler = TaskScheduler::with_poll_interval(database.clone(), Duration::from_millis(50))
        .await
        .expect("scheduler")
        .with_runtime(runtime)
        .with_dispatcher(dispatcher.clone())
        .with_shutdown(stopped);
    // The least important run is made first, so it has waited longest.
    for name in ["low", "default", "high"] {
        scheduler
            .schedule_workflow_execution_with_priority(name, Context::new(), RunPriority::default())
            .await
            .expect("schedule");
    }

    let scheduler = Arc::new(scheduler);
    let looping = {
        let scheduler = scheduler.clone();
        tokio::spawn(async move { scheduler.run_scheduling_loop().await })
    };

    // Wait for the first pass that offers all three, and then some more
    // passes: the loop ticks every 50 ms, a pass takes 900 ms to decide.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let decided = dispatcher
            .seen
            .lock()
            .unwrap()
            .iter()
            .filter(|s| matches!(s, Seen::Decided(_)))
            .count();
        if decided >= 6 || tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let _ = stop.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(10), looping).await;
    let seen = dispatcher.seen.lock().unwrap().clone();
    drop_schema(&database, &schema).await;

    let decided: Vec<&str> = seen
        .iter()
        .filter_map(|s| match s {
            Seen::Decided(name) => Some(name.as_str()),
            Seen::Offered(_) => None,
        })
        .collect();
    assert!(decided.len() >= 6, "two passes decided: {seen:?}");
    // The turns: the tasks decide in priority order, though the most
    // important one takes longest to decide.
    assert_eq!(&decided[..3], ["high", "default", "low"], "{seen:?}");
    assert_eq!(&decided[3..6], ["high", "default", "low"], "{seen:?}");

    // The first pass offers each task once. A new pass offers nothing until
    // every dispatch of the pass before has decided.
    let third_decision = seen
        .iter()
        .position(|s| *s == Seen::Decided("low".into()))
        .expect("the first pass decided");
    let offers_before: Vec<&Seen> = seen[..third_decision]
        .iter()
        .filter(|s| matches!(s, Seen::Offered(_)))
        .collect();
    assert_eq!(
        offers_before.len(),
        3,
        "no task is offered twice while the first pass takes its turns: {seen:?}"
    );
}
