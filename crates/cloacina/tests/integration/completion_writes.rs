//! Completion writes (COLLIERY-I-0632 / CLOACINA-T-0974): rows a task attaches
//! under the reserved key are written in the same transaction that marks the
//! task completed, the key never reaches the saved context, and a payload the
//! host cannot write leaves the task not completed.

use cloacina::dal::DAL;
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{Context, TaskError, task, workflow};
use cloacina_workflow::completion_writes::{
    COMPLETION_WRITES_KEY, CompletionWrites, DeadLetterWrite, StreamStateWrite,
};
use serde_json::json;

#[workflow(
    name = "completion_writes_test_workflow",
    description = "One task that attaches completion writes",
    author = "Test"
)]
pub mod completion_writes_test_workflow {
    use super::*;

    #[task(id = "write_segment", dependencies = [])]
    pub async fn write_segment(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("segment_written", json!(true))?;
        let writes = CompletionWrites {
            stream_state: vec![StreamStateWrite {
                sync: "orders".into(),
                stream: "public.orders".into(),
                partition: String::new(),
                cursor: Some("\"2026-10-16T00:00:00Z\"".into()),
                opaque: vec![7, 7],
                exhausted: true,
            }],
            dead_letters: vec![DeadLetterWrite {
                sync: "orders".into(),
                stream: "public.orders".into(),
                record: r#"{"id":"x"}"#.into(),
                reason: "id is not an integer".into(),
            }],
            stream_schemas: vec![],
        };
        writes
            .attach(context)
            .map_err(|e| TaskError::ExecutionFailed {
                message: e.to_string(),
                task_id: "write_segment".into(),
                timestamp: chrono::Utc::now(),
            })?;
        Ok(())
    }
}

#[workflow(
    name = "completion_writes_bad_payload_workflow",
    description = "One task whose completion writes cannot be applied",
    author = "Test"
)]
pub mod completion_writes_bad_payload_workflow {
    use super::*;

    #[task(id = "bad_segment", dependencies = [])]
    pub async fn bad_segment(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // An empty stream name has no home: the host must refuse the whole
        // transaction rather than land an orphan row.
        let writes = CompletionWrites {
            stream_state: vec![StreamStateWrite {
                sync: "orders".into(),
                stream: String::new(),
                ..Default::default()
            }],
            dead_letters: vec![],
            stream_schemas: vec![],
        };
        writes
            .attach(context)
            .map_err(|e| TaskError::ExecutionFailed {
                message: e.to_string(),
                task_id: "bad_segment".into(),
                timestamp: chrono::Utc::now(),
            })?;
        Ok(())
    }
}

/// The naming convention of the integration lane: a test with `sqlite` in its
/// name runs in the SQLite lane, every other test in the Postgres lane.
#[cfg(feature = "sqlite")]
async fn sqlite_runner() -> DefaultRunner {
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    DefaultRunner::with_config(":memory:", config)
        .await
        .expect("sqlite runner")
}

/// A Postgres runner in a fresh schema per test, against the dev service on
/// port 15432 (`angreal services up`).
#[cfg(feature = "postgres")]
async fn postgres_runner(schema: &str) -> DefaultRunner {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://cloacina:cloacina@localhost:15432/cloacina".to_string());
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    DefaultRunner::builder()
        .database_url(&database_url)
        .schema(schema)
        .with_config(config)
        .build()
        .await
        .expect("postgres runner")
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn completion_writes_land_with_the_completion_and_leave_the_context_sqlite() {
    let runner = sqlite_runner().await;
    writes_land_with_the_completion(runner).await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn completion_writes_land_with_the_completion_and_leave_the_context() {
    let runner = postgres_runner("cw_land").await;
    writes_land_with_the_completion(runner).await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn a_payload_the_host_cannot_write_does_not_complete_the_task_sqlite() {
    let runner = sqlite_runner().await;
    bad_payload_does_not_complete(runner).await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn a_payload_the_host_cannot_write_does_not_complete_the_task() {
    let runner = postgres_runner("cw_bad").await;
    bad_payload_does_not_complete(runner).await;
}

async fn writes_land_with_the_completion(runner: DefaultRunner) {
    let dal = DAL::new(runner.database().clone());
    // A previous run in the same Postgres schema may have left rows.
    dal.sync_stream_state().reset_sync("orders").await.unwrap();
    dal.sync_dead_letters().clear_sync("orders").await.unwrap();

    let result = runner
        .execute("completion_writes_test_workflow", Context::new())
        .await
        .expect("workflow ran");
    assert_eq!(
        result.status,
        cloacina::executor::WorkflowStatus::Completed,
        "the task must complete"
    );

    // The cursor landed, once, with the committed values.
    let state = dal
        .sync_stream_state()
        .get("orders", "public.orders", "")
        .await
        .unwrap()
        .expect("stream state row");
    assert_eq!(state.cursor.as_deref(), Some("\"2026-10-16T00:00:00Z\""));
    assert_eq!(state.opaque.0, vec![7, 7]);
    assert!(state.exhausted.0);
    assert_eq!(
        dal.sync_stream_state()
            .list_for_sync("orders")
            .await
            .unwrap()
            .len(),
        1
    );

    // The dead-letter landed, stamped with the task and workflow execution.
    let dead = dal
        .sync_dead_letters()
        .list_for_sync("orders", 10)
        .await
        .unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].reason, "id is not an integer");
    assert_eq!(
        dead[0].workflow_execution_id.map(|u| u.0),
        Some(result.execution_id)
    );
    assert!(dead[0].task_execution_id.is_some());
    assert_eq!(
        dal.sync_dead_letters()
            .count_for_sync("orders")
            .await
            .unwrap(),
        1
    );

    // The reserved key never reached the saved context; the task's own key did.
    let final_context = result.final_context;
    assert_eq!(final_context.get("segment_written"), Some(&json!(true)));
    assert!(
        final_context.get(COMPLETION_WRITES_KEY).is_none(),
        "the reserved key must be stripped before the context is saved"
    );

    runner.shutdown().await.unwrap();
}

async fn bad_payload_does_not_complete(runner: DefaultRunner) {
    let dal = DAL::new(runner.database().clone());
    dal.sync_stream_state().reset_sync("orders").await.unwrap();

    let result = runner
        .execute("completion_writes_bad_payload_workflow", Context::new())
        .await
        .expect("workflow ran to a terminal state");
    assert_ne!(
        result.status,
        cloacina::executor::WorkflowStatus::Completed,
        "a task whose completion writes fail must not be completed"
    );
    assert!(
        dal.sync_stream_state()
            .list_for_sync("orders")
            .await
            .unwrap()
            .is_empty(),
        "nothing may land when the transaction rolls back"
    );

    runner.shutdown().await.unwrap();
}
