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

//! CRUD operations for unified schedules.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::ScheduleDAL;
use crate::dal::unified::models::{NewUnifiedSchedule, UnifiedSchedule};
use crate::database::schema::unified::schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::schedule::{NewSchedule, Schedule};

// =============================================================================
// PostgreSQL implementations
// =============================================================================

impl<'a> ScheduleDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedSchedule {
            id,
            schedule_type: new_schedule.schedule_type,
            workflow_name: new_schedule.workflow_name,
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true)),
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone,
            catchup_policy: new_schedule.catchup_policy,
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            trigger_name: new_schedule.trigger_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule.allow_concurrent,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
            params: new_schedule.params,
            instance_name: new_schedule.instance_name,
        };

        conn.interact(move |conn| {
            diesel::insert_into(schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedSchedule = conn
            .interact(move |conn| schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn find_by_instance_name_postgres(
        &self,
        workflow_name: &str,
        instance_name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let wf = workflow_name.to_string();
        let inst = instance_name.to_string();
        let result: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::workflow_name.eq(wf))
                    .filter(schedules::instance_name.eq(inst))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(result.map(Into::into))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn find_by_instance_name_sqlite(
        &self,
        workflow_name: &str,
        instance_name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let wf = workflow_name.to_string();
        let inst = instance_name.to_string();
        let result: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::workflow_name.eq(wf))
                    .filter(schedules::instance_name.eq(inst))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(result.map(Into::into))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedSchedule {
            id,
            schedule_type: new_schedule.schedule_type,
            workflow_name: new_schedule.workflow_name,
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true)),
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone,
            catchup_policy: new_schedule.catchup_policy,
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            trigger_name: new_schedule.trigger_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule.allow_concurrent,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
            params: new_schedule.params,
            instance_name: new_schedule.instance_name,
        };

        conn.interact(move |conn| {
            diesel::insert_into(schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedSchedule = conn
            .interact(move |conn| schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedSchedule = conn
            .interact(move |conn| schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedSchedule = conn
            .interact(move |conn| schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn list_postgres(
        &self,
        schedule_type: Option<String>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                let mut query = schedules::table.into_boxed();

                if let Some(ref stype) = schedule_type {
                    query = query.filter(schedules::schedule_type.eq(stype));
                }

                if enabled_only {
                    query = query.filter(schedules::enabled.eq(enabled_true));
                }

                query
                    .order(schedules::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn list_sqlite(
        &self,
        schedule_type: Option<String>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                let mut query = schedules::table.into_boxed();

                if let Some(ref stype) = schedule_type {
                    query = query.filter(schedules::schedule_type.eq(stype));
                }

                if enabled_only {
                    query = query.filter(schedules::enabled.eq(enabled_true));
                }

                query
                    .order(schedules::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
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
        let enabled_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_true),
                    schedules::updated_at.eq(now),
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
        let enabled_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_true),
                    schedules::updated_at.eq(now),
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
        let enabled_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_false),
                    schedules::updated_at.eq(now),
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
        let enabled_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_false),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    // CLOACI-T-0749: pause / resume. Distinct from enable/disable — `paused`
    // is transient operator state that the scheduler honors (a paused schedule
    // is never fetched for firing), while `enabled` is the deliberate on/off.

    #[cfg(feature = "postgres")]
    pub(super) async fn pause_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let paused_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_true),
                    schedules::paused_at.eq(Some(now)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn pause_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let paused_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_true),
                    schedules::paused_at.eq(Some(now)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn resume_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let paused_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_false),
                    schedules::paused_at.eq(None::<UniversalTimestamp>),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn resume_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let paused_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_false),
                    schedules::paused_at.eq(None::<UniversalTimestamp>),
                    schedules::updated_at.eq(now),
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

        conn.interact(move |conn| diesel::delete(schedules::table.find(id)).execute(conn))
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

        conn.interact(move |conn| diesel::delete(schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_due_cron_schedules_postgres(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .filter(schedules::next_run_at.le(Some(now_ts)))
                    .order(schedules::next_run_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_due_cron_schedules_sqlite(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .filter(schedules::next_run_at.le(Some(now_ts)))
                    .order(schedules::next_run_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn next_cron_due_time_postgres(
        &self,
    ) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let earliest: Option<UniversalTimestamp> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .filter(schedules::next_run_at.is_not_null())
                    .order(schedules::next_run_at.asc())
                    .select(schedules::next_run_at)
                    .first::<Option<UniversalTimestamp>>(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??
            .flatten();

        Ok(earliest.map(DateTime::<Utc>::from))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn next_cron_due_time_sqlite(
        &self,
    ) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let earliest: Option<UniversalTimestamp> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .filter(schedules::next_run_at.is_not_null())
                    .order(schedules::next_run_at.asc())
                    .select(schedules::next_run_at)
                    .first::<Option<UniversalTimestamp>>(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??
            .flatten();

        Ok(earliest.map(DateTime::<Utc>::from))
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn claim_and_update_cron_postgres(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let current_ts = UniversalTimestamp::from(current_time);
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        let updated_rows = conn
            .interact(move |conn| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                let updated_rows = diesel::update(schedules::table.find(id))
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::next_run_at.le(Some(current_ts)))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .set((
                        schedules::last_run_at.eq(Some(last_run_ts)),
                        schedules::next_run_at.eq(Some(next_run_ts)),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                Ok::<_, diesel::result::Error>(updated_rows)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn claim_and_update_cron_sqlite(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let current_ts = UniversalTimestamp::from(current_time);
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        let updated_rows = conn
            .interact(move |conn| {
                diesel::update(schedules::table.find(id))
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::next_run_at.le(Some(current_ts)))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .set((
                        schedules::last_run_at.eq(Some(last_run_ts)),
                        schedules::next_run_at.eq(Some(next_run_ts)),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_schedule_times_postgres(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_run_at.eq(Some(last_run_ts)),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_schedule_times_sqlite(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_run_at.eq(Some(last_run_ts)),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_enabled_triggers_postgres(
        &self,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("trigger"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .order(schedules::created_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_enabled_triggers_sqlite(
        &self,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::schedule_type.eq("trigger"))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .order(schedules::created_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
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

        let last_poll_ts = UniversalTimestamp::from(last_poll_at);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_poll_at.eq(Some(last_poll_ts)),
                    schedules::updated_at.eq(now),
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

        let last_poll_ts = UniversalTimestamp::from(last_poll_at);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_poll_at.eq(Some(last_poll_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn upsert_trigger_postgres(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name =
            new_schedule
                .trigger_name
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "trigger_name is required for upsert_trigger".to_string(),
                })?;

        let trigger_name_check = trigger_name.clone();

        // Check if a schedule with this trigger_name already exists
        let existing: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::trigger_name.eq(trigger_name_check))
                    .filter(schedules::schedule_type.eq("trigger"))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        if let Some(existing) = existing {
            // Update existing schedule
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let workflow_name = new_schedule.workflow_name;
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule.allow_concurrent;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            conn.interact(move |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::workflow_name.eq(workflow_name),
                        schedules::poll_interval_ms.eq(poll_interval_ms),
                        schedules::allow_concurrent.eq(allow_concurrent),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(existing_id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            Ok(result.into())
        } else {
            // Insert new schedule
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();

            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };

            conn.interact(move |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            Ok(result.into())
        }
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn upsert_trigger_sqlite(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name =
            new_schedule
                .trigger_name
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "trigger_name is required for upsert_trigger".to_string(),
                })?;

        let trigger_name_check = trigger_name.clone();

        // Check if a schedule with this trigger_name already exists
        let existing: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::trigger_name.eq(trigger_name_check))
                    .filter(schedules::schedule_type.eq("trigger"))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        if let Some(existing) = existing {
            // Update existing schedule
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let workflow_name = new_schedule.workflow_name;
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule.allow_concurrent;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            conn.interact(move |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::workflow_name.eq(workflow_name),
                        schedules::poll_interval_ms.eq(poll_interval_ms),
                        schedules::allow_concurrent.eq(allow_concurrent),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(existing_id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            Ok(result.into())
        } else {
            // Insert new schedule
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();

            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };

            conn.interact(move |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            Ok(result.into())
        }
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn upsert_cron_postgres(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // Identity of a cron schedule: (workflow_name, cron_expression,
        // timezone). Re-registering the same packaged cron on a reconcile
        // re-load updates the existing row instead of inserting a duplicate.
        let cron_expression =
            new_schedule
                .cron_expression
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "cron_expression is required for upsert_cron".to_string(),
                })?;
        let workflow_name = new_schedule.workflow_name.clone();
        let timezone = new_schedule.timezone.clone();

        let (wf, cron, tz) = (
            workflow_name.clone(),
            cron_expression.clone(),
            timezone.clone(),
        );
        let existing: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                let mut q = schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::workflow_name.eq(wf))
                    .filter(schedules::cron_expression.eq(cron))
                    .into_boxed();
                q = match tz {
                    Some(t) => q.filter(schedules::timezone.eq(t)),
                    None => q.filter(schedules::timezone.is_null()),
                };
                q.first(conn).optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        if let Some(existing) = existing {
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let next_run_at = new_schedule.next_run_at;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            conn.interact(move |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::next_run_at.eq(next_run_at),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(existing_id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
            Ok(result.into())
        } else {
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();
            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };
            conn.interact(move |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
            Ok(result.into())
        }
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn upsert_cron_sqlite(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cron_expression =
            new_schedule
                .cron_expression
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "cron_expression is required for upsert_cron".to_string(),
                })?;
        let workflow_name = new_schedule.workflow_name.clone();
        let timezone = new_schedule.timezone.clone();

        let (wf, cron, tz) = (
            workflow_name.clone(),
            cron_expression.clone(),
            timezone.clone(),
        );
        let existing: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                let mut q = schedules::table
                    .filter(schedules::schedule_type.eq("cron"))
                    .filter(schedules::workflow_name.eq(wf))
                    .filter(schedules::cron_expression.eq(cron))
                    .into_boxed();
                q = match tz {
                    Some(t) => q.filter(schedules::timezone.eq(t)),
                    None => q.filter(schedules::timezone.is_null()),
                };
                q.first(conn).optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        if let Some(existing) = existing {
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let next_run_at = new_schedule.next_run_at;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            conn.interact(move |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::next_run_at.eq(next_run_at),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(existing_id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
            Ok(result.into())
        } else {
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();
            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };
            conn.interact(move |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

            let result: UnifiedSchedule = conn
                .interact(move |conn| schedules::table.find(id).first(conn))
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
            Ok(result.into())
        }
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_trigger_name_postgres(
        &self,
        name: String,
    ) -> Result<Option<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::trigger_name.eq(name))
                    .filter(schedules::schedule_type.eq("trigger"))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(|r| r.into()))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_trigger_name_sqlite(
        &self,
        name: String,
    ) -> Result<Option<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::trigger_name.eq(name))
                    .filter(schedules::schedule_type.eq("trigger"))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(|r| r.into()))
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn find_by_workflow_postgres(
        &self,
        workflow_name: String,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::workflow_name.eq(workflow_name))
                    .order(schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn find_by_workflow_sqlite(
        &self,
        workflow_name: String,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedSchedule> = conn
            .interact(move |conn| {
                schedules::table
                    .filter(schedules::workflow_name.eq(workflow_name))
                    .order(schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_cron_expression_and_timezone_postgres(
        &self,
        id: UniversalUuid,
        cron_expression: Option<String>,
        timezone: Option<String>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::cron_expression.eq(cron_expression),
                    schedules::timezone.eq(timezone),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_cron_expression_and_timezone_sqlite(
        &self,
        id: UniversalUuid,
        cron_expression: Option<String>,
        timezone: Option<String>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::cron_expression.eq(cron_expression),
                    schedules::timezone.eq(timezone),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
