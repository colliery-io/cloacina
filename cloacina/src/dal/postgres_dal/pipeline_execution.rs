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

use super::DAL;
use crate::database::schema::pipeline_executions;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::pipeline_execution::{NewPipelineExecution, PipelineExecution};
use diesel::prelude::*;

/// Data Access Layer for managing pipeline executions in the database.
///
/// This struct provides methods for creating, retrieving, updating, and managing
/// pipeline execution records. It handles all database operations related to
/// pipeline executions including status updates, recovery operations, and
/// execution tracking.
pub struct PipelineExecutionDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> PipelineExecutionDAL<'a> {
    /// Creates a new pipeline execution record in the database.
    ///
    /// # Arguments
    /// * `new_execution` - A `NewPipelineExecution` struct containing the execution details
    ///
    /// # Returns
    /// * `Result<PipelineExecution, ValidationError>` - The created pipeline execution or an error
    pub async fn create(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let execution: PipelineExecution = conn.interact(move |conn| {
            diesel::insert_into(pipeline_executions::table)
                .values(&new_execution)
                .get_result(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution)
    }

    /// Retrieves a pipeline execution by its unique identifier.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to retrieve
    ///
    /// # Returns
    /// * `Result<PipelineExecution, ValidationError>` - The pipeline execution or an error if not found
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<PipelineExecution, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let execution = conn.interact(move |conn| {
            pipeline_executions::table.find(id).first(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution)
    }

    /// Retrieves all active pipeline executions (status is either "Pending" or "Running").
    ///
    /// # Returns
    /// * `Result<Vec<PipelineExecution>, ValidationError>` - Vector of active pipeline executions
    pub async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let executions = conn.interact(move |conn| {
            pipeline_executions::table
                .filter(pipeline_executions::status.eq_any(vec!["Pending", "Running"]))
                .load(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions)
    }

    /// Updates the status of a pipeline execution.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to update
    /// * `status` - The new status to set
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_status(&self, id: UniversalUuid, status: &str) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let status = status.to_string();

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set(pipeline_executions::status.eq(status))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a pipeline execution as completed and sets the completion timestamp.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to mark as completed
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Completed"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Retrieves the most recent version of a pipeline by name.
    ///
    /// # Arguments
    /// * `pipeline_name` - The name of the pipeline to check
    ///
    /// # Returns
    /// * `Result<Option<String>, ValidationError>` - The most recent version string or None if no executions exist
    pub async fn get_last_version(&self, pipeline_name: &str) -> Result<Option<String>, ValidationError> {
        let conn = self.dal.pool.get().await?;
        let pipeline_name = pipeline_name.to_string();

        let version: Option<String> = conn.interact(move |conn| {
            pipeline_executions::table
                .filter(pipeline_executions::pipeline_name.eq(pipeline_name))
                .order(pipeline_executions::started_at.desc())
                .select(pipeline_executions::pipeline_version)
                .first(conn)
                .optional()
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(version)
    }

    /// Marks a pipeline as failed and records the failure reason.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to mark as failed
    /// * `reason` - A string describing the reason for failure
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn mark_failed(&self, id: UniversalUuid, reason: &str) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let reason = reason.to_string();

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Failed"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                    pipeline_executions::error_details.eq(reason),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Increments the recovery attempt counter for a pipeline execution.
    /// Also updates the last recovery timestamp.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to update
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn increment_recovery_attempts(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::recovery_attempts
                        .eq(pipeline_executions::recovery_attempts + 1),
                    pipeline_executions::last_recovery_at.eq(diesel::dsl::now),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Cancels a pipeline execution and marks it as cancelled.
    ///
    /// # Arguments
    /// * `id` - The UUID of the pipeline execution to cancel
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Cancelled"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Retrieves a list of recent pipeline executions, ordered by start time.
    ///
    /// # Arguments
    /// * `limit` - The maximum number of executions to retrieve
    ///
    /// # Returns
    /// * `Result<Vec<PipelineExecution>, ValidationError>` - Vector of recent pipeline executions
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let executions = conn.interact(move |conn| {
            pipeline_executions::table
                .order(pipeline_executions::started_at.desc())
                .limit(limit)
                .load(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions)
    }
}
