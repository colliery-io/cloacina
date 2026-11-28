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

//! The end of the output of each attempt of a task (CLOACINA-T-1091).
//!
//! The fleet executor writes the row of an attempt when the wait of its
//! dispatch takes the agent's report. Output retention deletes rows by their
//! own age ([`delete_attempt_outputs_older_than`]); run-history retention
//! deletes them with their run.
//!
//! [`delete_attempt_outputs_older_than`]: TaskExecutionDAL::delete_attempt_outputs_older_than

use super::TaskExecutionDAL;
use crate::dal::unified::models::TaskAttemptOutput;
use crate::database::schema::unified::task_attempt_outputs;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::fleet::AttemptOutput;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Store the output of `attempt` of a task.
    pub async fn record_attempt_output(
        &self,
        task_execution_id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
        attempt: i32,
        output: AttemptOutput,
    ) -> Result<(), ValidationError> {
        let row = TaskAttemptOutput {
            id: UniversalUuid::new_v4(),
            task_execution_id,
            workflow_execution_id,
            attempt,
            output: output.text,
            total_bytes: i64::try_from(output.total_bytes).unwrap_or(i64::MAX),
            cut: UniversalBool::from(output.cut),
            recorded_at: UniversalTimestamp::now(),
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(task_attempt_outputs::table)
                .values(&row)
                .execute(conn)
        })?;
        Ok(())
    }

    /// The outputs of one task, in attempt order; with `attempt`, of that
    /// attempt only.
    pub async fn attempt_outputs(
        &self,
        task_execution_id: UniversalUuid,
        attempt: Option<i32>,
    ) -> Result<Vec<TaskAttemptOutput>, ValidationError> {
        let rows: Vec<TaskAttemptOutput> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = task_attempt_outputs::table
                .filter(task_attempt_outputs::task_execution_id.eq(task_execution_id))
                .into_boxed();
            if let Some(attempt) = attempt {
                query = query.filter(task_attempt_outputs::attempt.eq(attempt));
            }
            query
                .order((
                    task_attempt_outputs::attempt.asc(),
                    task_attempt_outputs::recorded_at.asc(),
                ))
                .select(TaskAttemptOutput::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// Delete up to `limit` output rows recorded before `cutoff`. Returns how
    /// many went.
    pub async fn delete_attempt_outputs_older_than(
        &self,
        cutoff: UniversalTimestamp,
        limit: i64,
    ) -> Result<usize, ValidationError> {
        let limit = limit.clamp(1, 5_000);
        let deleted: usize = crate::interact_on_backend!(self.dal, |conn| {
            let ids: Vec<UniversalUuid> = task_attempt_outputs::table
                .filter(task_attempt_outputs::recorded_at.lt(cutoff))
                .select(task_attempt_outputs::id)
                .limit(limit)
                .load(conn)?;
            if ids.is_empty() {
                return Ok(0);
            }
            diesel::delete(task_attempt_outputs::table.filter(task_attempt_outputs::id.eq_any(ids)))
                .execute(conn)
        })?;
        Ok(deleted)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::dal::DAL;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::workflow_execution::NewWorkflowExecution;

    async fn task() -> (DAL, UniversalUuid, UniversalUuid) {
        let url = format!(
            "file:attempt_output_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        let dal = DAL::new(db);
        let workflow_id = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "orders".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id;
        let task_id = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: "public::embedded::orders::load".into(),
                status: "Running".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        (dal, workflow_id, task_id)
    }

    fn output(text: &str, cut: bool) -> AttemptOutput {
        AttemptOutput {
            text: text.into(),
            total_bytes: 1000,
            cut,
        }
    }

    #[tokio::test]
    async fn outputs_are_kept_per_attempt_and_read_in_attempt_order() {
        let (dal, wf, task) = task().await;
        let tasks = dal.task_execution();
        tasks
            .record_attempt_output(task, wf, 2, output("second", false))
            .await
            .unwrap();
        tasks
            .record_attempt_output(task, wf, 1, output("first", true))
            .await
            .unwrap();

        let all = tasks.attempt_outputs(task, None).await.unwrap();
        let seen: Vec<(i32, &str, bool, i64)> = all
            .iter()
            .map(|r| (r.attempt, r.output.as_str(), r.cut.into(), r.total_bytes))
            .collect();
        assert_eq!(
            seen,
            vec![(1, "first", true, 1000), (2, "second", false, 1000)]
        );

        let second = tasks.attempt_outputs(task, Some(2)).await.unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].output, "second");
        assert!(
            tasks
                .attempt_outputs(task, Some(3))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn only_rows_older_than_the_cutoff_are_deleted() {
        let (dal, wf, task) = task().await;
        let tasks = dal.task_execution();
        tasks
            .record_attempt_output(task, wf, 1, output("old", false))
            .await
            .unwrap();
        let cutoff = UniversalTimestamp::now();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tasks
            .record_attempt_output(task, wf, 2, output("new", false))
            .await
            .unwrap();

        assert_eq!(
            tasks
                .delete_attempt_outputs_older_than(cutoff, 100)
                .await
                .unwrap(),
            1
        );
        let left = tasks.attempt_outputs(task, None).await.unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].output, "new");
    }
}
