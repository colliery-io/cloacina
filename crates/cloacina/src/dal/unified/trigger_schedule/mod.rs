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

//! Unified Trigger Schedule DAL with runtime backend selection
//!
//! This module provides operations for TriggerSchedule entities that work with
//! both PostgreSQL and SQLite backends.

mod crud;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::trigger_schedule::{NewTriggerSchedule, TriggerSchedule};
use chrono::{DateTime, Utc};

/// Data access layer for trigger schedule operations with runtime backend selection.
#[derive(Clone)]
pub struct TriggerScheduleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TriggerScheduleDAL<'a> {
    /// Creates a new TriggerScheduleDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new trigger schedule record in the database.
    pub async fn create(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_schedule).await,
            self.create_sqlite(new_schedule).await
        )
    }

    /// Retrieves a trigger schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<TriggerSchedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Retrieves a trigger schedule by its name.
    pub async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Option<TriggerSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_name_postgres(name).await,
            self.get_by_name_sqlite(name).await
        )
    }

    /// Retrieves all enabled trigger schedules.
    pub async fn get_enabled(&self) -> Result<Vec<TriggerSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_enabled_postgres().await,
            self.get_enabled_sqlite().await
        )
    }

    /// Lists trigger schedules with pagination.
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_postgres(limit, offset).await,
            self.list_sqlite(limit, offset).await
        )
    }

    /// Updates the last poll time for a trigger schedule.
    pub async fn update_last_poll(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_last_poll_postgres(id, last_poll_at).await,
            self.update_last_poll_sqlite(id, last_poll_at).await
        )
    }

    /// Enables a trigger schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.enable_postgres(id).await,
            self.enable_sqlite(id).await
        )
    }

    /// Disables a trigger schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.disable_postgres(id).await,
            self.disable_sqlite(id).await
        )
    }

    /// Deletes a trigger schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_postgres(id).await,
            self.delete_sqlite(id).await
        )
    }

    /// Creates or updates a trigger schedule by name.
    pub async fn upsert(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_postgres(new_schedule).await,
            self.upsert_sqlite(new_schedule).await
        )
    }
}
