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

//! Backend-divergent CRUD operations for unified schedules (CLOACI-I-0135).
//!
//! Every other schedule DAL method is backend-agnostic diesel and lives inline
//! in the public methods (`mod.rs`) via `interact_on_backend!`. `claim_and_update_cron`
//! is the one method whose bodies genuinely diverge by backend: the Postgres arm
//! issues `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` to make the claim
//! atomic under concurrent schedulers, which has no SQLite equivalent (SQLite's
//! single-writer model already serializes the update). It therefore stays an
//! explicit `*_postgres`/`*_sqlite` twin pair.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::ScheduleDAL;
use crate::database::schema::unified::schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;

impl<'a> ScheduleDAL<'a> {
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
}
