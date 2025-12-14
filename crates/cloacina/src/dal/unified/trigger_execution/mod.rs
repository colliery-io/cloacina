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

//! Unified Trigger Execution DAL with runtime backend selection
//!
//! This module provides operations for TriggerExecution entities that work with
//! both PostgreSQL and SQLite backends.

mod crud;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::trigger_execution::{NewTriggerExecution, TriggerExecution};
use chrono::{DateTime, Utc};

/// Data access layer for trigger execution operations with runtime backend selection.
#[derive(Clone)]
pub struct TriggerExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TriggerExecutionDAL<'a> {
    /// Creates a new TriggerExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new trigger execution record in the database.
    pub async fn create(
        &self,
        new_execution: NewTriggerExecution,
    ) -> Result<TriggerExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_execution).await,
            self.create_sqlite(new_execution).await
        )
    }

    /// Retrieves a trigger execution by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<TriggerExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Checks if there's an active (incomplete) execution for a trigger with the given context hash.
    /// Returns true if an active execution exists.
    pub async fn has_active_execution(
        &self,
        trigger_name: &str,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.has_active_execution_postgres(trigger_name, context_hash)
                .await,
            self.has_active_execution_sqlite(trigger_name, context_hash)
                .await
        )
    }

    /// Marks an execution as completed.
    pub async fn complete(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.complete_postgres(id, completed_at).await,
            self.complete_sqlite(id, completed_at).await
        )
    }

    /// Links a trigger execution to a pipeline execution.
    pub async fn link_pipeline_execution(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.link_pipeline_execution_postgres(id, pipeline_execution_id)
                .await,
            self.link_pipeline_execution_sqlite(id, pipeline_execution_id)
                .await
        )
    }

    /// Retrieves recent executions for a trigger.
    pub async fn get_recent(
        &self,
        trigger_name: &str,
        limit: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_recent_postgres(trigger_name, limit).await,
            self.get_recent_sqlite(trigger_name, limit).await
        )
    }

    /// Lists executions for a trigger with pagination.
    pub async fn list_by_trigger(
        &self,
        trigger_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_trigger_postgres(trigger_name, limit, offset)
                .await,
            self.list_by_trigger_sqlite(trigger_name, limit, offset)
                .await
        )
    }

    /// Marks all incomplete executions for a pipeline as completed.
    /// Used when a pipeline completes to ensure trigger execution tracking is updated.
    pub async fn complete_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.complete_by_pipeline_postgres(pipeline_execution_id, completed_at)
                .await,
            self.complete_by_pipeline_sqlite(pipeline_execution_id, completed_at)
                .await
        )
    }
}
