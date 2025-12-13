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

//! State transition operations for cron schedules.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::CronScheduleDAL;
use crate::database::schema::unified::cron_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;

impl<'a> CronScheduleDAL<'a> {
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
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
        let enabled_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_true),
                    cron_schedules::updated_at.eq(now),
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_true),
                    cron_schedules::updated_at.eq(now),
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_false),
                    cron_schedules::updated_at.eq(now),
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_false),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_next_run_postgres(
        &self,
        id: UniversalUuid,
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_next_run_sqlite(
        &self,
        id: UniversalUuid,
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
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn claim_and_update_postgres(
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

                let updated_rows = diesel::update(cron_schedules::table.find(id))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                Ok::<_, diesel::result::Error>(updated_rows)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn claim_and_update_sqlite(
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
                diesel::update(cron_schedules::table.find(id))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_expression_and_timezone_postgres(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
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
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(id));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_expression_and_timezone_sqlite(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
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
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(id));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
