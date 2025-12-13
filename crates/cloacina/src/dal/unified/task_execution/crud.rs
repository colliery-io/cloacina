/*
 *  Copyright 2025 Colliery Software
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

//! CRUD operations for task executions.

use super::TaskExecutionDAL;
use crate::dal::unified::models::{NewUnifiedTaskExecution, UnifiedTaskExecution};
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_execution::{NewTaskExecution, TaskExecution};
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Creates a new task execution record in the database.
    pub async fn create(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_task).await,
            self.create_sqlite(new_task).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified_task = NewUnifiedTaskExecution {
            id,
            pipeline_execution_id: new_task.pipeline_execution_id,
            task_name: new_task.task_name,
            status: new_task.status,
            attempt: new_task.attempt,
            max_attempts: new_task.max_attempts,
            trigger_rules: new_task.trigger_rules,
            task_configuration: new_task.task_configuration,
            created_at: now,
            updated_at: now,
        };

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| {
                diesel::insert_into(task_executions::table)
                    .values(&new_unified_task)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified_task = NewUnifiedTaskExecution {
            id,
            pipeline_execution_id: new_task.pipeline_execution_id,
            task_name: new_task.task_name,
            status: new_task.status,
            attempt: new_task.attempt,
            max_attempts: new_task.max_attempts,
            trigger_rules: new_task.trigger_rules,
            task_configuration: new_task.task_configuration,
            created_at: now,
            updated_at: now,
        };

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| {
                diesel::insert_into(task_executions::table)
                    .values(&new_unified_task)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    /// Retrieves a specific task execution by its ID.
    pub async fn get_by_id(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(task_id).await,
            self.get_by_id_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_id_postgres(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| task_executions::table.find(task_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_id_sqlite(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| task_executions::table.find(task_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    /// Retrieves all tasks associated with a pipeline execution.
    pub async fn get_all_tasks_for_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_all_tasks_for_pipeline_postgres(pipeline_execution_id)
                .await,
            self.get_all_tasks_for_pipeline_sqlite(pipeline_execution_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_all_tasks_for_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_all_tasks_for_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }
}
