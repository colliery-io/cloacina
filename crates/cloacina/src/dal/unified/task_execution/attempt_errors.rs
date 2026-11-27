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

//! The errors of the failed attempts of a task (CLOACINA-T-1100).
//!
//! [`schedule_retry`](TaskExecutionDAL::schedule_retry) writes the row of an
//! attempt that is retried, [`mark_failed`](TaskExecutionDAL::mark_failed)
//! the row of the attempt that fails the task. Run-history retention deletes
//! them with their run.

use super::TaskExecutionDAL;
use crate::dal::unified::models::TaskAttemptError;
use crate::database::schema::unified::task_attempt_errors;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use diesel::prelude::*;
use std::collections::HashMap;

impl<'a> TaskExecutionDAL<'a> {
    /// The errors of the failed attempts of one task, in attempt order.
    pub async fn attempt_errors(
        &self,
        task_id: UniversalUuid,
    ) -> Result<Vec<TaskAttemptError>, ValidationError> {
        let rows: Vec<TaskAttemptError> = crate::interact_on_backend!(self.dal, |conn| {
            task_attempt_errors::table
                .filter(task_attempt_errors::task_execution_id.eq(task_id))
                .order((
                    task_attempt_errors::attempt.asc(),
                    task_attempt_errors::failed_at.asc(),
                ))
                .select(TaskAttemptError::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// The errors of the failed attempts of every task of a workflow
    /// execution, by task execution id, each list in attempt order.
    pub async fn attempt_errors_for_workflow(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<HashMap<UniversalUuid, Vec<TaskAttemptError>>, ValidationError> {
        let rows: Vec<TaskAttemptError> = crate::interact_on_backend!(self.dal, |conn| {
            task_attempt_errors::table
                .filter(task_attempt_errors::workflow_execution_id.eq(workflow_execution_id))
                .order((
                    task_attempt_errors::attempt.asc(),
                    task_attempt_errors::failed_at.asc(),
                ))
                .select(TaskAttemptError::as_select())
                .load(conn)
        })?;
        let mut by_task: HashMap<UniversalUuid, Vec<TaskAttemptError>> = HashMap::new();
        for row in rows {
            by_task.entry(row.task_execution_id).or_default().push(row);
        }
        Ok(by_task)
    }
}
