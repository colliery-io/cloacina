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

//! CRUD operations for cron schedules.

use diesel::prelude::*;

use super::CronScheduleDAL;
use crate::dal::unified::models::{NewUnifiedCronSchedule, UnifiedCronSchedule};
use crate::database::schema::unified::cron_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_schedule::{CronSchedule, NewCronSchedule};

impl<'a> CronScheduleDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronSchedule {
            id,
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: UniversalBool::from(new_schedule.enabled.map(|b| b.into()).unwrap_or(true)),
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronSchedule {
            id,
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: UniversalBool::from(new_schedule.enabled.map(|b| b.into()).unwrap_or(true)),
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn delete_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| diesel::delete(cron_schedules::table.find(id)).execute(conn))
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

        conn.interact(move |conn| diesel::delete(cron_schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
