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

//! CRUD operations for trigger schedules.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::TriggerScheduleDAL;
use crate::dal::unified::models::{NewUnifiedTriggerSchedule, UnifiedTriggerSchedule};
use crate::database::schema::unified::trigger_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::trigger_schedule::{NewTriggerSchedule, TriggerSchedule};

impl<'a> TriggerScheduleDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = new_schedule.id.unwrap_or_else(UniversalUuid::new_v4);
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTriggerSchedule {
            id,
            trigger_name: new_schedule.trigger_name,
            workflow_name: new_schedule.workflow_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule
                .allow_concurrent
                .unwrap_or_else(|| UniversalBool::new(false)),
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::new(true)),
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(trigger_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTriggerSchedule = conn
            .interact(move |conn| trigger_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = new_schedule.id.unwrap_or_else(UniversalUuid::new_v4);
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTriggerSchedule {
            id,
            trigger_name: new_schedule.trigger_name,
            workflow_name: new_schedule.workflow_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule
                .allow_concurrent
                .unwrap_or_else(|| UniversalBool::new(false)),
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::new(true)),
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(trigger_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTriggerSchedule = conn
            .interact(move |conn| trigger_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<TriggerSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTriggerSchedule = conn
            .interact(move |conn| trigger_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<TriggerSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTriggerSchedule = conn
            .interact(move |conn| trigger_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_name_postgres(
        &self,
        name: &str,
    ) -> Result<Option<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let name = name.to_string();
        let result: Option<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .filter(trigger_schedules::trigger_name.eq(&name))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_name_sqlite(
        &self,
        name: &str,
    ) -> Result<Option<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let name = name.to_string();
        let result: Option<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .filter(trigger_schedules::trigger_name.eq(&name))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_enabled_postgres(
        &self,
    ) -> Result<Vec<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .filter(trigger_schedules::enabled.eq(UniversalBool::new(true)))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_enabled_sqlite(&self) -> Result<Vec<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .filter(trigger_schedules::enabled.eq(UniversalBool::new(true)))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn list_postgres(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .order(trigger_schedules::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn list_sqlite(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTriggerSchedule> = conn
            .interact(move |conn| {
                trigger_schedules::table
                    .order(trigger_schedules::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_last_poll_postgres(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(last_poll_at);
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::last_poll_at.eq(Some(timestamp)),
                    trigger_schedules::updated_at.eq(timestamp),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_last_poll_sqlite(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(last_poll_at);
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::last_poll_at.eq(Some(timestamp)),
                    trigger_schedules::updated_at.eq(timestamp),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn enable_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::enabled.eq(UniversalBool::new(true)),
                    trigger_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn enable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::enabled.eq(UniversalBool::new(true)),
                    trigger_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn disable_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::enabled.eq(UniversalBool::new(false)),
                    trigger_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn disable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(trigger_schedules::table.find(id))
                .set((
                    trigger_schedules::enabled.eq(UniversalBool::new(false)),
                    trigger_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn delete_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| diesel::delete(trigger_schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn delete_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| diesel::delete(trigger_schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn upsert_postgres(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        // Check if exists by name
        if let Some(existing) = self
            .get_by_name_postgres(&new_schedule.trigger_name)
            .await?
        {
            // Update existing
            let conn = self
                .dal
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

            let now = UniversalTimestamp::now();
            let id = existing.id;
            let workflow_name = new_schedule.workflow_name.clone();
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule
                .allow_concurrent
                .unwrap_or_else(|| UniversalBool::new(false));
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::new(true));

            conn.interact(move |conn| {
                diesel::update(trigger_schedules::table.find(id))
                    .set((
                        trigger_schedules::workflow_name.eq(workflow_name),
                        trigger_schedules::poll_interval_ms.eq(poll_interval_ms),
                        trigger_schedules::allow_concurrent.eq(allow_concurrent),
                        trigger_schedules::enabled.eq(enabled),
                        trigger_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            self.get_by_id_postgres(existing.id).await
        } else {
            // Create new
            self.create_postgres(new_schedule).await
        }
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn upsert_sqlite(
        &self,
        new_schedule: NewTriggerSchedule,
    ) -> Result<TriggerSchedule, ValidationError> {
        // Check if exists by name
        if let Some(existing) = self.get_by_name_sqlite(&new_schedule.trigger_name).await? {
            // Update existing
            let conn = self
                .dal
                .database
                .get_sqlite_connection()
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

            let now = UniversalTimestamp::now();
            let id = existing.id;
            let workflow_name = new_schedule.workflow_name.clone();
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule
                .allow_concurrent
                .unwrap_or_else(|| UniversalBool::new(false));
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::new(true));

            conn.interact(move |conn| {
                diesel::update(trigger_schedules::table.find(id))
                    .set((
                        trigger_schedules::workflow_name.eq(workflow_name),
                        trigger_schedules::poll_interval_ms.eq(poll_interval_ms),
                        trigger_schedules::allow_concurrent.eq(allow_concurrent),
                        trigger_schedules::enabled.eq(enabled),
                        trigger_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            self.get_by_id_sqlite(existing.id).await
        } else {
            // Create new
            self.create_sqlite(new_schedule).await
        }
    }
}
